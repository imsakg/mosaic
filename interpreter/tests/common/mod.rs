#![allow(dead_code)]
use interpreter::ast::*;

pub fn expr(kind: ExprKind) -> Expr {
    Expr { kind, line: 1 }
}
pub fn stmt(kind: StmtKind) -> Stmt {
    Stmt { kind, line: 1 }
}
pub fn num(n: f64) -> Expr {
    expr(ExprKind::Literal(Literal::Number(n)))
}
pub fn text(s: &str) -> Expr {
    expr(ExprKind::Literal(Literal::String(s.to_owned())))
}
pub fn boolean(b: bool) -> Expr {
    expr(ExprKind::Literal(Literal::Bool(b)))
}
pub fn nil() -> Expr {
    expr(ExprKind::Literal(Literal::Nil))
}
pub fn name(s: &str) -> Expr {
    expr(ExprKind::Variable(s.to_owned()))
}
pub fn assign(s: &str, value: Expr) -> Expr {
    expr(ExprKind::Assign(s.to_owned(), Box::new(value)))
}
pub fn unary(op: Op, value: Expr) -> Expr {
    expr(ExprKind::Unary(op, Box::new(value)))
}
pub fn binary(left: Expr, op: Op, right: Expr) -> Expr {
    expr(ExprKind::Binary(Box::new(left), op, Box::new(right)))
}
pub fn logical(left: Expr, op: Op, right: Expr) -> Expr {
    expr(ExprKind::Logical(Box::new(left), op, Box::new(right)))
}
pub fn print(value: Expr) -> Stmt {
    stmt(StmtKind::Print(value))
}
pub fn var(s: &str, value: Option<Expr>) -> Stmt {
    stmt(StmtKind::Var(s.to_owned(), value))
}
pub fn expression(value: Expr) -> Stmt {
    stmt(StmtKind::Expression(value))
}
pub fn block(body: Vec<Stmt>) -> Stmt {
    stmt(StmtKind::Block(body))
}
pub fn while_(condition: Expr, body: Stmt) -> Stmt {
    stmt(StmtKind::While(condition, Box::new(body)))
}
pub fn if_(condition: Expr, yes: Stmt, no: Option<Stmt>) -> Stmt {
    stmt(StmtKind::If(condition, Box::new(yes), no.map(Box::new)))
}
