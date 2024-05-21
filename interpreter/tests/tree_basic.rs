use interpreter::tree::Interpreter;
#[test]
fn tree_interpreter_evaluates_values_variables_and_blocks() {
    let mut i = Interpreter::new();
    assert_eq!(
        i.run("var a=2; { var a=3; print a*4; } print a;").unwrap(),
        ["12", "2"]
    );
    assert_eq!(i.run("print a+1;").unwrap(), ["3"]);
}
#[test]
fn lexical_closures_do_not_see_later_shadowing_declarations() {
    let mut i = Interpreter::new();
    assert_eq!(
        i.run("var a=\"global\"; { fun show(){print a;} show(); var a=\"block\"; show(); }")
            .unwrap(),
        ["global", "global"]
    );
}
