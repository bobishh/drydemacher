use std::collections::BTreeMap;

use ecky_render::scheme::SchemeSourceCompiler;
use ecky_render::{KernelPlanner, PortableKernelPlanner, SourceCompiler};

#[test]
fn compiled_source_lowers_to_a_portable_kernel_plan() {
    let program = SchemeSourceCompiler
        .compile(
            r#"
            (model
              (part body
                (translate 1 2 3
                  (box 10 20 30))))
            "#,
        )
        .expect("source compiles");

    let plan = PortableKernelPlanner
        .plan(&program, &BTreeMap::new())
        .expect("normalized Core IR lowers");

    assert_eq!(plan.schema_version, 1);
    assert_eq!(plan.parts.len(), 1);
    assert_eq!(plan.parts[0].key, "body");
    assert_eq!(
        plan.parts[0]
            .commands
            .iter()
            .map(|command| command.op.as_str())
            .collect::<Vec<_>>(),
        vec!["box", "translate"]
    );
    assert_eq!(plan.parts[0].root, plan.parts[0].commands[1].output);
}

#[test]
fn portable_planner_rejects_unresolved_host_assets() {
    let program = SchemeSourceCompiler
        .compile(r#"(model (part body (svg "/tmp/host-only.svg" 20 20)))"#)
        .expect("source compiles");

    let error = PortableKernelPlanner
        .plan(&program, &BTreeMap::new())
        .expect_err("host asset must be resolved before portable lowering");

    assert_eq!(error.code(), "unresolvedAsset");
    assert!(error.to_string().contains("svg"));
}
