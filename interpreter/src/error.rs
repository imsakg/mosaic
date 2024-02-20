//! Structured diagnostics shared by scanning, parsing and tree evaluation.
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub phase: &'static str,
    pub line: usize,
    pub message: String,
}
impl Diagnostic {
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
