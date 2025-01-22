//! Immutable persistent collections for the tree backend.
use crate::error::Diagnostic;
use crate::runtime::Value;

const MAX_ITEMS: usize = 65_536;
const MAX_DEPTH: usize = 64;
pub(crate) const NAMES: &[&str] = &["tuple"];

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

pub(crate) fn call(name: &str, args: Vec<Value>, line: usize) -> Result<Value, Diagnostic> {
    let error = |message: &str| Diagnostic::new("Runtime", line, message);
    match name {
        "tuple" => tuple(args, line),
        _ => Err(error("Unknown collection function.")),
    }
}
