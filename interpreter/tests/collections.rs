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

// More collection contracts.
