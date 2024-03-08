use interpreter::{
    ast::{ExprKind as E, Literal, StmtKind as S},
    parse,
};
#[test]
fn for_desugaring_keeps_initializer_condition_and_increment_in_order() {
    let ss = parse("for (var i=0; i<3; i=i+1) print i;").unwrap();
    let S::Block(parts) = &ss[0].kind else {
        panic!()
    };
    assert!(matches!(parts[0].kind, S::Var(..)));
    let S::While(_, body) = &parts[1].kind else {
        panic!()
    };
    let S::Block(parts) = &body.kind else {
        panic!()
    };
    assert!(matches!(parts[0].kind, S::Print(_)));
    assert!(matches!(parts[1].kind, S::Expression(_)));
}
#[test]
fn omitted_for_condition_becomes_true() {
    let ss = parse("for (;;) print 1;").unwrap();
    let S::While(condition, _) = &ss[0].kind else {
        panic!()
    };
    assert_eq!(condition.kind, E::Literal(Literal::Bool(true)));
}
