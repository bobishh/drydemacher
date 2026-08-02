use ecky_cad_lib::steel_data::{
    from_immutable_steel, parse_steel_data, to_immutable_steel, write_steel_data, SteelDataValue,
};
use steel_core::rvals::{SteelVal, SteelVector};

const SENTINEL: &str = "private-secret-source";

fn keyword(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push((b'a' + (index % 26) as u8) as char);
        index /= 26;
        if index == 0 {
            break;
        }
    }
    format!(":{}", letters.into_iter().rev().collect::<String>())
}

fn assert_safe_limit_error(input: &str, cause: &str) {
    let error = parse_steel_data(input).unwrap_err();
    assert!(error.message.contains(cause), "{error}");
    assert!(input.is_char_boundary(error.offset), "{error}");
    let diagnostic = error.to_string();
    assert!(diagnostic.contains("at byte "), "{diagnostic}");
    assert!(diagnostic.contains("line "), "{diagnostic}");
    assert!(diagnostic.contains("column "), "{diagnostic}");
    assert!(!diagnostic.contains(SENTINEL), "{diagnostic}");
}

#[test]
fn commented_data_canonicalizes_without_evaluation() {
    let value = parse_steel_data(
        "; input comment\n{:z 1e+03, :a [true nil \"line\\n\\u0001\"], :ns/name :value}\n",
    )
    .expect("accepted data");

    assert_eq!(
        value,
        SteelDataValue::Map(vec![
            (":z".into(), SteelDataValue::Float(1000.0),),
            (
                ":a".into(),
                SteelDataValue::Vector(vec![
                    SteelDataValue::Bool(true),
                    SteelDataValue::Nil,
                    SteelDataValue::String("line\n\u{1}".into()),
                ]),
            ),
            (":ns/name".into(), SteelDataValue::Keyword(":value".into()),),
        ])
    );
    assert_eq!(
        write_steel_data(&value).unwrap(),
        "{:a [true nil \"line\\n\\u0001\"] :ns/name :value :z 1.0e3}\n"
    );
}

#[test]
fn keyword_segments_allow_digits_after_first_letter() {
    for keyword in [":build123d", ":schema2/value3", ":ns2/build123d-v2"] {
        assert_eq!(
            parse_steel_data(keyword).unwrap(),
            SteelDataValue::Keyword(keyword.into())
        );
        assert_eq!(
            write_steel_data(&SteelDataValue::Keyword(keyword.into())).unwrap(),
            format!("{keyword}\n")
        );
    }
}

#[test]
fn keyword_segments_reject_invalid_initials_and_empty_parts() {
    for keyword in [
        ":1build",
        ":ns/2value",
        ":build-2d",
        ":Build123d",
        ":ns/Name2",
        ":build--v2",
        ":ns/",
        ":/name",
    ] {
        assert!(parse_steel_data(keyword).is_err(), "{keyword}");
        assert!(
            write_steel_data(&SteelDataValue::Keyword(keyword.into())).is_err(),
            "{keyword}"
        );
    }
}

#[test]
fn forbidden_classes_report_stable_causes() {
    for (input, cause) in [
        ("()", "lists are forbidden"),
        ("#{:a}", "tags and sets are forbidden"),
        ("#tag nil", "tags and sets are forbidden"),
        ("'nil", "quote forms are forbidden"),
        ("`nil", "quote forms are forbidden"),
        ("~nil", "quote forms are forbidden"),
        ("~@nil", "quote forms are forbidden"),
        (".", "dotted forms"),
        ("eval", "symbols"),
    ] {
        let error = parse_steel_data(input).unwrap_err();
        assert!(error.message.contains(cause), "{input:?}: {error}");
        assert_eq!((error.offset, error.line, error.column), (0, 1, 1));
    }
}

#[test]
fn hostile_forms_and_integer_overflow_fail_with_location() {
    for input in [
        "()",
        "#{:a}",
        "(eval \"bad\")",
        "'(1 2)",
        "`[1]",
        "~value",
        "~@value",
        "(a . b)",
        ".",
        "eval",
        "#instant \"2026-01-01\"",
        "{:a 1 :a 2}",
        "{:n 9223372036854775808}",
        "{:n -9223372036854775809}",
        "{:n NaN}",
    ] {
        let error = parse_steel_data(input).expect_err(input);
        assert!(error.to_string().contains("line 1, column"), "{error}");
    }
}

