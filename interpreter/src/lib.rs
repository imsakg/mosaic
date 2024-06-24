pub mod args;
pub mod ast;
pub mod build;
pub mod error;
pub mod gc;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod resolver;
pub mod runtime;
pub mod session;
pub mod token;
pub mod tree;
pub mod vm;

/// Scan and parse a complete source unit without executing any of it.
pub fn parse(source: &str) -> Result<Vec<ast::Stmt>, Vec<error::Diagnostic>> {
    let mut lexer = lexer::Lexer::new(source.to_owned());
    let tokens = lexer.scan_tokens();
    if !lexer.errors().is_empty() {
        return Err(lexer.errors().to_vec());
    }
    parser::Parser::new(tokens).parse()
}
