//! Native standard-library functions shared through named callable values.
use crate::error::Diagnostic;
use crate::runtime::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const NAMES: &[&str] = &["clock"];

pub(crate) fn call(name: &str, args: Vec<Value>, line: usize) -> Result<Value, Diagnostic> {
    let error = |message: String| Diagnostic::new("Runtime", line, message);
    match name {
        "clock" => {
            if !args.is_empty() {
                return Err(error(format!(
                    "Expected 0 arguments but got {}.",
                    args.len()
                )));
            }
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| error("System clock is before Unix epoch.".into()))?
                .as_secs_f64();
            Ok(Value::Number(seconds))
        }
        _ => Err(error(format!("Unknown native function '{name}'."))),
    }
}
