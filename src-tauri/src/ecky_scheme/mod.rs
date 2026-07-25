use crate::contracts::{AppError, AppErrorCode, AppResult};
use crate::ecky_core_ir::{CompilerError, CompilerErrorKind, CoreProgram, CoreResult};

pub use ecky_render::scheme::{
    bootstrap, bootstrap_shape, cad, core, params, BootstrapShape, ModuleSpec, APP_MODULES,
    BOOTSTRAP_SHAPE,
};

pub mod compiler {
    pub use super::{
        compile_to_core_program, compile_to_legacy_source, try_compile_to_core_program,
        try_compile_to_legacy_source,
    };
    pub use ecky_render::scheme::compiler::{
        collect_free_variables, expr_head_name, expr_identifier, expr_list_items,
    };
}

pub fn compile_to_core_program(source: &str) -> CoreResult<CoreProgram> {
    ecky_render::scheme::compile_to_core_program(source)
}

pub fn compile_to_legacy_source(source: &str) -> AppResult<String> {
    ecky_render::scheme::compile_to_legacy_source(source).map_err(core_err_to_app)
}

pub fn try_compile_to_core_program(source: &str) -> Option<AppResult<CoreProgram>> {
    ecky_render::scheme::try_compile_to_core_program(source)
        .map(|result| result.map_err(core_err_to_app))
}

pub fn try_compile_to_legacy_source(source: &str) -> Option<AppResult<String>> {
    ecky_render::scheme::try_compile_to_legacy_source(source)
        .map(|result| result.map_err(core_err_to_app))
}

fn core_err_to_app(err: CompilerError) -> AppError {
    match err.kind {
        CompilerErrorKind::Parse => AppError::parse(err.to_string()),
        CompilerErrorKind::Resolve | CompilerErrorKind::TypeMismatch => {
            AppError::validation(err.to_string())
        }
        CompilerErrorKind::UnsupportedFeature => {
            AppError::new(AppErrorCode::Validation, err.to_string())
        }
        CompilerErrorKind::Backend | CompilerErrorKind::Internal => {
            AppError::internal(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_has_app_modules_only() {
        let shape = bootstrap_shape();
        assert!(shape.sandboxed);
        assert_eq!(shape.modules.len(), 3);
        assert_eq!(shape.modules[0].scheme_name, "ecky/core");
        assert_eq!(shape.modules[1].scheme_name, "ecky/cad");
        assert_eq!(shape.modules[2].scheme_name, "ecky/params");
    }

    #[test]
    fn shape_blocks_unsafe_ops() {
        let ops = bootstrap::BLOCKED_UNSAFE_OPS;
        assert!(ops.contains(&"create-directory!"));
        assert!(ops.contains(&"delete-directory!"));
        assert!(ops.contains(&"delete-file!"));
        assert!(ops.contains(&"open-input-file"));
        assert!(ops.contains(&"open-output-file"));
    }

    #[test]
    fn compiles_old_style_model_source_via_steel() {
        let compiled = compile_to_legacy_source(
            r#"
            (model
              (params
                (number radius 10 :label "Radius")
                (toggle printed false))
              (part body
                (translate 0 0 5
                  (extrude (circle radius) 20))))
            "#,
        )
        .expect("compile");

        assert!(compiled.contains("(model"));
        assert!(compiled.contains("(number radius 10"));
        assert!(compiled.contains("(translate 0 0 5"));
        assert!(compiled.contains("(circle radius)"));
    }

    #[test]
    fn compiles_scheme_helpers_into_model_source() {
        let compiled = compile_to_legacy_source(
            r#"
            (define (cup-body radius height)
              (extrude (circle radius) height))

            (model
              (part body (cup-body 12 30)))
            "#,
        )
        .expect("compile");

        assert!(compiled.contains("(part body"));
        assert!(compiled.contains("(extrude (circle "), "{}", compiled);
        assert!(!compiled.contains("##"), "{}", compiled);
        compile_to_core_program(&compiled).expect("emitted source reparses");
    }

    #[test]
    fn scheme_source_flows_through_ecky_ir_lowerer() {
        let code = crate::ecky_ir::lower_to_build123d(
            r#"
            (define (cup-body radius height)
              (extrude (circle radius) height))

            (model
              (part body (cup-body 12 30)))
            "#,
        )
        .expect("lower");

        assert!(code.contains("Circle("), "{}", code);
        assert!(code.contains("extrude"), "{}", code);
    }
}
