//! Tree-walk evaluator with lexical closures, classes and boundary tracing GC.
use crate::ast::{Expr, ExprKind as E, Literal, Op, Stmt, StmtKind as S};
use crate::error::Diagnostic;
use crate::runtime::{Heap, HeapStats, Id, Object, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Diagnostic>;
#[derive(Debug)]
enum Flow {
    Normal,
    Return(Value),
}

pub struct Interpreter {
    heap: Heap,
    environment: Id,
    output: Vec<String>,
    call_depth: usize,
    evaluation_depth: usize,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
impl Interpreter {
    pub fn new() -> Self {
        let mut heap = Heap::default();
        let environment = heap.allocate(Object::Environment {
            parent: None,
            values: HashMap::from([("clock".into(), Value::Clock)]),
        });
        Self {
            heap,
            environment,
            output: Vec::new(),
            call_depth: 0,
            evaluation_depth: 0,
        }
    }
    /// Compile an entire source unit first. Globals survive both success and errors.
    pub fn run(&mut self, source: &str) -> std::result::Result<Vec<String>, Vec<Diagnostic>> {
        self.output.clear();
        let statements = crate::resolver::resolve(crate::parse(source)?)?;
        let result = self.execute_all(&statements);
        self.environment = 0;
        self.call_depth = 0;
        self.heap.collect([0]);
        match result {
            Ok(_) => Ok(self.output.clone()),
            Err(e) => Err(vec![e]),
        }
    }
    /// Output emitted before a runtime error is retained until the next run.
    pub fn output(&self) -> &[String] {
        &self.output
    }
    pub fn heap_stats(&self) -> HeapStats {
        self.heap.stats()
    }
    fn error(&self, line: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new("Runtime", line, message)
    }
    fn define(&mut self, env: Id, name: String, value: Value) {
        let Object::Environment { values, .. } = self.heap.get_mut(env) else {
            unreachable!()
        };
        values.insert(name, value);
    }
    fn environment(&mut self, parent: Id) -> Id {
        self.heap.allocate(Object::Environment {
            parent: Some(parent),
            values: HashMap::new(),
        })
    }
    fn lookup(&self, mut env: Id, name: &str, line: usize) -> Result<Value> {
        loop {
            let Object::Environment { parent, values } = self.heap.get(env) else {
                unreachable!()
            };
            if let Some(v) = values.get(name) {
                return Ok(v.clone());
            }
            match parent {
                Some(p) => env = *p,
                None => break,
            }
        }
        Err(self.error(
            line,
            format!("Undefined variable '{}'.", display_name(name)),
        ))
    }
    fn assign(&mut self, mut env: Id, name: &str, value: Value, line: usize) -> Result<()> {
        loop {
            let Object::Environment { parent, values } = self.heap.get_mut(env) else {
                unreachable!()
            };
            if let Some(v) = values.get_mut(name) {
                *v = value;
                return Ok(());
            }
            match parent {
                Some(p) => env = *p,
                None => break,
            }
        }
        Err(self.error(
            line,
            format!("Undefined variable '{}'.", display_name(name)),
        ))
    }
    fn execute_all(&mut self, ss: &[Stmt]) -> Result<Flow> {
        for s in ss {
            if let Flow::Return(value) = self.execute(s)? {
                return Ok(Flow::Return(value));
            }
        }
        Ok(Flow::Normal)
    }
    fn in_environment(&mut self, ss: &[Stmt], env: Id) -> Result<Flow> {
        let previous = self.environment;
        self.environment = env;
        let result = self.execute_all(ss);
        self.environment = previous;
        result
    }
    fn execute(&mut self, s: &Stmt) -> Result<Flow> {
        match &s.kind {
            S::Expression(e) => {
                self.evaluate(e)?;
            }
            S::Print(e) => {
                let v = self.evaluate(e)?;
                self.output.push(self.display(&v));
            }
            S::Var(name, init) => {
                let value = match init {
                    Some(e) => self.evaluate(e)?,
                    None => Value::Nil,
                };
                self.define(self.environment, name.clone(), value);
            }
            S::Block(body) => {
                let env = self.environment(self.environment);
                return self.in_environment(body, env);
            }
            S::If(c, t, e) => {
                if self.evaluate(c)?.truthy() {
                    return self.execute(t);
                }
                if let Some(e) = e {
                    return self.execute(e);
                }
            }
            S::While(c, b) => {
                while self.evaluate(c)?.truthy() {
                    if let Flow::Return(v) = self.execute(b)? {
                        return Ok(Flow::Return(v));
                    }
                }
            }
            S::Function(f) => {
                let id = self.heap.allocate(Object::Function {
                    declaration: f.clone(),
                    closure: self.environment,
                    initializer: false,
                });
                self.define(self.environment, f.name.clone(), Value::Object(id));
            }
            S::Return(e) => {
                return Ok(Flow::Return(match e {
                    Some(e) => self.evaluate(e)?,
                    None => Value::Nil,
                }))
            }
            S::Class(name, superclass, methods) => {
                let superclass = match superclass {
                    Some(e) => match self.evaluate(e)? {
                        Value::Object(id) if matches!(self.heap.get(id), Object::Class { .. }) => {
                            Some(id)
                        }
                        _ => return Err(self.error(s.line, "Superclass must be a class.")),
                    },
                    None => None,
                };
                self.define(self.environment, name.clone(), Value::Nil);
                let closure = if let Some(parent) = superclass {
                    let env = self.environment(self.environment);
                    self.define(env, "super".into(), Value::Object(parent));
                    env
                } else {
                    self.environment
                };
                let mut table = HashMap::new();
                for method in methods {
                    let function = self.heap.allocate(Object::Function {
                        declaration: method.clone(),
                        closure,
                        initializer: method.name == "init",
                    });
                    table.insert(method.name.clone(), function);
                }
                let class = self.heap.allocate(Object::Class {
                    name: display_name(name).into(),
                    superclass,
                    methods: table,
                });
                self.assign(self.environment, name, Value::Object(class), s.line)?;
            }
        }
        Ok(Flow::Normal)
    }
    fn evaluate(&mut self, e: &Expr) -> Result<Value> {
        if self.evaluation_depth >= 128 {
            return Err(self.error(e.line, "Evaluation depth limit exceeded."));
        }
        self.evaluation_depth += 1;
        let result = self.evaluate_inner(e);
        self.evaluation_depth -= 1;
        result
    }
    fn evaluate_inner(&mut self, e: &Expr) -> Result<Value> {
        let line = e.line;
        match &e.kind {
            E::Literal(l) => Ok(match l {
                Literal::Nil => Value::Nil,
                Literal::Bool(b) => Value::Bool(*b),
                Literal::Number(n) => Value::Number(*n),
                Literal::String(s) => Value::String(s.as_str().into()),
            }),
            E::Variable(name) => self.lookup(self.environment, name, line),
            E::Assign(name, expr) => {
                let value = self.evaluate(expr)?;
                self.assign(self.environment, name, value.clone(), line)?;
                Ok(value)
            }
            E::Unary(op, expr) => {
                let v = self.evaluate(expr)?;
                match (op, v) {
                    (Op::Not, v) => Ok(Value::Bool(!v.truthy())),
                    (Op::Negate, Value::Number(n)) => Ok(Value::Number(-n)),
                    _ => Err(self.error(line, "Operand must be a number.")),
                }
            }
            E::Binary(a, op, b) => {
                let a = self.evaluate(a)?;
                let b = self.evaluate(b)?;
                self.binary(a, *op, b, line)
            }
            E::Logical(a, op, b) => {
                let a = self.evaluate(a)?;
                if (*op == Op::Or && a.truthy()) || (*op == Op::And && !a.truthy()) {
                    Ok(a)
                } else {
                    self.evaluate(b)
                }
            }
            E::Call(c, args) => {
                let callee = self.evaluate(c)?;
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.evaluate(a)?);
                }
                self.call(callee, values, line)
            }
            E::Get(object, name) => {
                let object = self.evaluate(object)?;
                self.property(object, name, line)
            }
            E::Set(object, name, value) => {
                let object = self.evaluate(object)?;
                let Value::Object(id) = object else {
                    return Err(self.error(line, "Only instances have fields."));
                };
                if !matches!(self.heap.get(id), Object::Instance { .. }) {
                    return Err(self.error(line, "Only instances have fields."));
                }
                let v = self.evaluate(value)?;
                let Object::Instance { fields, .. } = self.heap.get_mut(id) else {
                    unreachable!()
                };
                fields.insert(name.clone(), v.clone());
                Ok(v)
            }
            E::This => self.lookup(self.environment, "this", line),
            E::Super(name) => {
                let Value::Object(class) = self.lookup(self.environment, "super", line)? else {
                    unreachable!()
                };
                let Value::Object(instance) = self.lookup(self.environment, "this", line)? else {
                    unreachable!()
                };
                match self.find_method(class, name) {
                    Some(function) => Ok(Value::Object(
                        self.heap.allocate(Object::Bound { function, instance }),
                    )),
                    None => Err(self.error(line, format!("Undefined property '{name}'."))),
                }
            }
        }
    }
    fn binary(&self, a: Value, op: Op, b: Value, line: usize) -> Result<Value> {
        match op {
            Op::Equal => return Ok(Value::Bool(a == b)),
            Op::NotEqual => return Ok(Value::Bool(a != b)),
            Op::Add => {
                if let (Value::String(a), Value::String(b)) = (&a, &b) {
                    return Ok(Value::String(format!("{a}{b}").into()));
                }
            }
            _ => {}
        }
        if let (Value::Number(a), Value::Number(b)) = (&a, &b) {
            return Ok(match op {
                Op::Add => Value::Number(a + b),
                Op::Subtract => Value::Number(a - b),
                Op::Multiply => Value::Number(a * b),
                Op::Divide => Value::Number(a / b),
                Op::Greater => Value::Bool(a > b),
                Op::GreaterEqual => Value::Bool(a >= b),
                Op::Less => Value::Bool(a < b),
                Op::LessEqual => Value::Bool(a <= b),
                _ => return Err(self.error(line, "Invalid binary operator.")),
            });
        }
        Err(self.error(
            line,
            if op == Op::Add {
                "Operands must be two numbers or two strings."
            } else {
                "Operands must be numbers."
            },
        ))
    }
    fn find_method(&self, mut class: Id, name: &str) -> Option<Id> {
        loop {
            let Object::Class {
                superclass,
                methods,
                ..
            } = self.heap.get(class)
            else {
                unreachable!()
            };
            if let Some(id) = methods.get(name) {
                return Some(*id);
            }
            class = (*superclass)?;
        }
    }
    fn property(&mut self, value: Value, name: &str, line: usize) -> Result<Value> {
        if let Value::Object(instance) = value {
            if let Object::Instance { class, fields } = self.heap.get(instance) {
                if let Some(value) = fields.get(name) {
                    return Ok(value.clone());
                }
                if let Some(function) = self.find_method(*class, name) {
                    return Ok(Value::Object(
                        self.heap.allocate(Object::Bound { function, instance }),
                    ));
                }
                return Err(self.error(line, format!("Undefined property '{name}'.")));
            }
        }
        Err(self.error(line, "Only instances have properties."))
    }
    fn call(&mut self, value: Value, args: Vec<Value>, line: usize) -> Result<Value> {
        if value == Value::Clock {
            self.arity(0, args.len(), line)?;
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| self.error(line, "System clock is before Unix epoch."))?
                .as_secs_f64();
            return Ok(Value::Number(seconds));
        }
        if let Value::Object(id) = value {
            match self.heap.get(id).clone() {
                Object::Function { .. } => return self.call_function(id, args, None, line),
                Object::Bound { function, instance } => {
                    return self.call_function(function, args, Some(instance), line)
                }
                Object::Class { .. } => {
                    let method = self.find_method(id, "init");
                    if method.is_none() {
                        self.arity(0, args.len(), line)?;
                    }
                    let instance = self.heap.allocate(Object::Instance {
                        class: id,
                        fields: HashMap::new(),
                    });
                    if let Some(method) = method {
                        self.call_function(method, args, Some(instance), line)?;
                    }
                    return Ok(Value::Object(instance));
                }
                _ => {}
            }
        }
        Err(self.error(line, "Can only call functions and classes."))
    }
    fn arity(&self, expected: usize, actual: usize, line: usize) -> Result<()> {
        if expected == actual {
            Ok(())
        } else {
            Err(self.error(
                line,
                format!("Expected {expected} arguments but got {actual}."),
            ))
        }
    }
    fn call_function(
        &mut self,
        id: Id,
        args: Vec<Value>,
        instance: Option<Id>,
        line: usize,
    ) -> Result<Value> {
        let Object::Function {
            declaration,
            closure,
            initializer,
        } = self.heap.get(id).clone()
        else {
            unreachable!()
        };
        self.arity(declaration.params.len(), args.len(), line)?;
        if self.call_depth >= 64 {
            return Err(self.error(line, "Call depth limit exceeded."));
        }
        let parent = if let Some(instance) = instance {
            let bound = self.environment(closure);
            self.define(bound, "this".into(), Value::Object(instance));
            bound
        } else {
            closure
        };
        let env = self.environment(parent);
        for (name, value) in declaration.params.iter().zip(args) {
            self.define(env, name.clone(), value);
        }
        self.call_depth += 1;
        let result = self.in_environment(&declaration.body, env);
        self.call_depth -= 1;
        let flow = result?;
        if initializer {
            return self.lookup(parent, "this", line);
        }
        Ok(match flow {
            Flow::Return(v) => v,
            Flow::Normal => Value::Nil,
        })
    }
    fn display(&self, value: &Value) -> String {
        match value {
            Value::Nil => "nil".into(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.to_string(),
            Value::Clock => "<native fn>".into(),
            Value::Object(id) => match self.heap.get(*id) {
                Object::Function { declaration, .. } => {
                    format!("<fn {}>", display_name(&declaration.name))
                }
                Object::Class { name, .. } => name.clone(),
                Object::Instance { class, .. } => {
                    if let Object::Class { name, .. } = self.heap.get(*class) {
                        format!("{name} instance")
                    } else {
                        unreachable!()
                    }
                }
                Object::Bound { function, .. } => self.display(&Value::Object(*function)),
                Object::Environment { .. } => "<environment>".into(),
            },
        }
    }
}
fn display_name(name: &str) -> &str {
    name.split_once(':').map_or(name, |(_, name)| name)
}
