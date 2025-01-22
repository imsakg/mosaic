//! Immutable persistent collections for the tree backend.
use crate::error::Diagnostic;
use crate::runtime::Value;

const MAX_ITEMS: usize = 65_536;
const MAX_DEPTH: usize = 64;
pub(crate) const NAMES: &[&str] = &["append", "get", "len", "tuple"];

fn tuple(values: Vec<Value>, line: usize) -> Result<Value, Diagnostic> {
    if values.len() > MAX_ITEMS {
        return Err(Diagnostic::new(
            "Runtime",
            line,
            "Collection item limit exceeded.",
        ));
    }
    if values
        .iter()
        .any(|value| value.collection_depth() >= MAX_DEPTH)
    {
        return Err(Diagnostic::new(
            "Runtime",
            line,
            "Collection depth limit exceeded.",
        ));
    }
    Ok(Value::Tuple(values.into()))
}

fn arity(name: &str, args: &[Value], expected: usize, line: usize) -> Result<(), Diagnostic> {
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

fn tuple_arg(value: &Value, line: usize) -> Result<&[Value], Diagnostic> {
    if let Value::Tuple(values) = value {
        Ok(values)
    } else {
        Err(Diagnostic::new("Runtime", line, "Expected a tuple."))
    }
}
fn index_arg(value: &Value, line: usize) -> Result<f64, Diagnostic> {
    if let Value::Number(n) = value {
        if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 {
            return Ok(*n);
        }
    }
    Err(Diagnostic::new(
        "Runtime",
        line,
        "Index must be a finite nonnegative integer.",
    ))
}
pub(crate) fn call(name: &str, args: Vec<Value>, line: usize) -> Result<Value, Diagnostic> {
    let error = |message: &str| Diagnostic::new("Runtime", line, message);
    match name {
        "tuple" => tuple(args, line),
        "len" => {
            arity(name, &args, 1, line)?;
            let length = match &args[0] {
                Value::Tuple(values) => values.len(),
                Value::String(text) => text.chars().count(),
                _ => return Err(error("len requires a string or tuple.")),
            };
            Ok(Value::Number(length as f64))
        }
        "get" => {
            arity(name, &args, 2, line)?;
            let values = tuple_arg(&args[0], line)?;
            let index = index_arg(&args[1], line)?;
            Ok(if index < values.len() as f64 {
                values[index as usize].clone()
            } else {
                Value::Nil
            })
        }
        "append" => {
            arity(name, &args, 2, line)?;
            let mut values = tuple_arg(&args[0], line)?.to_vec();
            values.push(args[1].clone());
            tuple(values, line)
        }
        _ => Err(error("Unknown collection function.")),
    }
}
