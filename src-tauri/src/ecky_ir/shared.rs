use csgrs::mesh::Mesh;
use csgrs::sketch::Sketch;

use crate::contracts::AppError;

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
