use interpreter::parse;

#[test]
fn expected_tokens_are_paired_with_the_unexpected_source_token() {
    let error = parse("var = 1;").unwrap_err().remove(0);
    assert!(error.message.starts_with("Expected variable name."));
    assert!(error.message.contains("Found \"=\""), "{error}");
    let error = parse("print 1\n").unwrap_err().remove(0);
    assert_eq!(error.line, 2);
    assert!(error.message.contains("end of input"), "{error}");
}

#[test]
fn unexpected_literal_labels_are_unicode_safe_and_bounded() {
    let source = format!("var \"{}\";", "🦀".repeat(400));
    let error = parse(&source).unwrap_err().remove(0);
    assert!(error.message.contains('…'), "{error}");
    assert!(error.message.len() < 240);
    assert!(!error.message.contains('\n'));
}
