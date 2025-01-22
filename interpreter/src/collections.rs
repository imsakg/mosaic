//! Immutable persistent collections for the tree backend.
use crate::error::Diagnostic;
use crate::runtime::Value;

const MAX_ITEMS: usize = 65_536;
const MAX_DEPTH: usize = 64;
pub(crate) const NAMES: &[&str] = &["len", "tuple"];

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
        _ => Err(error("Unknown collection function.")),
    }
}
