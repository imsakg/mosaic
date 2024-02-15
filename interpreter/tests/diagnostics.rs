use interpreter::{error::Diagnostic, lexer::Lexer, token::TokenType};
#[test]
fn diagnostics_are_values_with_stable_display() {
    let d = Diagnostic::new("Parse", 7, "Expected ';'.");
    assert_eq!(d.to_string(), "[line 7] Parse error: Expected ';'.");
    assert_eq!(d.clone(), d);
}
#[test]
fn scanners_collect_multiple_errors_and_finish_once() {
    let mut lexer = Lexer::new("@\n# 9".into());
    let ts = lexer.scan_tokens();
    assert_eq!(lexer.errors().len(), 2);
    assert_eq!(lexer.errors()[1].line, 2);
    assert_eq!(ts.last().unwrap().token_type, TokenType::EOF);
    assert_eq!(lexer.scan_tokens(), ts);
    assert_eq!(lexer.errors().len(), 2);
}
#[test]
fn overflowing_numeric_literals_are_rejected() {
    let mut lexer = Lexer::new("9".repeat(400));
    lexer.scan_tokens();
    assert!(lexer.errors()[0].message.contains("out of range"));
}
