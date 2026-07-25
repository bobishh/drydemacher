use ecky_render::core_ir::{
    parse_core_edge_selector_payload, parse_core_face_selector_payload, verify_core_program,
    CoreEdgeAxis, CoreEdgeBound, CoreEdgeSelectorClause, CoreFaceSelectorClause, CoreLiteral,
    CoreNode, CoreNodeKind, CoreOperation, CorePart, CorePrimitive, CoreProgram,
    CoreSelectorPayload, CoreValueKind, NodeId, PartId, ProgramId,
};
use ecky_render::deterministic;

#[test]
fn core_ir_is_owned_by_the_platform_neutral_crate() {
    let number = |id, value| {
        CoreNode::new(
            NodeId::new(id),
            CoreNodeKind::Literal(CoreLiteral::Number(value)),
            CoreValueKind::Number,
        )
    };
    let root = CoreNode::new(
        NodeId::new(4),
        CoreNodeKind::Call {
            op: CoreOperation::Primitive(CorePrimitive::Box),
            args: vec![number(1, 10.0), number(2, 20.0), number(3, 30.0)],
            keywords: Vec::new(),
        },
        CoreValueKind::Solid,
    );
    let program = CoreProgram::new(
        ProgramId::new(1),
        Vec::new(),
        vec![CorePart {
            id: PartId::new(1),
            key: "body".into(),
            label: "Body".into(),
            root,
        }],
    );

    verify_core_program(&program).expect("box program validates");
    assert_eq!(
        deterministic::noise2(0.25, 0.75, 42.0),
        deterministic::noise2(0.25, 0.75, 42.0)
    );
}

#[test]
fn core_selector_grammar_lives_with_core_ir() {
    assert_eq!(
        parse_core_edge_selector_payload("x-min+axis-z").expect("edge selector"),
        CoreSelectorPayload::EdgeClauses(vec![
            CoreEdgeSelectorClause::Boundary {
                axis: CoreEdgeAxis::X,
                bound: CoreEdgeBound::Min,
            },
            CoreEdgeSelectorClause::Axis(CoreEdgeAxis::Z),
        ])
    );
    assert_eq!(
        parse_core_face_selector_payload("planar+normal-z").expect("face selector"),
        CoreSelectorPayload::FaceClauses(vec![
            CoreFaceSelectorClause::Planar,
            CoreFaceSelectorClause::Normal(CoreEdgeAxis::Z),
        ])
    );
    assert!(parse_core_edge_selector_payload("target-id:body:face:0").is_err());
}
