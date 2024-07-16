use interpreter::{args::Backend, repl, session::Session};
#[test]
fn multiline_else_can_follow_a_closing_brace_on_the_next_line() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut s = Session::new(Backend::Tree, false);
    let source = "if (false) {\n print \"bad\";\n}\nelse {\n print \"good\";\n}\nprint 2;\n";
    repl::run(source.as_bytes(), &mut out, &mut err, &mut s, false).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "good\n2\n");
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
}
