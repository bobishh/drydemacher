use ecky_render::SourceCompiler;

use crate::contracts::{AppError, AppResult};

/// Native Ecky source adapter. Steel ownership stays outside render consumers.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeSourceCompiler;

impl SourceCompiler for NativeSourceCompiler {
    type Error = AppError;

    fn compile(&self, source: &str) -> AppResult<ecky_render::core_ir::CoreProgram> {
        match crate::ecky_scheme::try_compile_to_core_program(source) {
            Some(result) => result,
            None => Err(AppError::parse(
                "Source is not compileable `.ecky` model syntax.",
            )),
        }
    }
}
