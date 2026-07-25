pub mod bootstrap;
pub mod cad;
pub mod compiler;
pub mod core;
pub mod params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleSpec {
    pub scheme_name: &'static str,
    pub rust_module: &'static str,
    pub exports: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapShape {
    pub sandboxed: bool,
    pub modules: &'static [ModuleSpec],
    pub blocked_unsafe_ops: &'static [&'static str],
}

pub const APP_MODULES: [ModuleSpec; 3] = [core::MODULE, cad::MODULE, params::MODULE];

pub const BOOTSTRAP_SHAPE: BootstrapShape = BootstrapShape {
    sandboxed: true,
    modules: &APP_MODULES,
    blocked_unsafe_ops: bootstrap::BLOCKED_UNSAFE_OPS,
};

pub fn bootstrap_shape() -> &'static BootstrapShape {
    &BOOTSTRAP_SHAPE
}

pub use compiler::{
    compile_to_core_program, compile_to_legacy_source, try_compile_to_core_program,
    try_compile_to_legacy_source,
};

/// Platform-neutral Ecky Scheme compiler.
#[derive(Debug, Clone, Copy, Default)]
pub struct SchemeSourceCompiler;

impl crate::SourceCompiler for SchemeSourceCompiler {
    type Error = crate::core_ir::CompilerError;

    fn compile(&self, source: &str) -> Result<crate::core_ir::CoreProgram, Self::Error> {
        compile_to_core_program(source)
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
}
