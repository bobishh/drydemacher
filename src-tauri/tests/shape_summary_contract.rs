use ecky_cad_lib::contracts::PartBinding;
use ecky_cad_lib::shape_summary::{
    decode_shape_summary, encode_shape_summary, normalize_freecad_shape_summary,
    shape_parts_from_part_bindings, FreecadShapeSummaryJson,
};
use ecky_cad_lib::steel_data::{write_steel_data, SteelDataValue};

fn freecad_fixture() -> FreecadShapeSummaryJson {
    serde_json::from_value(serde_json::json!({
        "source": {
            "format": "FCStd",
            "name": "/private/projects/bracket.FCStd",
            "hash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        },
        "topology": { "solids": 1, "shells": 1, "faces": 6, "edges": 12, "vertices": 8 },
        "bounds": { "min": [0.0, 0.0, 0.0], "max": [10.0, 20.0, 30.0] },
        "parts": [
            { "id": "z", "label": "Zulu", "kind": "solid", "volume": 10.0, "area": 20.0 },
            { "id": "ä", "label": "Umlaut", "kind": "mesh", "volume": 1.0, "area": 2.0 }
        ]
    }))
    .unwrap()
}

#[test]
fn bdd_shape_summary_has_exact_fields_and_utf8_sorted_parts() {
    let summary = normalize_freecad_shape_summary(freecad_fixture()).unwrap();
    let encoded = encode_shape_summary(&summary).unwrap();
    let text = write_steel_data(&encoded).unwrap();
    assert!(text.contains(":schema :ecky/shape-summary"));
    assert!(text.find(":id \"z\"").unwrap() < text.find(":id \"ä\"").unwrap());
    assert!(text.contains(":name \"bracket.FCStd\""));
    assert_eq!(decode_shape_summary(&encoded).unwrap(), summary);

    let mut unknown = encoded.clone();
    let SteelDataValue::Map(root) = &mut unknown else {
        unreachable!()
    };
    root.push((
        ":leaked-path".into(),
        SteelDataValue::String("/private/secret".into()),
    ));
    assert!(decode_shape_summary(&unknown).is_err());

    let mut empty = freecad_fixture();
    empty.parts.clear();
    let empty = normalize_freecad_shape_summary(empty).unwrap();
    let empty_edn = write_steel_data(&encode_shape_summary(&empty).unwrap()).unwrap();
    assert!(empty_edn.contains(":parts []"));
}

#[test]
fn bdd_shape_summary_rejects_missing_metrics_and_invalid_topology_integers() {
    let mut missing = serde_json::to_value(freecad_fixture()).unwrap();
    missing["parts"][0].as_object_mut().unwrap().remove("area");
    assert!(serde_json::from_value::<FreecadShapeSummaryJson>(missing).is_err());

    let mut negative = serde_json::to_value(freecad_fixture()).unwrap();
    negative["topology"]["faces"] = serde_json::json!(-1);
    assert!(serde_json::from_value::<FreecadShapeSummaryJson>(negative).is_err());

    let mut overflow = serde_json::to_value(freecad_fixture()).unwrap();
    overflow["topology"]["faces"] = serde_json::json!(9_223_372_036_854_775_808u64);
    let parsed: FreecadShapeSummaryJson = serde_json::from_value(overflow).unwrap();
    assert!(normalize_freecad_shape_summary(parsed).is_err());
}

#[test]
fn bdd_freecad_boundary_rejects_unknowns_and_non_authoritative_part_metrics() {
    let mut unknown = serde_json::to_value(freecad_fixture()).unwrap();
    unknown["extractorPath"] = serde_json::json!("/private/freecad.py");
    assert!(serde_json::from_value::<FreecadShapeSummaryJson>(unknown).is_err());

    let mut invalid = freecad_fixture();
    invalid.parts[0].volume = f64::NAN;
    assert!(normalize_freecad_shape_summary(invalid).is_err());
}

#[test]
fn bdd_part_binding_requires_authoritative_metrics_before_summary_emission() {
    let mut binding = PartBinding {
        part_id: "body".into(),
        freecad_object_name: "Body".into(),
        label: "Body".into(),
        kind: "Part::Feature".into(),
        semantic_role: None,
        viewer_asset_path: None,
        viewer_node_ids: vec![],
        parameter_keys: vec![],
        editable: true,
        bounds: None,
        volume: None,
        area: None,
    };
    assert!(shape_parts_from_part_bindings(&[binding.clone()]).is_err());
    binding.volume = Some(12.0);
    binding.area = Some(34.0);
    let parts = shape_parts_from_part_bindings(&[binding]).unwrap();
    assert_eq!(parts[0].kind, "solid");
}
