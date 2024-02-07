use interpreter::tree::Interpreter;

#[test]
fn native_functions_are_values_with_names_and_exact_arity() {
    let mut interpreter = Interpreter::new();
    assert_eq!(interpreter.run("print clock;").unwrap(), ["<native clock>"]);
    assert_eq!(
        interpreter
            .run("var now = clock; print now() > 0;")
            .unwrap(),
        ["true"]
    );
    assert!(interpreter.run("clock(1);").unwrap_err()[0]
        .message
        .contains("Expected 0"));
    assert_eq!(
        interpreter.run("var clock = 7; print clock;").unwrap(),
        ["7"]
    );
}
