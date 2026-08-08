use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use ecky_fem::{
    FaerSparseCholeskySolver, FemLinearSolveResult, FemSparseMatrix, FemValidationError,
    LinearSolver,
};
use serde::{Deserialize, Serialize};

const WORKER_ARG: &str = "--ecky-fem-solver-worker";
const WORKER_PROTOCOL_VERSION: u32 = 1;
static WORKER_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SolverWorkerRequest {
    protocol_version: u32,
    matrix: FemSparseMatrix,
    rhs: Vec<f64>,
    relative_tolerance: f64,
    maximum_dimension: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status", deny_unknown_fields)]
enum SolverWorkerResponse {
    Success { result: FemLinearSolveResult },
    Failure { error: FemValidationError },
}

pub(crate) struct KillableFaerWorkerSolver<'a> {
    executable: PathBuf,
    scratch_dir: &'a Path,
    cancelled: &'a AtomicBool,
    maximum_runtime: Duration,
}

impl<'a> KillableFaerWorkerSolver<'a> {
    pub(crate) fn for_current_app(
        scratch_dir: &'a Path,
        cancelled: &'a AtomicBool,
        maximum_runtime_ms: u64,
    ) -> Option<Self> {
        solver_worker_executable().map(|executable| Self {
            executable,
            scratch_dir,
            cancelled,
            maximum_runtime: Duration::from_millis(maximum_runtime_ms),
        })
    }

    #[cfg(test)]
    fn with_executable(
        executable: PathBuf,
        scratch_dir: &'a Path,
        cancelled: &'a AtomicBool,
        maximum_runtime: Duration,
    ) -> Self {
        Self {
            executable,
            scratch_dir,
            cancelled,
            maximum_runtime,
        }
    }

    fn solve_in_worker(
        &self,
        matrix: &FemSparseMatrix,
        rhs: &[f64],
        relative_tolerance: f64,
        maximum_dimension: usize,
    ) -> Result<FemLinearSolveResult, FemValidationError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
        fs::create_dir_all(self.scratch_dir).map_err(io_error("solver scratch"))?;
        let nonce = WORKER_NONCE.fetch_add(1, Ordering::Relaxed);
        let stem = format!("faer-{}-{nonce}", std::process::id());
        let request_path = self.scratch_dir.join(format!("{stem}.request.json"));
        let response_path = self.scratch_dir.join(format!("{stem}.response.json"));
        let response_temp_path = response_temp_path(&response_path);
        let request = SolverWorkerRequest {
            protocol_version: WORKER_PROTOCOL_VERSION,
            matrix: matrix.clone(),
            rhs: rhs.to_vec(),
            relative_tolerance,
            maximum_dimension,
        };
        let bytes = serde_json::to_vec(&request).map_err(|error| FemValidationError {
            field: "solverWorker.request".into(),
            message: format!("serialization failed: {error}"),
        })?;
        fs::write(&request_path, bytes).map_err(io_error("solver request"))?;

        let mut child = match Command::new(&self.executable)
            .arg(WORKER_ARG)
            .arg(&request_path)
            .arg(&response_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                cleanup_worker_files(&request_path, &response_path, &response_temp_path);
                return Err(FemValidationError {
                    field: "solverWorker.executable".into(),
                    message: format!(
                        "could not start killable Faer worker '{}': {error}",
                        self.executable.display()
                    ),
                });
            }
        };
        let started = Instant::now();
        let status = loop {
            if self.cancelled.load(Ordering::Acquire) {
                let _ = child.kill();
                let _ = child.wait();
                cleanup_worker_files(&request_path, &response_path, &response_temp_path);
                return Err(cancelled_error());
            }
            if started.elapsed() > self.maximum_runtime {
                let _ = child.kill();
                let _ = child.wait();
                cleanup_worker_files(&request_path, &response_path, &response_temp_path);
                return Err(FemValidationError {
                    field: "solverWorker.maximumRuntimeMs".into(),
                    message: format!(
                        "killable Faer worker exceeded {} ms and was terminated",
                        self.maximum_runtime.as_millis()
                    ),
                });
            }
            match child.try_wait().map_err(io_error("solver worker wait"))? {
                Some(status) => break status,
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        };
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        if !status.success() {
            cleanup_worker_files(&request_path, &response_path, &response_temp_path);
            return Err(FemValidationError {
                field: "solverWorker.process".into(),
                message: format!(
                    "killable Faer worker exited with {status}: {}",
                    stderr.trim()
                ),
            });
        }
        let response = fs::read(&response_path)
            .map_err(io_error("solver response"))
            .and_then(|response_bytes| {
                serde_json::from_slice(&response_bytes).map_err(|error| FemValidationError {
                    field: "solverWorker.response".into(),
                    message: format!("invalid response: {error}"),
                })
            });
        cleanup_worker_files(&request_path, &response_path, &response_temp_path);
        match response? {
            SolverWorkerResponse::Success { result } => Ok(result),
            SolverWorkerResponse::Failure { error } => Err(error),
        }
    }
}

impl LinearSolver for KillableFaerWorkerSolver<'_> {
    fn solve(
        &self,
        matrix: &FemSparseMatrix,
        rhs: &[f64],
        relative_tolerance: f64,
        maximum_dimension: usize,
    ) -> Result<FemLinearSolveResult, FemValidationError> {
        self.solve_in_worker(matrix, rhs, relative_tolerance, maximum_dimension)
    }
}

