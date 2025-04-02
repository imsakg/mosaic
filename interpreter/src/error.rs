//! Structured diagnostics shared by scanning, parsing and tree evaluation.
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub phase: &'static str,
    pub line: usize,
    pub message: String,
}
impl Diagnostic {
    /// Render a bounded, plain-text source excerpt without changing stable Display.
    pub fn render(&self, source: &str) -> String {
        let text = self
            .line
            .checked_sub(1)
            .and_then(|line| source.split('\n').nth(line));
        let Some(text) = text else {
            return self.to_string();
        };
        let mut chars = text.trim_end_matches('\r').chars();
        let mut excerpt: String = chars
            .by_ref()
            .take(200)
            .map(|c| {
                if c == '\t' {
                    ' '
                } else if c.is_control() {
                    '�'
                } else {
                    c
                }
            })
            .collect();
        if chars.next().is_some() {
            excerpt.push('…');
        }
        format!("{self}\n{:>4} | {excerpt}", self.line)
    }
    /// Attach an innermost-first call frame, capping recursive diagnostics.
    pub fn with_frame(mut self, function: &str, line: usize) -> Self {
        if self
            .message
            .lines()
            .filter(|line| line.starts_with("  at "))
            .count()
            < 16
        {
            self.message
                .push_str(&format!("\n  at {function} (line {line})"));
        } else if !self.message.contains("additional frames omitted") {
            self.message.push_str("\n  … additional frames omitted");
        }
        self
    }
    pub fn new(phase: &'static str, line: usize, message: impl Into<String>) -> Self {
        Self {
            phase,
            line,
            message: message.into(),
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[line {}] {} error: {}",
            self.line, self.phase, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}
