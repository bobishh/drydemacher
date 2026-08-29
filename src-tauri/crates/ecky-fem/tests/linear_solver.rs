#[cfg(target_os = "macos")]
use ecky_fem::AccelerateSparseCholeskySolver;
use ecky_fem::{
    FaerSparseCholeskySolver, FemSparseEntry, FemSparseMatrix, LinearSolver,
    ReferenceCholeskySolver,
};

#[test]
fn reference_cholesky_solves_spd_system_and_reports_residual() {
    let matrix =
        FemSparseMatrix::from_dense(vec![vec![4.0, 1.0], vec![1.0, 3.0]]).expect("SPD matrix");
    let result = ReferenceCholeskySolver
        .solve(&matrix, &[1.0, 2.0], 1.0e-12, 8)
        .expect("SPD solve");

    assert!((result.solution[0] - 1.0 / 11.0).abs() <= 1.0e-12);
    assert!((result.solution[1] - 7.0 / 11.0).abs() <= 1.0e-12);
    assert!(result.relative_residual <= 1.0e-12);
}

#[test]
fn reference_cholesky_rejects_singular_nonsymmetric_and_over_budget_systems() {
    let singular = FemSparseMatrix::from_dense(vec![vec![1.0, 1.0], vec![1.0, 1.0]])
        .expect("singular matrix contract");
    assert!(ReferenceCholeskySolver
        .solve(&singular, &[1.0, 1.0], 1.0e-12, 8)
        .unwrap_err()
        .message
        .contains("positive definite"));

    let nonsymmetric = FemSparseMatrix::from_dense(vec![vec![2.0, 1.0], vec![0.0, 2.0]])
        .expect("nonsymmetric matrix contract");
    assert!(ReferenceCholeskySolver
        .solve(&nonsymmetric, &[1.0, 1.0], 1.0e-12, 8)
        .unwrap_err()
        .message
        .contains("symmetric"));

    let spd =
        FemSparseMatrix::from_dense(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).expect("identity matrix");
    assert!(ReferenceCholeskySolver
        .solve(&spd, &[1.0, 1.0], 1.0e-12, 1)
        .unwrap_err()
        .message
        .contains("budget"));
}

#[test]
fn production_sparse_solver_matches_oracle_and_rejects_non_spd() {
    let matrix = FemSparseMatrix::from_dense(vec![
        vec![10.0, 2.0, 0.0],
        vec![2.0, 7.0, 1.0],
        vec![0.0, 1.0, 5.0],
    ])
    .expect("matrix");
    let rhs = [7.0, -8.0, 6.0];
    let oracle = ReferenceCholeskySolver
        .solve(&matrix, &rhs, 1.0e-12, 10)
        .expect("oracle solve");
    let solved = FaerSparseCholeskySolver
        .solve(&matrix, &rhs, 1.0e-12, 10)
        .expect("Faer sparse solve");
    for (actual, expected) in solved.solution.iter().zip(oracle.solution) {
        assert!((actual - expected).abs() < 1.0e-11);
    }
    assert!(solved.relative_residual <= 1.0e-12);
    assert_eq!(solved.solver_identity.backend, "faer");
    assert_eq!(solved.solver_identity.backend_version, "0.24.4");
    assert_eq!(solved.solver_identity.factorization, "sparse-llt");
    assert_eq!(solved.solver_identity.ordering, "faer-default-amd");
    assert_eq!(solved.solver_identity.parallelism, "sequential");
    assert_eq!(solved.solver_identity.relative_tolerance, 1.0e-12);

    let non_spd = FemSparseMatrix::from_dense(vec![vec![1.0, 2.0], vec![2.0, 1.0]])
        .expect("symmetric indefinite matrix");
    let error = FaerSparseCholeskySolver
        .solve(&non_spd, &[1.0, 1.0], 1.0e-12, 10)
        .expect_err("non-SPD must fail");
    assert!(error.message.contains("Faer") || error.message.contains("positive definite"));
}

#[test]
fn production_sparse_solver_rejects_non_finite_factor_inputs_and_rhs() {
    let non_finite_matrix = FemSparseMatrix {
        dimension: 1,
        entries: vec![FemSparseEntry {
            row: 0,
            col: 0,
            value: f64::INFINITY,
        }],
    };
    let error = FaerSparseCholeskySolver
        .solve(&non_finite_matrix, &[1.0], 1.0e-12, 4)
        .expect_err("non-finite factor input must fail before Faer");
    assert_eq!(error.field, "matrix.entries.value");
    assert!(error.message.contains("finite"));

    let identity = FemSparseMatrix::from_dense(vec![vec![1.0]]).expect("identity");
    let error = FaerSparseCholeskySolver
        .solve(&identity, &[f64::NAN], 1.0e-12, 4)
        .expect_err("non-finite RHS must fail before factorization");
    assert_eq!(error.field, "rhs.value");
    assert!(error.message.contains("finite"));
}

