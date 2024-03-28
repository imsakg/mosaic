use interpreter::{
    ast::{ExprKind as E, StmtKind as S},
    parse,
};
#[test]
fn functions_keep_parameters_body_and_return() {
    let ss = parse("fun add(a,b) { return a+b; } print add(1,2);").unwrap();
    let S::Function(f) = &ss[0].kind else {
        panic!()
    };
    assert_eq!(f.params, ["a", "b"]);
    assert!(matches!(f.body[0].kind, S::Return(Some(_))));
    let S::Print(e) = &ss[1].kind else { panic!() };
    assert!(matches!(&e.kind,E::Call(_,args) if args.len()==2));
}
#[test]
fn chained_calls_and_property_reads_are_legal() {
    assert!(parse("factory()(1).name;").is_ok());
    assert!(parse("fun empty() { return; }").is_ok());
}
