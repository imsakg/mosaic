use interpreter::{
    ast::{ExprKind as E, Op, StmtKind as S},
    parse,
};
#[test]
fn logical_and_binds_inside_logical_or() {
    let ss = parse("print false or true and false;").unwrap();
    let S::Print(e) = &ss[0].kind else { panic!() };
    let E::Logical(_, Op::Or, r) = &e.kind else {
        panic!()
    };
    assert!(matches!(r.kind, E::Logical(_, Op::And, _)));
}
#[test]
fn dangling_else_attaches_to_the_nearest_if() {
    let ss = parse("if (true) if (false) print 1; else print 2;").unwrap();
    let S::If(_, inner, None) = &ss[0].kind else {
        panic!()
    };
    assert!(matches!(inner.kind, S::If(_, _, Some(_))));
}
