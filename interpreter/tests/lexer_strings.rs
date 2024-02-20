use interpreter::{lexer::Lexer, token::TokenType as T};
#[test]
fn escape_sequences_and_multiline_strings_preserve_values() {
    let mut lexer = Lexer::new("\"a\\n\\t\\\"\\\\\" \"one\ntwo\" print".into());
    let ts = lexer.scan_tokens();
    assert!(lexer.errors().is_empty());
    assert_eq!(ts[0].token_type, T::String("a\n\t\"\\".into()));
    assert_eq!(ts[1].token_type, T::String("one\ntwo".into()));
    assert_eq!(ts[1].line, 1);
    assert_eq!(ts[2].line, 2);
}
#[test]
fn bad_escape_does_not_prevent_following_tokens() {
    let mut lexer = Lexer::new("\"\\q\" print 3;".into());
    let ts = lexer.scan_tokens();
    assert_eq!(lexer.errors().len(), 1);
    assert_eq!(ts[1].token_type, T::Print);
}
