use interpreter::tree::Interpreter;

fn output(source: &str) -> Vec<String> {
    Interpreter::new().run(source).unwrap()
}

#[test]
fn tuples_are_immutable_values_with_structural_equality() {
    assert_eq!(
        output(
            r#"print tuple(); print tuple(1); print tuple(1, "hello", tuple(false, nil)); print tuple(1, 2) == tuple(1, 2); print !tuple();"#
        ),
        ["()", "(1,)", "(1, hello, (false, nil))", "true", "false"]
    );
}

#[test]
fn length_counts_unicode_scalars_and_tuple_items() {
    assert_eq!(
        output(r#"print len(tuple(1, 2)); print len(tuple()); print len("aé🦀"); print len("");"#),
        ["2", "0", "3", "0"]
    );
    let error = Interpreter::new().run("len(7);").unwrap_err();
    assert!(error[0].message.contains("string or tuple"));
    assert!(Interpreter::new().run("len();").is_err());
}

#[test]
fn introspection_distinguishes_values_and_renders_without_coercion() {
    assert_eq!(
        output(
            r#"class C {} fun f() {} print type_of(nil); print type_of(true); print type_of(3); print type_of("x"); print type_of(tuple()); print type_of(f); print type_of(clock); print type_of(C); print type_of(C()); print str(tuple(1, false));"#
        ),
        [
            "nil",
            "boolean",
            "number",
            "string",
            "tuple",
            "function",
            "function",
            "class",
            "instance",
            "(1, false)"
        ]
    );
    assert!(Interpreter::new().run("str();").is_err());
    assert!(Interpreter::new().run("type_of(1, 2);").is_err());
}

#[test]
fn tuple_lookup_checks_indices_without_panicking() {
    assert_eq!(output("print get(tuple(9, nil), 0); print get(tuple(9), 1); print get(tuple(), 99999999999999999999);"), ["9", "nil", "nil"]);
    for source in [
        "get(tuple(), -1);",
        "get(tuple(), 0.5);",
        "get(tuple(), 0/0);",
        "get(tuple(), 1/0);",
        "get(1, 0);",
    ] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
}

#[test]
fn append_returns_a_new_tuple_without_mutating_aliases() {
    assert_eq!(output("var a = tuple(1); var b = a; print append(a, 2); print b; print append(tuple(), tuple(3));"), ["(1, 2)", "(1,)", "((3,),)"]);
    assert!(Interpreter::new().run("append(1, 2);").is_err());
    assert!(Interpreter::new().run("append(tuple());").is_err());
}

#[test]
fn concatenation_preserves_order_and_nested_values() {
    assert_eq!(output("print concat(tuple(1, 2), tuple(3)); print concat(tuple(), tuple()); print concat(tuple(tuple(1)), tuple(nil));"), ["(1, 2, 3)", "()", "((1,), nil)"]);
    assert!(Interpreter::new().run("concat(tuple(), 1);").is_err());
}

#[test]
fn slices_clamp_unicode_and_tuple_boundaries() {
    assert_eq!(
        output(
            r#"print slice(tuple(1, 2, 3), 1, 9); print slice("aé🦀z", 1, 3); print slice("hello", 99, 100); print slice(tuple(), 0, 0);"#
        ),
        ["(2, 3)", "é🦀", "", "()"]
    );
    for source in [
        "slice(tuple(), 2, 1);",
        "slice(1, 0, 1);",
        "slice(tuple(), -1, 0);",
        "slice(tuple(), 0, 0.5);",
    ] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
}

#[test]
fn maps_use_string_keys_and_deterministic_value_semantics() {
    assert_eq!(
        output(
            r#"print map(); print map(tuple("z", 1), tuple("a", 2)); print map(tuple("a", 1), tuple("a", 3)); print len(map(tuple("x", 1))); print type_of(map()); print map(tuple("x", 1)) == map(tuple("x", 1)); print !map();"#
        ),
        [
            "{}",
            r#"{"a": 2, "z": 1}"#,
            r#"{"a": 3}"#,
            "1",
            "map",
            "true",
            "false"
        ]
    );
    for source in ["map(1);", "map(tuple(1, 2));", "map(tuple(1));"] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
}

#[test]
fn map_lookup_distinguishes_absent_keys_from_stored_nil() {
    assert_eq!(
        output(
            r#"var m = map(tuple("x", nil), tuple("a", 3)); print get(m, "a"); print get(m, "x"); print get(m, "unknown"); print has(m, "x"); print has(m, "unknown");"#
        ),
        ["3", "nil", "nil", "true", "false"]
    );
    for source in ["get(map(), 1);", "has(map(), 1);", r#"has(tuple(), "x");"#] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
}

#[test]
fn put_preserves_map_aliases_and_replaces_existing_keys() {
    assert_eq!(
        output(
            r#"var a = map(tuple("x", 1)); var b = put(a, "x", 2); print get(a, "x"); print get(b, "x"); print len(put(b, "y", nil)); print has(b, "y");"#
        ),
        ["1", "2", "2", "false"]
    );
    for source in [
        "put(map(), 1, 2);",
        r#"put(tuple(), "x", 2);"#,
        "put(map());",
    ] {
        assert!(Interpreter::new().run(source).is_err(), "{source}");
    }
}

#[test]
fn remove_returns_an_unchanged_value_when_key_is_absent() {
    assert_eq!(
        output(
            r#"var a = map(tuple("x", nil)); print has(remove(a, "x"), "x"); print has(a, "x"); print remove(a, "missing") == a; print len(remove(map(), "x"));"#
        ),
        ["false", "true", "true", "0"]
    );
    assert!(Interpreter::new().run("remove(map(), 2);").is_err());
}

// More collection contracts.
