use interpreter::{lexer::Lexer, token::TokenType as T};
#[test]
fn keywords_use_whole_identifiers_and_allow_internal_underscores() {
    let mut lexer = Lexer::new(
        "var _first_2 = true; variable and or super this class fun return nil false".into(),
    );
    let ts = lexer.scan_tokens();
    assert!(lexer.errors().is_empty());
    assert_eq!(ts[0].token_type, T::Var);
    assert_eq!(ts[1].token_type, T::Identifier("_first_2".into()));
    assert_eq!(ts[5].token_type, T::Identifier("variable".into()));
    assert_eq!(ts[6].token_type, T::And);
    assert_eq!(ts[7].token_type, T::Or);
    assert_eq!(ts[8].token_type, T::Super);
}
#[test]
fn unsupported_unicode_identifier_is_diagnostic_not_a_panic() {
    let mut lexer = Lexer::new("🎼 café".into());
    lexer.scan_tokens();
    assert_eq!(lexer.errors().len(), 2);
}
