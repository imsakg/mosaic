use interpreter::tree::Interpreter;

#[test]
fn failed_nested_calls_report_innermost_frames_first() {
    let source = "fun fail() { return 1 + false; }\nfun middle() { fail(); }\nmiddle();";
    let error = Interpreter::new().run(source).unwrap_err().remove(0);
    assert_eq!(error.line, 1);
    assert!(
        error
            .message
            .contains("\n  at fail (line 2)\n  at middle (line 3)"),
        "{error}"
    );
}

#[test]
fn local_function_names_and_arity_errors_remain_readable() {
    let source = "{ fun local(a) {} local(); }";
    let error = Interpreter::new().run(source).unwrap_err().remove(0);
    assert!(error.message.contains("Expected 1 arguments but got 0."));
    assert!(error.message.contains("at local (line 1)"));
    assert!(!error.message.contains('@'));
}

#[test]
fn recursive_traces_are_bounded_and_session_state_recovers() {
    let mut interpreter = Interpreter::new();
    let error = interpreter
        .run("fun recurse() { recurse(); } recurse();")
        .unwrap_err()
        .remove(0);
    assert!(error.message.contains("Call depth limit"));
    assert_eq!(
        error
            .message
            .lines()
            .filter(|line| line.starts_with("  at "))
            .count(),
        16
    );
    assert!(error.message.contains("additional frames omitted"));
    assert_eq!(
        interpreter
            .run("fun ok() { return 7; } print ok();")
            .unwrap(),
        ["7"]
    );
}
