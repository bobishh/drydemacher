use ecky_cad_lib::ecky_cad_host::source_compiler::NativeSourceCompiler;
use ecky_render::core_ir::{CoreNodeKind, CoreOperation, CorePrimitive};
use ecky_render::SourceCompiler;

#[test]
fn native_source_compiler_implements_the_render_crate_port() {
    let program = NativeSourceCompiler
        .compile("(model (part body (box 10 20 30)))")
        .expect("source compiles through port");

    assert_eq!(program.parts.len(), 1);
    assert!(matches!(
        program.parts[0].root.kind,
        CoreNodeKind::Call {
            op: CoreOperation::Primitive(CorePrimitive::Box),
            ..
        }
    ));
}
