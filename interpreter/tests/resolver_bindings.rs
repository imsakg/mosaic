use interpreter::{parse, resolver::resolve};
#[test]
fn resolver_rejects_local_self_initialization_and_duplicates() {
    for source in ["{ var a=a; }", "{ var a; var a; }"] {
        assert!(resolve(parse(source).unwrap()).is_err());
    }
}
#[test]
fn lexical_keys_bind_to_the_declaration_visible_at_definition() {
    let ss = resolve(parse("var a=1; { fun show() { print a; } var a=2; }").unwrap()).unwrap();
    let printed = format!("{ss:?}");
    assert!(printed.contains("Variable(\"a\")"));
    assert!(printed.contains("@"));
}
