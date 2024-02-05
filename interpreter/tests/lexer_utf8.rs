use interpreter::{lexer::Lexer, token::TokenType};

#[test]
fn utf8_strings_are_emitted_without_byte_index_panics() {
    let mut lexer = Lexer::new("print \"müzik 🎼\";".into());
    let tokens = lexer.scan_tokens();
    assert_eq!(tokens[1].token_type, TokenType::String("müzik 🎼".into()));
    assert_eq!(tokens[1].lexeme, "\"müzik 🎼\"");
    assert!(lexer.errors().is_empty());
}
#[test]
fn unterminated_literals_produce_diagnostics_instead_of_panicking() {
    let mut lexer = Lexer::new("\"unfinished".into());
    lexer.scan_tokens();
    assert!(lexer.errors()[0].message.contains("Unterminated"));
}
#[test]
fn whitespace_is_iterative_and_scanning_is_idempotent() {
    let mut lexer = Lexer::new(format!("{}var x = 9;", " \n".repeat(20_000)));
    let tokens = lexer.scan_tokens();
    assert_eq!(tokens[0].lexeme, "var");
    assert_eq!(tokens[0].line, 20_001);
    assert_eq!(tokens, lexer.scan_tokens());
}
