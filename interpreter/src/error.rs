use crate::token::Token;

pub enum Error {
    RuntimeError(RuntimeError),
}

pub struct RuntimeError {
    pub token: Token,
    pub message: String,
}
#[derive(Debug)]
pub struct ErrorReporter {
    had_error: bool,
    had_runtime_error: bool,
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self {
            had_error: false,
            had_runtime_error: false,
        }
    }

    pub fn error(&mut self, line: usize, message: &str) {
        self.report(line, "", message);
    }

    fn runtime_error(&mut self, error: RuntimeError) {
        self.report(error.token.line, &error.message, &error.message);
        self.had_runtime_error = true;
    }

    fn report(&mut self, line: usize, location: &str, message: &str) {
        eprintln!("[line {}] Error{}: {}", line, location, message);
        self.had_error = true;
    }
}
