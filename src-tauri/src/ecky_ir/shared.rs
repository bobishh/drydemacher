use csgrs::mesh::Mesh;
use csgrs::sketch::Sketch;

use crate::contracts::{AppError, AuthoringError, AuthoringReason};

pub(super) type IrMesh = Mesh<()>;
pub(super) type IrSketch = Sketch<()>;
pub(super) type LoopPoints = Vec<[f64; 2]>;

pub(super) fn unsupported(details: impl Into<String>) -> AppError {
    AppError::with_details(
        crate::contracts::AppErrorCode::Validation,
        "Unsupported on current geometry backend. Switch backend and rerender.",
        details.into(),
    )
}

pub(super) fn validation(message: impl Into<String>) -> AppError {
    AppError::validation(message.into())
}

pub(super) fn surface_dependency_error(err: AppError) -> AuthoringError {
    copy_boundary_fields(
        AuthoringError::surface(AuthoringReason::ParseSyntax, err.to_string()),
        err,
    )
}

pub(super) fn lowering_dependency_error(backend: &str, err: AppError) -> AuthoringError {
    let authoring =
        if err.message == "Unsupported on current geometry backend. Switch backend and rerender." {
            AuthoringError::unsupported_backend(
                backend,
                err.operation
                    .clone()
                    .unwrap_or_else(|| "lowering".to_string()),
                err.to_string(),
            )
        } else {
            AuthoringError::core_ir(AuthoringReason::Type, err.to_string())
        };
    copy_boundary_fields(authoring, err)
}

fn copy_boundary_fields(mut authoring: AuthoringError, err: AppError) -> AuthoringError {
    if authoring.op.is_none() {
        authoring.op = err.operation;
    }
    if let Some(start_line) = err.start_line {
        authoring.span = Some((start_line, err.end_line.unwrap_or(start_line)));
    }
    if let Some(fix) = err.fix {
        authoring.fix = Some(fix);
    }
    authoring
}
