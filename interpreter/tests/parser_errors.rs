use interpreter::{parse, parser::Parser};
#[test]
fn parser_recovers_at_declaration_boundaries() {
    let errors = parse("var = 1; print ; var fine=2;").unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors[0].message.contains("variable name"));
    assert!(errors[1].message.contains("expression"));
}
#[test]
fn missing_delimiters_and_scanner_errors_never_execute() {
    for source in [
        "print (1;",
        "{ print 1;",
        "if true print 1;",
        "a.;",
        "@ print 1;",
        "print 1",
        "}",
    ] {
        assert!(parse(source).is_err(), "{source}");
    }
    assert!(parse("").unwrap().is_empty());
    assert!(Parser::new(vec![]).parse().unwrap().is_empty());
}
