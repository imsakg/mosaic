use interpreter::parse;
#[test]
fn argument_and_parameter_limits_are_reported() {
    let args = vec!["1"; 256].join(",");
    let error = parse(&format!("f({args});")).unwrap_err();
    assert!(error[0].message.contains("255 arguments"));
    let params = (0..256)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(parse(&format!("fun f({params}) {{}}")).unwrap_err()[0]
        .message
        .contains("255 parameters"));
}
#[test]
fn excessively_nested_grouping_is_a_diagnostic() {
    let source = format!("print {}1{};", "(".repeat(100), ")".repeat(100));
    assert!(parse(&source)
        .unwrap_err()
        .iter()
        .any(|e| e.message.contains("nesting limit")));
}
