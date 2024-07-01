use interpreter::{parse, resolver::resolve, tree::Interpreter};
#[test]
fn assigning_a_local_inside_its_initializer_fails_before_execution() {
    let source = "print \"not run\"; { var a = (a=1); }";
    assert!(resolve(parse(source).unwrap()).is_err());
    let mut i = Interpreter::new();
    assert_eq!(i.run(source).unwrap_err()[0].phase, "Resolve");
    assert!(i.output().is_empty());
}
