use interpreter::{lexer::Lexer, token::TokenType as T};
#[test]
fn punctuation_and_two_character_operators_use_maximal_munch() {
    let mut lexer = Lexer::new("(){} ,.-+;/*x*/* ! != = == > >= < <=".into());
    let actual: Vec<_> = lexer
        .scan_tokens()
        .into_iter()
        .map(|t| t.token_type)
        .collect();
    assert!(lexer.errors().is_empty());
    assert_eq!(
        actual,
        vec![
            T::LeftParen,
            T::RightParen,
            T::LeftBrace,
            T::RightBrace,
            T::Comma,
            T::Dot,
            T::Minus,
            T::Plus,
            T::Semicolon,
            T::Star,
            T::Bang,
            T::BangEqual,
            T::Equal,
            T::EqualEqual,
            T::Greater,
            T::GreaterEqual,
            T::Less,
            T::LessEqual,
            T::EOF
        ]
    );
}
#[test]
fn trailing_decimal_dot_is_a_separate_token() {
    let ts = Lexer::new("12. 0.25".into()).scan_tokens();
    assert_eq!(ts[0].token_type, T::Number(12.0));
    assert_eq!(ts[1].token_type, T::Dot);
    assert_eq!(ts[2].token_type, T::Number(0.25));
}
