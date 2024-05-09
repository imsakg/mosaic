//! Static lexical binding using alpha-renamed local identifiers.
//!
//! Globals retain their source spelling; each local receives an unlexable `@N:`
//! prefix. This gives closures static scope without pointer-keyed AST side tables.
use crate::ast::{Expr, ExprKind as E, Function, Stmt, StmtKind as S};
use crate::error::Diagnostic;
use std::collections::HashMap;

pub fn resolve(mut statements: Vec<Stmt>) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
    let mut resolver = Resolver::default();
    resolver.statements(&mut statements);
    if resolver.errors.is_empty() {
        Ok(statements)
    } else {
        Err(resolver.errors)
    }
}
#[derive(Default)]
struct Resolver {
    scopes: Vec<HashMap<String, (String, bool)>>,
    next: usize,
    function: u8,
    class: u8,
    errors: Vec<Diagnostic>,
}
impl Resolver {
    fn error(&mut self, line: usize, message: &str) {
        self.errors.push(Diagnostic::new("Resolve", line, message));
    }
    fn declare(&mut self, name: &mut String, line: usize) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                self.errors.push(Diagnostic::new(
                    "Resolve",
                    line,
                    "Already a variable with this name in this scope.",
                ));
            }
            self.next += 1;
            let key = format!("@{}:{}", self.next, name);
            scope.insert(name.clone(), (key.clone(), false));
            *name = key;
        }
    }
    fn define(&mut self, key: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            for (value, ready) in scope.values_mut() {
                if value == key {
                    *ready = true;
                }
            }
        }
    }
    fn reference(&mut self, name: &mut String, line: usize, read: bool) {
        for scope in self.scopes.iter().rev() {
            if let Some((key, ready)) = scope.get(name) {
                if read && !ready {
                    self.errors.push(Diagnostic::new(
                        "Resolve",
                        line,
                        "Cannot read local variable in its own initializer.",
                    ));
                }
                *name = key.clone();
                return;
            }
        }
    }
    fn statements(&mut self, ss: &mut [Stmt]) {
        for s in ss {
            self.statement(s);
        }
    }
    fn function(&mut self, f: &mut Function, kind: u8) {
        let previous = self.function;
        self.function = kind;
        self.scopes.push(HashMap::new());
        for param in &mut f.params {
            self.declare(param, f.line);
            self.define(param);
        }
        self.statements(&mut f.body);
        self.scopes.pop();
        self.function = previous;
    }
    fn statement(&mut self, s: &mut Stmt) {
        match &mut s.kind {
            S::Expression(e) | S::Print(e) => self.expression(e),
            S::Var(name, init) => {
                self.declare(name, s.line);
                if let Some(e) = init {
                    self.expression(e);
                }
                self.define(name);
            }
            S::Block(body) => {
                self.scopes.push(HashMap::new());
                self.statements(body);
                self.scopes.pop();
            }
            S::If(c, t, e) => {
                self.expression(c);
                self.statement(t);
                if let Some(e) = e {
                    self.statement(e);
                }
            }
            S::While(c, b) => {
                self.expression(c);
                self.statement(b);
            }
            S::Function(f) => {
                self.declare(&mut f.name, s.line);
                self.define(&f.name);
                self.function(f, 1);
            }
            S::Return(e) => {
                if self.function == 0 {
                    self.error(s.line, "Cannot return from top-level code.");
                }
                if let Some(e) = e {
                    if self.function == 3 {
                        self.error(s.line, "Cannot return a value from an initializer.");
                    }
                    self.expression(e);
                }
            }
            S::Class(name, superclass, methods) => {
                let original = name.clone();
                self.declare(name, s.line);
                self.define(name);
                let previous = self.class;
                self.class = if superclass.is_some() { 2 } else { 1 };
                if let Some(parent) = superclass {
                    if matches!(&parent.kind,E::Variable(n) if n==&original) {
                        self.error(s.line, "A class cannot inherit from itself.");
                    }
                    self.expression(parent);
                }
                for method in methods {
                    let kind = if method.name == "init" { 3 } else { 2 };
                    self.function(method, kind);
                }
                self.class = previous;
            }
        }
    }
    fn expression(&mut self, e: &mut Expr) {
        match &mut e.kind {
            E::Literal(_) => {}
            E::Variable(name) => self.reference(name, e.line, true),
            E::Assign(name, value) => {
                self.expression(value);
                self.reference(name, e.line, false);
            }
            E::Unary(_, x) => self.expression(x),
            E::Binary(a, _, b) | E::Logical(a, _, b) => {
                self.expression(a);
                self.expression(b);
            }
            E::Call(c, args) => {
                self.expression(c);
                for a in args {
                    self.expression(a);
                }
            }
            E::Get(x, _) => self.expression(x),
            E::Set(x, _, v) => {
                self.expression(x);
                self.expression(v);
            }
            E::This => {
                if self.class == 0 {
                    self.error(e.line, "Cannot use 'this' outside of a class.");
                }
            }
            E::Super(_) => match self.class {
                0 => self.error(e.line, "Cannot use 'super' outside of a class."),
                1 => self.error(e.line, "Cannot use 'super' in a class with no superclass."),
                _ => {}
            },
        }
    }
}