#[test]
fn production_sparse_solver_rejects_solution_above_authored_residual_tolerance() {
    let matrix = FemSparseMatrix::from_dense(vec![
        vec![0.1, 0.02, 0.003],
        vec![0.02, 0.3, 0.04],
        vec![0.003, 0.04, 0.7],
    ])
    .expect("SPD matrix");
    let error = FaerSparseCholeskySolver
        .solve(&matrix, &[0.7, -1.3, 2.1], 1.0e-30, 8)
        .expect_err("residual above an authored tolerance must fail");
    assert_eq!(error.field, "solution.relativeResidual");
    assert!(error.message.contains("exceeds tolerance"));
}

#[test]
fn sparse_solver_batch_rhs_matches_independent_solves() {
    let matrix =
        FemSparseMatrix::from_dense(vec![vec![4.0, 1.0], vec![1.0, 3.0]]).expect("SPD matrix");
    let right_hand_sides = vec![vec![1.0, 2.0], vec![-3.0, 4.0]];
    let batch = FaerSparseCholeskySolver
        .solve_many(&matrix, &right_hand_sides, 1.0e-12, 2)
        .expect("one factorization with multiple right-hand sides");
    assert_eq!(batch.len(), right_hand_sides.len());
    for (rhs, observed) in right_hand_sides.iter().zip(batch) {
        let independent = FaerSparseCholeskySolver
            .solve(&matrix, rhs, 1.0e-12, 2)
            .expect("independent solve");
        assert_eq!(observed.solution, independent.solution);
        assert_eq!(observed.relative_residual, independent.relative_residual);
    }
}

#[test]
fn production_parallel_sparse_solver_records_requested_worker_count() {
    let matrix = FemSparseMatrix::from_dense(vec![
        vec![6.0, -1.0, 0.0],
        vec![-1.0, 6.0, -1.0],
        vec![0.0, -1.0, 6.0],
    ])
    .expect("SPD matrix");
    let solved = FaerSparseCholeskySolver::with_thread_count(4)
        .solve_many(
            &matrix,
            &[vec![1.0, 0.0, 0.0], vec![0.0, 0.0, 1.0]],
            1.0e-12,
            3,
        )
        .expect("parallel sparse batch solve");

    assert_eq!(solved.len(), 2);
    assert!(solved
        .iter()
        .all(|result| result.solver_identity.thread_count == 4));
    assert!(solved
        .iter()
        .all(|result| result.solver_identity.parallelism == "rayon"));
}

#[cfg(target_os = "macos")]
#[test]
fn accelerate_sparse_solver_reuses_one_factor_for_batch_rhs() {
    let matrix = FemSparseMatrix::from_dense(vec![
        vec![10.0, 2.0, 0.0],
        vec![2.0, 7.0, 1.0],
        vec![0.0, 1.0, 5.0],
    ])
    .expect("SPD matrix");
    let solved = AccelerateSparseCholeskySolver
        .solve_many(
            &matrix,
            &[vec![7.0, -8.0, 6.0], vec![1.0, 0.0, -1.0]],
            1.0e-12,
            3,
        )
        .expect("Accelerate batch solve");

    assert_eq!(solved.len(), 2);
    assert!(solved.iter().all(|item| item.relative_residual <= 1.0e-12));
    assert!(solved.iter().all(|item| {
        item.solver_identity.backend == "accelerate-sparse"
            && item.solver_identity.parallelism == "accelerate-managed"
            && item.solver_identity.thread_count == 0
    }));
}

#[cfg(target_os = "macos")]
#[test]
fn accelerate_sparse_solver_preserves_noncanonical_coordinate_compatibility() {
    let mut matrix = FemSparseMatrix::from_dense(vec![
        vec![5.0, 1.0, 0.0],
        vec![1.0, 4.0, -1.0],
        vec![0.0, -1.0, 3.0],
    ])
    .unwrap();
    matrix.entries.reverse();
    let solved = AccelerateSparseCholeskySolver
        .solve(&matrix, &[2.0, -1.0, 3.0], 1.0e-12, 3)
        .expect("noncanonical compatibility solve");
    assert!(solved.relative_residual <= 1.0e-12);
}
