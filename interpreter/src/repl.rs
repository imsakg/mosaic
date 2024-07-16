//! Portable multiline REPL. It never writes history or other working files.
use crate::{lexer::Lexer, session::Session, token::TokenType as T};
use std::io::{self, BufRead, Write};

/// Decide whether a complete lexical unit needs more input. Invalid closing
/// delimiters are submitted immediately so the parser can report the error.
pub fn needs_more(source: &str) -> bool {
    let mut lexer = Lexer::new(source.into());
    let tokens = lexer.scan_tokens();
    if lexer
        .errors()
        .iter()
        .any(|e| e.message.starts_with("Unterminated"))
    {
        return true;
    }
    if !lexer.errors().is_empty() {
        return false;
    }
    let mut parens = 0i32;
    let mut braces = 0i32;
    let mut last = None;
    for token in tokens {
        match token.token_type {
            T::LeftParen => parens += 1,
            T::RightParen => parens -= 1,
            T::LeftBrace => braces += 1,
            T::RightBrace => braces -= 1,
            T::EOF => continue,
            _ => {}
        }
        if parens < 0 || braces < 0 {
            return false;
        }
        last = Some(token.token_type);
    }
    parens > 0
        || braces > 0
        || (!source.trim_start().starts_with('=')
            && last.is_some_and(|t| !matches!(t, T::Semicolon | T::RightBrace)))
}
fn evaluate(
    session: &mut Session,
    source: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<()> {
    let transformed = source.trim_start().strip_prefix('=').map(|expression| {
        let tokens = Lexer::new(expression.into()).scan_tokens();
        let terminated = tokens
            .iter()
            .rev()
            .find(|t| t.token_type != T::EOF)
            .is_some_and(|t| t.token_type == T::Semicolon);
        if terminated {
            format!("print {expression}")
        } else {
            format!("print {expression}\n;")
        }
    });
    let result = session.run(transformed.as_deref().unwrap_or(source));
    let (output, messages) = match result {
        Ok(output) => (output, Vec::new()),
        Err(e) => (e.output, e.messages),
    };
    for line in output {
        writeln!(out, "{line}")?;
    }
    for line in messages {
        writeln!(err, "{line}")?;
    }
    Ok(())
}
fn accepts_else(source: &str) -> bool {
    use crate::ast::{Stmt, StmtKind};
    fn dangling(statement: &Stmt) -> bool {
        match &statement.kind {
            StmtKind::If(_, _, None) => true,
            StmtKind::If(_, _, Some(other)) | StmtKind::While(_, other) => dangling(other),
            _ => false,
        }
    }
    crate::parse(source)
        .ok()
        .and_then(|ss| ss.last().map(dangling))
        .unwrap_or(false)
}
pub fn run(
    mut input: impl BufRead,
    mut out: impl Write,
    mut err: impl Write,
    session: &mut Session,
    prompt: bool,
) -> io::Result<()> {
    let mut pending = String::new();
    let mut awaiting_else = false;
    loop {
        if prompt {
            write!(out, "{}", if pending.is_empty() { ">> " } else { ".. " })?;
            out.flush()?;
        }
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            if !pending.trim().is_empty() {
                evaluate(session, &pending, &mut out, &mut err)?;
            }
            return Ok(());
        }
        if awaiting_else {
            let tokens = Lexer::new(line.clone()).scan_tokens();
            let first = &tokens[0].token_type;
            if *first == T::EOF && !line.trim().is_empty() {
                pending.push_str(&line);
                continue;
            }
            awaiting_else = false;
            if *first != T::Else {
                evaluate(session, &pending, &mut out, &mut err)?;
                pending.clear();
            }
        }
        if pending.is_empty() {
            match line.trim() {
                ":quit" | ":q" => return Ok(()),
                ":help" => {
                    writeln!(
                        out,
                        ":quit — exit; :stats — heap counters; = expression — print a value"
                    )?;
                    continue;
                }
                ":stats" => {
                    writeln!(out, "{}", session.stats())?;
                    continue;
                }
                s if s.starts_with(':') => {
                    writeln!(err, "Unknown REPL command. Type :help.")?;
                    continue;
                }
                _ => {}
            }
        }
        pending.push_str(&line);
        if !needs_more(&pending) {
            if accepts_else(&pending) {
                awaiting_else = true;
                continue;
            }
            evaluate(session, &pending, &mut out, &mut err)?;
            pending.clear();
        }
    }
}
