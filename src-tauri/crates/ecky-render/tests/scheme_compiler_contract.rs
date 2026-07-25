use ecky_render::core_ir::{CoreNodeKind, CoreOperation, CorePrimitive};
use ecky_render::scheme::SchemeSourceCompiler;
use ecky_render::SourceCompiler;

#[test]
fn scheme_source_compiles_inside_platform_neutral_crate() {
    let program = SchemeSourceCompiler
        .compile(
            r#"
        (model
          (params (number width 12 :unit length))
          (part body (box width 20mm 30mm)))
        "#,
        )
        .expect("scheme source compiles");

    assert_eq!(program.parts.len(), 1);
    assert!(matches!(
        program.parts[0].root.kind,
        CoreNodeKind::Call {
            op: CoreOperation::Primitive(CorePrimitive::Box),
            ..
        }
    ));
}
