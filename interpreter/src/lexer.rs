use crate::error::{Error, ErrorReporter};

use super::token::{Token, TokenType, KEYWORDS};

#[derive(Debug)]
pub struct Lexer {
    source: String,
    start: usize,
    current: usize,
    line: usize,
    tokens: Vec<Token>,
    error_reporter: ErrorReporter,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
            tokens: Vec::new(),
            error_reporter: ErrorReporter::new(),
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }
        self.tokens.push(Token {
            token_type: TokenType::EOF,
            lexeme: String::new(),
            literal: None,
            line: self.line,
        });
        self.tokens.clone()
    }

    fn scan_token(&mut self) {
        if self.is_at_end() {
            return;
        }

        let c = self.advance();

        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            ';' => self.add_token(TokenType::Semicolon),
            '*' => self.add_token(TokenType::Star),
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenType::BangEqual)
                } else {
                    self.add_token(TokenType::Bang)
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenType::EqualEqual)
                } else {
                    self.add_token(TokenType::Equal)
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenType::LessEqual)
                } else {
                    self.add_token(TokenType::Less)
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenType::GreaterEqual)
                } else {
                    self.add_token(TokenType::Greater)
                }
            }
            '/' => {
                if self.match_char('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                    // self.scan_token() <- This is not needed
                } else {
                    self.add_token(TokenType::Slash)
                }
            }
            // Ignore whitespaces
            ' ' | '\r' | '\t' => self.scan_token(),
            '\n' => {
                self.line += 1;
                self.scan_token()
            }

            // Number Constants
            '0'..='9' => {
                self.number();
            }

            // Identifiers
            'a'..='z' | 'A'..='Z' | '_' => {
                self.identifier();
            }

            // String Literals
            '"' => {
                self.string();
            }

            // Handle unexpected character
            _ => {
                self.error_reporter
                    .error(self.line, "Unexpected character.");
                // self.add_token(TokenType::EOF) <- This is not needed
            }
        }
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source.chars().nth(self.current - 1).unwrap()
    }

    fn add_token(&mut self, token_type: TokenType) {
        self.tokens.push(self.make_token(token_type));
    }

    fn add_token_literal(&mut self, token_type: TokenType, literal: Option<String>) {
        self.tokens.push(Token {
            token_type,
            lexeme: self.source[self.start..self.current].to_string(),
            literal,
            line: self.line,
        });
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source.chars().nth(self.current).unwrap() != expected {
            return false;
        }
        self.current += 1;
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.chars().nth(self.current).unwrap()
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            return '\0';
        }
        self.source.chars().nth(self.current + 1).unwrap()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            token_type,
            lexeme: self.source[self.start..self.current].to_string(),
            literal: None,
            line: self.line,
        }
    }

    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let number = self.source[self.start + 1..self.current]
            .parse::<f64>()
            .unwrap();
        self.add_token_literal(TokenType::Number(number), Some(number.to_string()));
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() {
            self.advance();
        }
        let lexeme = self.source[self.start..self.current].to_string();

        let token_type = match KEYWORDS.get(&lexeme.as_str()) {
            Some(token_type) => token_type.clone(),
            None => TokenType::Identifier(lexeme),
        };
        self.add_token(token_type);
    }

    fn string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            eprintln!("[line {}] Unterminated string.", self.line);
            self.add_token(TokenType::EOF);
        }
        self.advance();

        Token {
            token_type: TokenType::String(
                self.source[self.start + 1..self.current - 1].to_string(),
            ),
            lexeme: self.source[self.start..self.current].to_string(),
            literal: None,
            line: self.line,
        }
    }
}
