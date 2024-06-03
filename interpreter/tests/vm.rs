mod common;
use common::*;
use interpreter::ast::*;
use interpreter::vm::{compile, disassemble, Vm};

#[test]
fn literals_and_unary_operators_print_without_host_stdout() {
    let mut vm = Vm::new();
    let program = vec![
        print(nil()),
        print(boolean(true)),
        print(boolean(false)),
        print(num(12.5)),
        print(text("hi\n🎼")),
        print(unary(Op::Negate, num(2.0))),
    ];
    assert_eq!(
        vm.run(&program).unwrap(),
        ["nil", "true", "false", "12.5", "hi\n🎼", "-2"]
    );
    assert_eq!(vm.run(&[]).unwrap(), Vec::<String>::new());
}

#[test]
fn arithmetic_and_all_comparisons_have_strict_value_semantics() {
    let program = vec![
        print(binary(num(7.0), Op::Add, num(2.0))),
        print(binary(num(7.0), Op::Subtract, num(2.0))),
        print(binary(num(7.0), Op::Multiply, num(2.0))),
        print(binary(num(7.0), Op::Divide, num(2.0))),
        print(binary(num(7.0), Op::Greater, num(2.0))),
        print(binary(num(7.0), Op::GreaterEqual, num(7.0))),
        print(binary(num(2.0), Op::Less, num(7.0))),
        print(binary(num(7.0), Op::LessEqual, num(7.0))),
        print(binary(num(2.0), Op::Equal, num(2.0))),
        print(binary(num(2.0), Op::NotEqual, num(2.0))),
        print(binary(nil(), Op::Equal, nil())),
        print(binary(boolean(true), Op::Equal, num(1.0))),
        print(binary(
            text("same"),
            Op::Equal,
            binary(text("sa"), Op::Add, text("me")),
        )),
    ];
    assert_eq!(
        Vm::new().run(&program).unwrap(),
        [
            "9", "5", "14", "3.5", "true", "true", "true", "true", "true", "false", "true",
            "false", "true"
        ]
    );
}

#[test]
fn only_nil_and_false_are_falsey() {
    let values = [nil(), boolean(false), boolean(true), num(0.0), text("")];
    let program: Vec<_> = values
        .into_iter()
        .map(|value| print(unary(Op::Not, value)))
        .collect();
    assert_eq!(
        Vm::new().run(&program).unwrap(),
        ["true", "true", "false", "false", "false"]
    );
}

#[test]
fn globals_persist_but_output_is_per_run() {
    let mut vm = Vm::new();
    assert_eq!(
        vm.run(&[var("x", Some(text("old"))), print(name("x"))])
            .unwrap(),
        ["old"]
    );
    assert_eq!(vm.run(&[print(assign("x", text("new")))]).unwrap(), ["new"]);
    assert_eq!(vm.run(&[print(name("x"))]).unwrap(), ["new"]);
    assert_eq!(
        vm.run(&[var("empty", None), print(name("empty"))]).unwrap(),
        ["nil"]
    );
}

#[test]
fn blocks_shadow_and_assign_the_nearest_binding() {
    let program = vec![
        var("a", Some(text("global"))),
        block(vec![
            var("a", Some(text("local"))),
            block(vec![expression(assign("a", text("changed")))]),
            print(name("a")),
        ]),
        print(name("a")),
        block(vec![expression(assign("a", text("outer")))]),
        print(name("a")),
    ];
    assert_eq!(
        Vm::new().run(&program).unwrap(),
        ["changed", "global", "outer"]
    );
}

#[test]
fn if_else_and_while_execute_compiled_forward_and_backward_jumps() {
    let program = vec![
        var("i", Some(num(0.0))),
        var("sum", Some(num(0.0))),
        while_(
            binary(name("i"), Op::Less, num(5.0)),
            block(vec![
                expression(assign("sum", binary(name("sum"), Op::Add, name("i")))),
                expression(assign("i", binary(name("i"), Op::Add, num(1.0)))),
            ]),
        ),
        if_(
            binary(name("sum"), Op::Equal, num(10.0)),
            print(text("yes")),
            Some(print(text("no"))),
        ),
        if_(boolean(false), print(name("must_not_execute")), None),
        if_(nil(), print(text("bad")), Some(print(name("sum")))),
    ];
    assert_eq!(Vm::new().run(&program).unwrap(), ["yes", "10"]);
    let listing = disassemble(&program).unwrap();
    assert!(listing.contains("JUMP_IF_FALSE"));
    assert!(listing.contains("JUMP"));
    assert!(listing.contains("ENTER_SCOPE"));
    assert!(listing.contains("EXIT_SCOPE"));
}

#[test]
fn logical_operators_short_circuit_and_return_the_operand() {
    let program = vec![
        var("effect", Some(num(0.0))),
        print(logical(nil(), Op::And, name("missing"))),
        print(logical(text("left"), Op::Or, name("missing"))),
        print(logical(num(0.0), Op::And, text("right"))),
        print(logical(boolean(false), Op::Or, text("fallback"))),
        expression(logical(
            boolean(false),
            Op::And,
            assign("effect", num(100.0)),
        )),
        expression(logical(boolean(true), Op::Or, assign("effect", num(100.0)))),
        print(name("effect")),
        print(logical(boolean(true), Op::And, assign("effect", num(1.0)))),
        print(logical(nil(), Op::Or, assign("effect", num(2.0)))),
    ];
    assert_eq!(
        Vm::new().run(&program).unwrap(),
        ["nil", "left", "right", "fallback", "0", "1", "2"]
    );
}

