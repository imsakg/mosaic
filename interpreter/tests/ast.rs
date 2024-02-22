use interpreter::ast::{Expr, ExprKind, Literal, Stmt, StmtKind};
#[test]
fn syntax_nodes_preserve_source_lines_and_literal_types() {
    let value = Expr {
        kind: ExprKind::Literal(Literal::Number(42.0)),
        line: 7,
    };
    let statement = Stmt {
        kind: StmtKind::Print(value.clone()),
        line: 7,
    };
    assert_eq!(statement.clone(), statement);
    assert_eq!(value.line, 7);
    assert!(format!("{statement:?}").contains("Number(42.0)"));
}
