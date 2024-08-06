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

pub(crate) const NAMES: &[&str] = &[
    "sqrt", "min", "max", "round", "floor", "ceil", "trunc", "clock", "abs",
];

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
        "min" | "max" => {
            expect_arity(name, &args, 2, line)?;
            let left = number_arg(name, &args[0], 1, line)?;
            let right = number_arg(name, &args[1], 2, line)?;
            let result = if left.is_nan() || right.is_nan() {
                f64::NAN
            } else if left == 0.0 && right == 0.0 {
                let negative = if name == "min" {
                    left.is_sign_negative() || right.is_sign_negative()
                } else {
                    left.is_sign_negative() && right.is_sign_negative()
                };
                if negative {
                    -0.0
                } else {
                    0.0
                }
            } else if name == "min" {
                left.min(right)
            } else {
                left.max(right)
            };
            Ok(Value::Number(result))
        }
        "sqrt" => {
            expect_arity(name, &args, 1, line)?;
            let number = number_arg(name, &args[0], 1, line)?;
            if number < 0.0 {
                return Err(error("sqrt requires a nonnegative number.".into()));
            }
            Ok(Value::Number(number.sqrt()))
        }
        _ => Err(error(format!("Unknown native function '{name}'."))),
    }
}