pub fn maybe_run_from_process_args() -> Option<i32> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.get(1).and_then(|value| value.to_str()) != Some(WORKER_ARG) {
        return None;
    }
    let Some(request_path) = args.get(2).map(PathBuf::from) else {
        eprintln!("FEM solver worker requires request and response paths.");
        return Some(2);
    };
    let Some(response_path) = args.get(3).map(PathBuf::from) else {
        eprintln!("FEM solver worker requires request and response paths.");
        return Some(2);
    };
    match run_worker_files(&request_path, &response_path) {
        Ok(()) => Some(0),
        Err(error) => {
            eprintln!("{error}");
            Some(3)
        }
    }
}

fn run_worker_files(request_path: &Path, response_path: &Path) -> Result<(), FemValidationError> {
    let request: SolverWorkerRequest =
        serde_json::from_slice(&fs::read(request_path).map_err(io_error("solver request"))?)
            .map_err(|error| FemValidationError {
                field: "solverWorker.request".into(),
                message: format!("invalid request: {error}"),
            })?;
    if request.protocol_version != WORKER_PROTOCOL_VERSION {
        return Err(FemValidationError {
            field: "solverWorker.protocolVersion".into(),
            message: format!(
                "unsupported protocol {}, expected {WORKER_PROTOCOL_VERSION}",
                request.protocol_version
            ),
        });
    }
    let response = match FaerSparseCholeskySolver.solve(
        &request.matrix,
        &request.rhs,
        request.relative_tolerance,
        request.maximum_dimension,
    ) {
        Ok(result) => SolverWorkerResponse::Success { result },
        Err(error) => SolverWorkerResponse::Failure { error },
    };
    let bytes = serde_json::to_vec(&response).map_err(|error| FemValidationError {
        field: "solverWorker.response".into(),
        message: format!("serialization failed: {error}"),
    })?;
    let temporary = response_temp_path(response_path);
    fs::write(&temporary, bytes).map_err(io_error("solver response"))?;
    fs::rename(&temporary, response_path).map_err(io_error("solver response publication"))?;
    Ok(())
}

fn solver_worker_executable() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("ECKY_FEM_SOLVER_WORKER_EXECUTABLE") {
        return Some(PathBuf::from(path));
    }
    let current = std::env::current_exe().ok()?;
    if current
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| parent.to_str() == Some("deps"))
    {
        return None;
    }
    Some(current)
}

fn response_temp_path(response_path: &Path) -> PathBuf {
    response_path.with_extension(format!("json.tmp-{}", std::process::id()))
}

fn cleanup_worker_files(request: &Path, response: &Path, response_temp: &Path) {
    for path in [request, response, response_temp] {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
}

fn cancelled_error() -> FemValidationError {
    FemValidationError {
        field: "cancelled".into(),
        message: "FEM factorization was cancelled; killable Faer worker terminated.".into(),
    }
}

fn io_error(label: &'static str) -> impl FnOnce(std::io::Error) -> FemValidationError {
    move |error| FemValidationError {
        field: "solverWorker.io".into(),
        message: format!("{label} failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ecky-fem-solver-worker-{label}-{}-{}",
            std::process::id(),
            WORKER_NONCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn worker_file_protocol_returns_a_finite_faer_solution_atomically() {
        let root = root("protocol");
        fs::create_dir_all(&root).unwrap();
        let request_path = root.join("request.json");
        let response_path = root.join("response.json");
        let request = SolverWorkerRequest {
            protocol_version: WORKER_PROTOCOL_VERSION,
            matrix: FemSparseMatrix::from_dense(vec![vec![4.0, 1.0], vec![1.0, 3.0]]).unwrap(),
            rhs: vec![1.0, 2.0],
            relative_tolerance: 1.0e-12,
            maximum_dimension: 8,
        };
        fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();

        run_worker_files(&request_path, &response_path).unwrap();

        assert!(response_path.is_file());
        assert!(!response_temp_path(&response_path).exists());
        let response: SolverWorkerResponse =
            serde_json::from_slice(&fs::read(&response_path).unwrap()).unwrap();
        assert!(
            matches!(response, SolverWorkerResponse::Success { result } if result.solution.iter().all(|value| value.is_finite()))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_terminates_uninterruptible_factorization_process_without_partial_response() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::Arc;

        let root = root("cancel");
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("blocking-worker.sh");
        fs::write(&executable, "#!/bin/sh\nexec sleep 30\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let trigger = cancelled.clone();
        let cancellation = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(40));
            trigger.store(true, Ordering::Release);
        });
        let solver = KillableFaerWorkerSolver::with_executable(
            executable,
            &root,
            cancelled.as_ref(),
            Duration::from_secs(5),
        );
        let matrix = FemSparseMatrix::from_dense(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).unwrap();
        let started = Instant::now();

        let error = solver
            .solve(&matrix, &[1.0, 1.0], 1.0e-12, 8)
            .expect_err("factorization process must be killable");

        cancellation.join().unwrap();
        assert_eq!(error.field, "cancelled");
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(
            fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .path()
                    .extension()
                    .is_some_and(|value| value == "json"))
                .count(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }
}
