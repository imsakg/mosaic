use interpreter::{
    ast::{ExprKind, Op, StmtKind},
    parse,
};
#[test]
fn multiplication_binds_more_tightly_than_addition() {
    let stmts = parse("print 1 + 2 * 3;").unwrap();
    let StmtKind::Print(e) = &stmts[0].kind else {
        panic!("print expected")
    };
    let ExprKind::Binary(_, Op::Add, rhs) = &e.kind else {
        panic!("add expected")
    };
    assert!(matches!(rhs.kind, ExprKind::Binary(_, Op::Multiply, _)));
}
#[test]
fn grouping_changes_the_tree_and_operators_keep_their_lines() {
    let stmts = parse("print (1 + 2)\n * 3;").unwrap();
    let StmtKind::Print(e) = &stmts[0].kind else {
        panic!()
    };
    let ExprKind::Binary(lhs, Op::Multiply, _) = &e.kind else {
        panic!()
    };
    assert_eq!(e.line, 2);
    assert!(matches!(lhs.kind, ExprKind::Binary(_, Op::Add, _)));
}
