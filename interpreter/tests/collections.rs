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

// More collection contracts.
