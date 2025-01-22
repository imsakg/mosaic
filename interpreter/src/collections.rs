//! Immutable persistent collections for the tree backend.
use crate::error::Diagnostic;
use crate::runtime::Value;
use std::collections::BTreeMap;

const MAX_ITEMS: usize = 65_536;
const MAX_DEPTH: usize = 64;
pub(crate) const NAMES: &[&str] = &[
    "entries", "values", "keys", "remove", "put", "has", "map", "slice", "concat", "append", "get",
    "len", "tuple",
];

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
fn key_arg(value: &Value, line: usize) -> Result<&str, Diagnostic> {
    if let Value::String(key) = value {
        Ok(key)
    } else {
        Err(Diagnostic::new(
            "Runtime",
            line,
            "Map keys must be strings.",
        ))
    }
}
fn map_value(values: BTreeMap<String, Value>, line: usize) -> Result<Value, Diagnostic> {
    if values.len() > MAX_ITEMS {
        return Err(Diagnostic::new(
            "Runtime",
            line,
            "Collection item limit exceeded.",
        ));
    }
    if values
        .values()
        .any(|value| value.collection_depth() >= MAX_DEPTH)
    {
        return Err(Diagnostic::new(
            "Runtime",
            line,
            "Collection depth limit exceeded.",
        ));
    }
    Ok(Value::Map(values.into()))
}

fn map_arg(value: &Value, line: usize) -> Result<&BTreeMap<String, Value>, Diagnostic> {
    if let Value::Map(values) = value {
        Ok(values)
    } else {
        Err(Diagnostic::new("Runtime", line, "Expected a map."))
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
                Value::Map(values) => values.len(),
                Value::String(text) => text.chars().count(),
                _ => return Err(error("len requires a string or tuple.")),
            };
            Ok(Value::Number(length as f64))
        }
        "get" => {
            arity(name, &args, 2, line)?;
            if let Value::Map(values) = &args[0] {
                return Ok(values
                    .get(key_arg(&args[1], line)?)
                    .cloned()
                    .unwrap_or(Value::Nil));
            }
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
        "concat" => {
            arity(name, &args, 2, line)?;
            let left = tuple_arg(&args[0], line)?;
            let right = tuple_arg(&args[1], line)?;
            if left.len() + right.len() > MAX_ITEMS {
                return Err(error("Collection item limit exceeded."));
            }
            tuple(left.iter().chain(right).cloned().collect(), line)
        }
        "slice" => {
            arity(name, &args, 3, line)?;
            let start = index_arg(&args[1], line)?;
            let end = index_arg(&args[2], line)?;
            if start > end {
                return Err(error("Slice start must not exceed its end."));
            }
            match &args[0] {
                Value::Tuple(values) => tuple(
                    values[(start.min(values.len() as f64) as usize)
                        ..(end.min(values.len() as f64) as usize)]
                        .to_vec(),
                    line,
                ),
                Value::String(text) => {
                    let length = text.chars().count();
                    let start = start.min(length as f64) as usize;
                    let end = end.min(length as f64) as usize;
                    Ok(Value::String(
                        text.chars()
                            .skip(start)
                            .take(end - start)
                            .collect::<String>()
                            .into(),
                    ))
                }
                _ => Err(error("slice requires a string or tuple.")),
            }
        }
        "map" => {
            let mut values = BTreeMap::new();
            for pair in args {
                let pair = tuple_arg(&pair, line)?;
                if pair.len() != 2 {
                    return Err(error("map entries must be two-element tuples."));
                }
                values.insert(key_arg(&pair[0], line)?.to_owned(), pair[1].clone());
            }
            map_value(values, line)
        }
        "has" => {
            arity(name, &args, 2, line)?;
            Ok(Value::Bool(
                map_arg(&args[0], line)?.contains_key(key_arg(&args[1], line)?),
            ))
        }
        "put" => {
            arity(name, &args, 3, line)?;
            let mut values = map_arg(&args[0], line)?.clone();
            values.insert(key_arg(&args[1], line)?.to_owned(), args[2].clone());
            map_value(values, line)
        }
        "remove" => {
            arity(name, &args, 2, line)?;
            let mut values = map_arg(&args[0], line)?.clone();
            values.remove(key_arg(&args[1], line)?);
            map_value(values, line)
        }
        "keys" => {
            arity(name, &args, 1, line)?;
            tuple(
                map_arg(&args[0], line)?
                    .keys()
                    .map(|key| Value::String(key.as_str().into()))
                    .collect(),
                line,
            )
        }
        "values" => {
            arity(name, &args, 1, line)?;
            tuple(map_arg(&args[0], line)?.values().cloned().collect(), line)
        }
        "entries" => {
            arity(name, &args, 1, line)?;
            let values = map_arg(&args[0], line)?
                .iter()
                .map(|(key, value)| {
                    tuple(
                        vec![Value::String(key.as_str().into()), value.clone()],
                        line,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            tuple(values, line)
        }
        _ => Err(error("Unknown collection function.")),
    }
}
