use interpreter::{args::Backend, repl, session::Session};
fn output(input: &str) -> (String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut s = Session::new(Backend::Tree, false);
    repl::run(input.as_bytes(), &mut out, &mut err, &mut s, false).unwrap();
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}
#[test]
fn shorthand_buffers_grouping_and_preserves_line_comments() {
    let (out, err) = output("= (\n 1+2\n)\n= 4 // note\n");
    assert_eq!(out, "3\n4\n");
    assert!(err.is_empty(), "{err}");
}
