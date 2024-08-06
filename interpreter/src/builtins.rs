//! Native standard-library functions shared through named callable values.
use crate::error::Diagnostic;
use crate::runtime::Value;
use std::time::{SystemTime, UNIX_EPOCH};

fn expect_arity(
    name: &str,
    args: &[Value],
    expected: usize,
    line: usize,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            "Runtime",
            line,
            format!(
                "{name}: Expected {expected} arguments but got {}.",
                args.len()
            ),
        ))
    }
}

fn number_arg(name: &str, value: &Value, position: usize, line: usize) -> Result<f64, Diagnostic> {
    match value {
        Value::Number(number) => Ok(*number),
        _ => Err(Diagnostic::new(
            "Runtime",
            line,
            format!("{name} argument {position} must be a number."),
        )),
    }
}

pub(crate) const NAMES: &[&str] = &["round", "floor", "ceil", "trunc", "clock", "abs"];

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
        "abs" => {
            expect_arity(name, &args, 1, line)?;
            Ok(Value::Number(number_arg(name, &args[0], 1, line)?.abs()))
        }
        "floor" | "ceil" | "trunc" => {
            expect_arity(name, &args, 1, line)?;
            let number = number_arg(name, &args[0], 1, line)?;
            let result = match name {
                "floor" => number.floor(),
                "ceil" => number.ceil(),
                _ => number.trunc(),
            };
            Ok(Value::Number(result))
        }
        "round" => {
            expect_arity(name, &args, 1, line)?;
            Ok(Value::Number(number_arg(name, &args[0], 1, line)?.round()))
        }
        _ => Err(error(format!("Unknown native function '{name}'."))),
    }
}
