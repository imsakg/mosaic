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

fn string_arg<'a>(
    name: &str,
    value: &'a Value,
    position: usize,
    line: usize,
) -> Result<&'a str, Diagnostic> {
    match value {
        Value::String(text) => Ok(text),
        _ => Err(Diagnostic::new(
            "Runtime",
            line,
            format!("{name} argument {position} must be a string."),
        )),
    }
}

// Require a complete ASCII decimal, not Rust's special NaN/inf spellings.
fn decimal_string(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let start = index;
    while matches!(bytes.get(index), Some(b'0'..=b'9')) {
        index += 1;
    }
    let mut digits = index - start;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        digits += index - start;
    }
    if digits == 0 {
        return false;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == start {
            return false;
        }
    }
    index == bytes.len()
}

pub(crate) const NAMES: &[&str] = &[
    "assert",
    "char_at",
    "repeat",
    "replace",
    "contains",
    "starts_with",
    "ends_with",
    "lower",
    "upper",
    "trim",
    "number",
    "is_nan",
    "is_finite",
    "log",
    "exp",
    "sin",
    "cos",
    "tan",
    "pow",
    "sqrt",
    "min",
    "max",
    "round",
    "floor",
    "ceil",
    "trunc",
    "clock",
    "abs",
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
        "pow" => {
            expect_arity(name, &args, 2, line)?;
            let base = number_arg(name, &args[0], 1, line)?;
            let exponent = number_arg(name, &args[1], 2, line)?;
            Ok(Value::Number(base.powf(exponent)))
        }
        "sin" | "cos" | "tan" => {
            expect_arity(name, &args, 1, line)?;
            let number = number_arg(name, &args[0], 1, line)?;
            let result = match name {
                "sin" => number.sin(),
                "cos" => number.cos(),
                _ => number.tan(),
            };
            Ok(Value::Number(result))
        }
        "log" | "exp" => {
            expect_arity(name, &args, 1, line)?;
            let number = number_arg(name, &args[0], 1, line)?;
            if name == "log" && number <= 0.0 {
                return Err(error("log requires a positive number.".into()));
            }
            Ok(Value::Number(if name == "log" {
                number.ln()
            } else {
                number.exp()
            }))
        }
        "is_nan" | "is_finite" => {
            expect_arity(name, &args, 1, line)?;
            let number = number_arg(name, &args[0], 1, line)?;
            Ok(Value::Bool(if name == "is_nan" {
                number.is_nan()
            } else {
                number.is_finite()
            }))
        }
        "number" => {
            expect_arity(name, &args, 1, line)?;
            match &args[0] {
                Value::Number(number) => Ok(Value::Number(*number)),
                Value::String(text) => {
                    let parsed = if decimal_string(text) {
                        text.parse::<f64>().ok()
                    } else {
                        None
                    };
                    match parsed.filter(|number| number.is_finite()) {
                        Some(number) => Ok(Value::Number(number)),
                        None => Err(error("number requires a finite decimal string.".into())),
                    }
                }
                _ => Err(error(
                    "number argument 1 must be a number or string.".into(),
                )),
            }
        }
        "trim" => {
            expect_arity(name, &args, 1, line)?;
            Ok(Value::String(
                string_arg(name, &args[0], 1, line)?.trim().into(),
            ))
        }
        "lower" | "upper" => {
            expect_arity(name, &args, 1, line)?;
            let text = string_arg(name, &args[0], 1, line)?;
            let result = if name == "lower" {
                text.to_lowercase()
            } else {
                text.to_uppercase()
            };
            Ok(Value::String(result.into()))
        }
        "starts_with" | "ends_with" | "contains" => {
            expect_arity(name, &args, 2, line)?;
            let text = string_arg(name, &args[0], 1, line)?;
            let pattern = string_arg(name, &args[1], 2, line)?;
            Ok(Value::Bool(match name {
                "starts_with" => text.starts_with(pattern),
                "ends_with" => text.ends_with(pattern),
                _ => text.contains(pattern),
            }))
        }
        "replace" => {
            expect_arity(name, &args, 3, line)?;
            let text = string_arg(name, &args[0], 1, line)?;
            let pattern = string_arg(name, &args[1], 2, line)?;
            let replacement = string_arg(name, &args[2], 3, line)?;
            Ok(Value::String(text.replace(pattern, replacement).into()))
        }
        "repeat" => {
            const MAX_BYTES: usize = 1_048_576;
            expect_arity(name, &args, 2, line)?;
            let text = string_arg(name, &args[0], 1, line)?;
            let count = number_arg(name, &args[1], 2, line)?;
            if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
                return Err(error(
                    "repeat count must be a finite nonnegative integer.".into(),
                ));
            }
            // Empty strings need no allocation, even for counts beyond usize.
            if text.is_empty() || count == 0.0 {
                return Ok(Value::String("".into()));
            }
            // Divide before converting/multiplying so neither casts nor byte sizes overflow.
            if count > (MAX_BYTES / text.len()) as f64 {
                return Err(error(format!(
                    "repeat result exceeds {MAX_BYTES} UTF-8 bytes."
                )));
            }
            Ok(Value::String(text.repeat(count as usize).into()))
        }
        "char_at" => {
            expect_arity(name, &args, 2, line)?;
            let text = string_arg(name, &args[0], 1, line)?;
            let index = number_arg(name, &args[1], 2, line)?;
            if !index.is_finite() || index < 0.0 || index.fract() != 0.0 {
                return Err(error(
                    "char_at index must be a finite nonnegative integer.".into(),
                ));
            }
            // Scalar counts cannot exceed byte counts. Reject huge indices before casting.
            if index >= text.len() as f64 {
                return Ok(Value::Nil);
            }
            Ok(match text.chars().nth(index as usize) {
                Some(character) => Value::String(character.to_string().into()),
                None => Value::Nil,
            })
        }
        "assert" => {
            if !(1..=2).contains(&args.len()) {
                return Err(error(format!(
                    "assert: Expected 1 or 2 arguments but got {}.",
                    args.len()
                )));
            }
            let message = if args.len() == 2 {
                Some(string_arg(name, &args[1], 2, line)?)
            } else {
                None
            };
            if args[0].truthy() {
                Ok(Value::Nil)
            } else {
                Err(error(match message {
                    Some(message) => format!("Assertion failed: {message}"),
                    None => "Assertion failed.".into(),
                }))
            }
        }
        _ => Err(error(format!("Unknown native function '{name}'."))),
    }
}
