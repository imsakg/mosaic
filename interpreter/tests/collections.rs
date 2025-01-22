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

// More collection contracts.
