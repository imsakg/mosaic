use interpreter::{
    ast::{ExprKind as E, StmtKind as S},
    parse,
};
#[test]
fn assignments_are_right_associative() {
    let ss = parse("a = b = 3;").unwrap();
    let S::Expression(e) = &ss[0].kind else {
        panic!()
    };
    let E::Assign(a, b) = &e.kind else { panic!() };
    assert_eq!(a, "a");
    assert!(matches!(&b.kind,E::Assign(name,_) if name=="b"));
}
#[test]
fn property_assignment_keeps_receiver_and_value() {
    let ss = parse("object.field = 7;").unwrap();
    let S::Expression(e) = &ss[0].kind else {
        panic!()
    };
    assert!(matches!(&e.kind,E::Set(_,name,_) if name=="field"));
    assert!(parse("(1 + 2) = 3;").unwrap_err()[0]
        .message
        .contains("assignment target"));
}