#[test]
fn runtime_errors_are_line_tagged_and_unwind_lexical_state() {
    let mut vm = Vm::new();
    vm.run(&[var("a", Some(text("global")))]).unwrap();
    let bad = Expr {
        kind: ExprKind::Binary(Box::new(num(1.0)), Op::Add, Box::new(text("wrong"))),
        line: 37,
    };
    let error = vm
        .run(&[block(vec![
            var("a", Some(text("local"))),
            var("leaked", Some(text("secret"))),
            print(bad),
        ])])
        .unwrap_err();
    assert!(error.contains("[line 37]"), "{error}");
    assert_eq!(vm.run(&[print(name("a"))]).unwrap(), ["global"]);
    assert!(vm
        .run(&[print(name("leaked"))])
        .unwrap_err()
        .contains("Undefined variable"));
    assert_eq!(vm.gc_stats().live_objects, 2);
}

#[test]
fn undefined_assignment_is_an_error_not_an_implicit_definition() {
    let mut vm = Vm::new();
    let mut assignment = assign("not_defined", text("new"));
    assignment.line = 19;
    let error = vm.run(&[expression(assignment)]).unwrap_err();
    assert!(error.contains("[line 19]"));
    assert!(error.contains("Undefined variable"));
    assert!(vm.run(&[print(name("not_defined"))]).is_err());
    assert_eq!(vm.gc_stats().live_objects, 0);
}

#[test]
fn local_self_initialization_and_duplicate_declarations_are_compile_errors() {
    let error = Vm::new()
        .run(&[block(vec![var("a", Some(name("a")))])])
        .unwrap_err();
    assert!(error.contains("initializer"), "{error}");
    let error = Vm::new()
        .run(&[block(vec![var("a", None), var("a", None)])])
        .unwrap_err();
    assert!(error.contains("already declared"), "{error}");
}

#[test]
fn unsupported_constructs_fail_during_compilation_even_in_unreachable_code() {
    let function = Function {
        name: "f".into(),
        params: vec![],
        body: vec![],
        line: 23,
    };
    let mut unsupported = vec![
        stmt(StmtKind::Function(function)),
        stmt(StmtKind::Return(None)),
        stmt(StmtKind::Class("C".into(), None, vec![])),
        expression(expr(ExprKind::Call(Box::new(name("f")), vec![]))),
        expression(expr(ExprKind::Get(Box::new(name("obj")), "field".into()))),
        expression(expr(ExprKind::Set(
            Box::new(name("obj")),
            "field".into(),
            Box::new(num(1.0)),
        ))),
        expression(expr(ExprKind::This)),
        expression(expr(ExprKind::Super("method".into()))),
    ];
    for statement in &mut unsupported {
        statement.line = 23;
    }
    for statement in unsupported {
        let mut vm = Vm::new();
        let program = vec![
            var("must_not_execute", Some(num(1.0))),
            if_(boolean(false), statement, None),
        ];
        let error = vm.run(&program).unwrap_err();
        assert!(error.contains("Compile error"), "{error}");
        assert!(error.contains("not supported"), "{error}");
        assert!(vm.run(&[print(name("must_not_execute"))]).is_err());
    }
}

#[test]
fn invalid_ast_operator_positions_are_compile_errors() {
    for invalid in [
        unary(Op::Add, num(1.0)),
        binary(num(1.0), Op::And, num(2.0)),
        logical(num(1.0), Op::Add, num(2.0)),
    ] {
        let error = compile(&[print(invalid)]).unwrap_err();
        assert!(error.contains("Compile error"));
        assert!(error.contains("operator"));
    }
}

#[test]
fn disassembly_shows_real_byte_offsets_constant_indices_and_lines() {
    let mut statement = print(binary(num(1.0), Op::Add, num(2.0)));
    statement.line = 42;
    let program = [statement];
    let chunk = compile(&program).unwrap();
    assert_eq!(chunk.code().len(), 12); // two 5-byte CONSTANTs, ADD, PRINT
    assert_eq!(chunk.constants().len(), 2);
    let listing = disassemble(&program).unwrap();
    assert!(listing.contains("0000"), "{listing}");
    assert!(listing.contains("0005"), "{listing}");
    assert!(listing.contains("0010"), "{listing}");
    assert!(listing.contains("0011   42 PRINT"), "{listing}");
    assert!(listing.contains("CONSTANT 0 Number(1.0)"), "{listing}");
    assert!(listing.contains("ADD"));
    assert_eq!(listing, chunk.disassemble().unwrap());
}

#[test]
fn incompatible_operand_types_are_runtime_errors() {
    let programs = [
        unary(Op::Negate, nil()),
        binary(text("a"), Op::Subtract, text("b")),
        binary(text("a"), Op::Greater, text("b")),
        binary(boolean(true), Op::Multiply, num(2.0)),
        binary(num(2.0), Op::Add, text("2")),
    ];
    for invalid in programs {
        let error = Vm::new().run(&[print(invalid)]).unwrap_err();
        assert!(error.contains("[line 1] Runtime error:"), "{error}");
    }
}

#[test]
fn ieee_division_and_nan_comparisons_are_not_rewritten_as_negated_orderings() {
    let nan = || binary(num(0.0), Op::Divide, num(0.0));
    let program = vec![
        print(binary(num(1.0), Op::Divide, num(0.0))),
        print(binary(nan(), Op::GreaterEqual, num(1.0))),
        print(binary(nan(), Op::LessEqual, num(1.0))),
        print(binary(nan(), Op::Equal, nan())),
        print(binary(nan(), Op::NotEqual, nan())),
    ];
    assert_eq!(
        Vm::new().run(&program).unwrap(),
        ["inf", "false", "false", "false", "true"]
    );
}
