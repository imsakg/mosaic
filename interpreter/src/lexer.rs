//! UTF-8-safe linear scanner. Offsets always refer to source bytes.
use crate::error::Diagnostic;
use crate::token::{Token, TokenType as T, KEYWORDS};

#[derive(Debug)]
pub struct Lexer {
    source: String,
    start: usize,
    current: usize,
    line: usize,
    token_line: usize,
    tokens: Vec<Token>,
    errors: Vec<Diagnostic>,
    finished: bool,
}
impl Lexer {
    pub fn new(source: String) -> Self {
        Self { source, start: 0, current: 0, line: 1, token_line: 1,
            tokens: Vec::new(), errors: Vec::new(), finished: false }
    }
    pub fn errors(&self) -> &[Diagnostic] { &self.errors }
    pub fn scan_tokens(&mut self) -> Vec<Token> {
        if !self.finished {
            while self.current < self.source.len() {
                self.start = self.current;
                self.token_line = self.line;
                self.scan_token();
            }
            self.tokens.push(Token { token_type: T::EOF, lexeme: String::new(), literal: None, line: self.line });
            self.finished = true;
        }
        self.tokens.clone()
    }
    fn peek(&self) -> char { self.source[self.current..].chars().next().unwrap_or('\0') }
    fn peek_next(&self) -> char { self.source[self.current..].chars().nth(1).unwrap_or('\0') }
    fn advance(&mut self) -> char {
        let c = self.peek();
        if self.current < self.source.len() { self.current += c.len_utf8(); }
        c
    }
    fn take(&mut self, c: char) -> bool {
        if self.peek() == c && self.current < self.source.len() { self.advance(); true } else { false }
    }
    fn emit(&mut self, token_type: T) {
        let literal = match &token_type {
            T::Number(n) => Some(n.to_string()), T::String(s) => Some(s.clone()), _ => None,
        };
        self.tokens.push(Token { token_type, literal,
            lexeme: self.source[self.start..self.current].into(), line: self.token_line });
    }
    fn error(&mut self, message: impl Into<String>) {
        self.errors.push(Diagnostic::new("Scan", self.token_line, message));
    }
    fn scan_token(&mut self) {
        let c = self.advance();
        let token = match c {
            '(' => T::LeftParen, ')' => T::RightParen, '{' => T::LeftBrace, '}' => T::RightBrace,
            ',' => T::Comma, '.' => T::Dot, '-' => T::Minus, '+' => T::Plus,
            ';' => T::Semicolon, '*' => T::Star,
            '!' => if self.take('=') { T::BangEqual } else { T::Bang },
            '=' => if self.take('=') { T::EqualEqual } else { T::Equal },
            '<' => if self.take('=') { T::LessEqual } else { T::Less },
            '>' => if self.take('=') { T::GreaterEqual } else { T::Greater },
            '/' => {
                if self.take('/') {
                    while self.current < self.source.len() && self.peek() != '\n' { self.advance(); }
                    return;
                }
                if self.take('*') { self.block_comment(); return; }
                T::Slash
            }
            ' ' | '\r' | '\t' => return,
            '\n' => { self.line += 1; return; }
            '"' => { self.string(); return; }
            '0'..='9' => { self.number(); return; }
            c if c.is_ascii_alphabetic() || c == '_' => { self.identifier(); return; }
            _ => { self.error(format!("Unexpected character '{c}'.")); return; }
        };
        self.emit(token);
    }
    fn block_comment(&mut self) {
        let mut depth = 1;
        while self.current < self.source.len() {
            match self.advance() {
                '/' if self.take('*') => depth += 1,
                '*' if self.take('/') => { depth -= 1; if depth == 0 { return; } }
                '\n' => self.line += 1,
                _ => {}
            }
        }
        self.error("Unterminated block comment.");
    }
    fn number(&mut self) {
        while self.peek().is_ascii_digit() { self.advance(); }
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();
            while self.peek().is_ascii_digit() { self.advance(); }
        }
        match self.source[self.start..self.current].parse::<f64>() {
            Ok(value) if value.is_finite() => self.emit(T::Number(value)),
            _ => self.error("Number literal is out of range."),
        }
    }
    fn identifier(&mut self) {
        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' { self.advance(); }
        let name = &self.source[self.start..self.current];
        self.emit(KEYWORDS.get(name).cloned().unwrap_or_else(|| T::Identifier(name.into())));
    }
    fn string(&mut self) {
        let mut text = String::new();
        while self.current < self.source.len() {
            match self.advance() {
                '"' => { self.emit(T::String(text)); return; }
                '\n' => { self.line += 1; text.push('\n'); }
                '\\' if self.current < self.source.len() => {
                    let c = self.advance();
                    match c {
                        'n' => text.push('\n'), 'r' => text.push('\r'), 't' => text.push('\t'),
                        '"' => text.push('"'), '\\' => text.push('\\'),
                        _ => { if c == '\n' { self.line += 1; } self.error(format!("Unknown escape '\\{c}'.")); }
                    }
                }
                c => text.push(c),
            }
        }
        self.error("Unterminated string.");
    }
}
