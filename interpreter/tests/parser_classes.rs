use interpreter::{
    ast::{ExprKind as E, StmtKind as S},
    parse,
};
#[test]
fn classes_parse_inheritance_initializers_this_and_super() {
    let ss = parse("class Child < Parent { init(x) { this.x=x; } get() { return super.get(); } }")
        .unwrap();
    let S::Class(name, Some(parent), methods) = &ss[0].kind else {
        panic!()
    };
    assert_eq!(name, "Child");
    assert!(matches!(&parent.kind,E::Variable(n) if n=="Parent"));
    assert_eq!(methods.len(), 2);
    assert_eq!(methods[0].name, "init");
    assert_eq!(methods[1].name, "get");
}
#[test]
fn method_declarations_do_not_need_fun_keyword() {
    assert!(parse("class C { fun f() {} }").is_err());
    assert!(parse("class C { f() {} }").is_ok());
}
