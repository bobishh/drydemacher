struct Boundary<'a> {
    name: &'a str,
    source: &'a str,
    evidence: &'a str,
}

#[test]
fn external_json_adapter_inventory_covers_protocol_boundaries_not_config_persistence() {
    let boundaries = [
        Boundary {
            name: "MCP",
            source: include_str!("../src/mcp/contracts.rs"),
            evidence: "serde",
        },
        Boundary {
            name: "Tauri/Specta",
            source: include_str!("../src/commands/config.rs"),
            evidence: "#[specta::specta]",
        },
        Boundary {
            name: "provider REST",
            source: include_str!("../src/llm.rs"),
            evidence: "response.json()",
        },
        Boundary {
            name: "Direct-OCCT plan/report",
            source: include_str!("../src/ecky_cad_host/direct_occt_runner.rs"),
            evidence: "serde_json::from_str",
        },
        Boundary {
            name: "Build123d report",
            source: include_str!("../src/build123d.rs"),
            evidence: "RunnerReport",
        },
        Boundary {
            name: "FreeCAD report",
            source: include_str!("../src/freecad.rs"),
            evidence: "RunnerReport",
        },
        Boundary {
            name: "project mirror",
            source: include_str!("../src/project_mirror.rs"),
            evidence: "serde_json::",
        },
        Boundary {
            name: "package/archive/index",
            source: include_str!("../src/component_package_runtime.rs"),
            evidence: "serde_json::",
        },
        Boundary {
            name: "runtime manifests",
            source: include_str!("../src/model_runtime.rs"),
            evidence: "serde_json::",
        },
        Boundary {
            name: "database JSON columns",
            source: include_str!("../src/db.rs"),
            evidence: "serde_json::",
        },
    ];

    for boundary in boundaries {
        assert!(
            boundary.source.contains(boundary.evidence),
            "{} boundary must retain typed JSON adapter evidence: {}",
            boundary.name,
            boundary.evidence
        );
    }

    let persistence = include_str!("../src/config_store.rs");
    assert!(persistence.contains("CONFIG_EDN_FILE"));
    assert!(persistence.contains("CONFIG_JSON_FILE"));
    assert!(persistence.contains("serde_json::from_str(&raw)"));
    assert!(!persistence.contains("to_string(&config)"));
    assert!(!persistence.contains("to_vec(&config)"));
}
