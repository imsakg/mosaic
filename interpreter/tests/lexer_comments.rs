use interpreter::{lexer::Lexer, token::TokenType as T};
#[test]
fn comments_do_not_leak_tokens_or_break_line_numbers() {
    let mut lexer = Lexer::new("// first\n/* outer /* nested */ end */\nprint 2 / 1;".into());
    let ts = lexer.scan_tokens();
    assert!(lexer.errors().is_empty());
    assert_eq!(ts[0].token_type, T::Print);
    assert_eq!(ts[0].line, 3);
    assert_eq!(ts[2].token_type, T::Slash);
}
#[test]
fn unterminated_comment_is_reported_at_its_opening_line() {
    let mut lexer = Lexer::new("\n/* missing".into());
    lexer.scan_tokens();
    assert_eq!(lexer.errors()[0].line, 2);
    assert!(lexer.errors()[0].message.contains("block comment"));
}
