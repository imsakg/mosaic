use interpreter::{lexer::Lexer, token::TokenType};
#[test]
fn numbers_keep_the_first_digit() {
    let tokens = Lexer::new("123 4.5 0".into()).scan_tokens();
    assert_eq!(tokens[0].token_type, TokenType::Number(123.0));
    assert_eq!(tokens[1].token_type, TokenType::Number(4.5));
    assert_eq!(tokens[2].token_type, TokenType::Number(0.0));
}