#[test]
fn malformed_utf8_adjacent_offsets_never_panic() {
    let mut split_limit = " ".repeat(1024 * 1024 - 1);
    split_limit.push('é');
    for input in [&split_limit, "\"\\é\"", "\"\\u€é\""] {
        let outcome = std::panic::catch_unwind(|| parse_steel_data(input));
        assert!(outcome.is_ok(), "parser panicked");
        let error = outcome.unwrap().expect_err("malformed input accepted");
        assert!(input.is_char_boundary(error.offset), "{error}");
    }
}

#[test]
fn diagnostics_pin_location_and_never_echo_source_text() {
    let duplicate = parse_steel_data("\n  {:private-secret 1 :private-secret 2}").unwrap_err();
    assert_eq!(
        (duplicate.offset, duplicate.line, duplicate.column),
        (22, 2, 22)
    );
    assert!(duplicate.message.contains("duplicate map key"));
    assert!(!duplicate.to_string().contains("private-secret"));

    let overflow = parse_steel_data("{:n 9223372036854775808}").unwrap_err();
    assert_eq!((overflow.offset, overflow.line, overflow.column), (4, 1, 5));
    assert_eq!(overflow.message, "integer overflow");
}

#[test]
fn map_keys_consume_node_budget() {
    let one_map = format!(
        "{{{}}}",
        (0..5_000)
            .map(|index| format!("{} nil", keyword(index)))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let input = format!(
        "[{}]",
        (0..10)
            .map(|_| one_map.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert!(input.len() < 1024 * 1024);
    let error = parse_steel_data(&input).unwrap_err();
    assert!(error.message.contains("node limit"));
}

#[test]
fn map_collection_limit_accepts_boundary_and_rejects_next_entry() {
    let boundary = format!(
        "{{{}}}",
        (0..10_000)
            .map(|index| format!("{} nil", keyword(index)))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let value = parse_steel_data(&boundary).expect("10000 map entries");
    assert!(matches!(value, SteelDataValue::Map(entries) if entries.len() == 10_000));

    let over_limit = format!(
        "{{:private-secret-source nil {}}}",
        (0..10_000)
            .map(|index| format!("{} nil", keyword(index)))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert_safe_limit_error(&over_limit, "map entry limit");
}

#[test]
fn every_budget_failure_has_safe_location_and_cause() {
    let oversized_input = format!("; {SENTINEL}\n{}", " ".repeat(1024 * 1024));
    assert_safe_limit_error(&oversized_input, "input exceeds 1 MiB");

    let excessive_depth = format!("; {SENTINEL}\n{}nil{}", "[".repeat(65), "]".repeat(65));
    assert_safe_limit_error(&excessive_depth, "nesting depth");

    fn wide_tree(depth: usize) -> String {
        if depth == 0 {
            return "nil".into();
        }
        format!(
            "[{}]",
            (0..10)
                .map(|_| wide_tree(depth - 1))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
    let excessive_nodes = format!("; {SENTINEL}\n{}", wide_tree(5));
    assert_safe_limit_error(&excessive_nodes, "node limit");

    let excessive_string = format!("\"{SENTINEL}{}\"", "x".repeat(256 * 1024));
    assert_safe_limit_error(&excessive_string, "decoded string");

    let excessive_vector = format!("[\"{SENTINEL}\" {}]", "nil ".repeat(10_000));
    assert_safe_limit_error(&excessive_vector, "vector entry limit");

    let excessive_number = format!("; {SENTINEL}\n1{}", "0".repeat(128));
    assert_safe_limit_error(&excessive_number, "numeric token");
}

#[test]
fn exactly_one_root_form_is_required() {
    let error = parse_steel_data("nil true").unwrap_err();
    assert_eq!((error.offset, error.line, error.column), (4, 1, 5));
    assert_eq!(error.message, "expected EOF after top-level form");
}

#[test]
fn limits_and_noncanonical_forms_fail_closed() {
    let deep = format!("{}nil{}", "[".repeat(65), "]".repeat(65));
    fn wide_tree(depth: usize) -> String {
        if depth == 0 {
            return "nil".into();
        }
        format!(
            "[{}]",
            (0..10)
                .map(|_| wide_tree(depth - 1))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
    let too_many_nodes = wide_tree(5);
    let too_many_entries = format!("[{}]", "nil ".repeat(10_001));
    let too_long_number = format!("1{}", "0".repeat(128));
    let too_long_string = format!("\"{}\"", "x".repeat(256 * 1024 + 1));
    let too_large_input = " ".repeat(1024 * 1024 + 1);

    for input in [
        "{:n 01}",
        "{:n 1.}",
        "{:n Infinity}",
        "{:bad/key/name 1}",
        &deep,
        &too_many_entries,
        &too_many_nodes,
        &too_long_number,
        &too_long_string,
        &too_large_input,
    ] {
        assert!(parse_steel_data(input).is_err(), "must reject: {input:?}");
    }
}

#[test]
fn canonical_output_is_idempotent_and_preserves_unicode() {
    let input = "{:z -0.0 :a \"\\ud83d\\ude03\" :m {:b 1 :a 2}}";
    let once = write_steel_data(&parse_steel_data(input).unwrap()).unwrap();
    let twice = write_steel_data(&parse_steel_data(&once).unwrap()).unwrap();
    assert_eq!(once, twice);
    assert_eq!(once, "{:a \"😃\" :m {:a 2 :b 1} :z 0.0}\n");
}

#[test]
fn public_invalid_values_cannot_serialize() {
    for value in [
        SteelDataValue::Float(f64::NAN),
        SteelDataValue::Float(f64::INFINITY),
        SteelDataValue::Keyword("secret".into()),
        SteelDataValue::Keyword(":Bad".into()),
        SteelDataValue::Map(vec![
            (":a".into(), SteelDataValue::Nil),
            (":a".into(), SteelDataValue::Bool(true)),
        ]),
        SteelDataValue::Map(vec![("secret".into(), SteelDataValue::Nil)]),
        SteelDataValue::Vector(vec![SteelDataValue::Nil; 10_001]),
        SteelDataValue::String("x".repeat(256 * 1024 + 1)),
        SteelDataValue::Vector(vec![SteelDataValue::String("x".repeat(256 * 1024)); 4]),
    ] {
        assert!(write_steel_data(&value).is_err());
    }
}

#[test]
fn writer_counts_map_keys_toward_node_budget() {
    let map = SteelDataValue::Map(
        (0..5_000)
            .map(|index| (keyword(index), SteelDataValue::Nil))
            .collect(),
    );
    let value = SteelDataValue::Vector(vec![map; 10]);
    let error = write_steel_data(&value).unwrap_err();
    assert!(error.message.contains("node limit"));
}

#[test]
fn exponential_float_mantissa_retains_float_identity() {
    assert_eq!(
        write_steel_data(&SteelDataValue::Float(1e100)).unwrap(),
        "1.0e100\n"
    );
}

#[test]
fn immutable_steel_bridge_accepts_data_and_rejects_symbols_or_nonfinite_values() {
    let data = SteelDataValue::Vector(vec![
        SteelDataValue::Nil,
        SteelDataValue::Bool(true),
        SteelDataValue::Integer(42),
        SteelDataValue::Float(1.5),
        SteelDataValue::String("text".into()),
        SteelDataValue::Keyword(":kind".into()),
    ]);
    let steel = to_immutable_steel(&data).unwrap();
    assert_eq!(from_immutable_steel(&steel).unwrap(), data);

    let immutable = SteelVal::VectorV(
        vec![SteelVal::SymbolV("bare-symbol".into())]
            .into_iter()
            .collect::<SteelVector>(),
    );
    assert!(from_immutable_steel(&immutable).is_err());
    assert!(from_immutable_steel(&SteelVal::NumV(f64::NAN)).is_err());
}
