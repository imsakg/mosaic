mod common;
use common::*;
use interpreter::ast::*;
use interpreter::vm::Vm;

#[test]
fn binary_and_assignment_operands_are_evaluated_left_to_right() {
    let program = [
        var("a", Some(num(0.0))),
        print(binary(
            assign("a", num(1.0)),
            Op::Add,
            assign("a", num(2.0)),
        )),
        print(name("a")),
        print(assign("a", assign("a", num(3.0)))),
        print(name("a")),
    ];
    assert_eq!(Vm::new().run(&program).unwrap(), ["3", "2", "3", "3"]);
}

#[test]
fn numeric_opcode_results_match_ieee_operations_including_special_values() {
    let values = [
        f64::NEG_INFINITY,
        -7.0,
        -0.0,
        0.0,
        2.5,
        f64::INFINITY,
        f64::NAN,
    ];
    let mut program = Vec::new();
    let mut expected = Vec::new();
    for left in values {
        for right in values {
            for (op, result) in [
                (Op::Add, (left + right).to_string()),
                (Op::Subtract, (left - right).to_string()),
                (Op::Multiply, (left * right).to_string()),
                (Op::Divide, (left / right).to_string()),
                (Op::Equal, (left == right).to_string()),
                (Op::NotEqual, (left != right).to_string()),
                (Op::Greater, (left > right).to_string()),
                (Op::GreaterEqual, (left >= right).to_string()),
                (Op::Less, (left < right).to_string()),
                (Op::LessEqual, (left <= right).to_string()),
            ] {
                program.push(print(binary(num(left), op, num(right))));
                expected.push(result);
            }
        }
    }
    assert_eq!(Vm::new().run(&program).unwrap(), expected);
}

#[test]
fn global_redeclaration_reads_the_previous_value_but_local_shadowing_is_distinct() {
    let mut vm = Vm::new();
    vm.set_gc_stress(true);
    let program = [
        var("a", Some(text("old"))),
        var("a", Some(binary(name("a"), Op::Add, text("-new")))),
        block(vec![var("a", Some(text("shadow"))), print(name("a"))]),
        print(name("a")),
    ];
    assert_eq!(vm.run(&program).unwrap(), ["shadow", "old-new"]);
    assert_eq!(vm.gc_stats().live_objects, 2);
}

#[test]
fn global_self_initialization_without_an_existing_binding_is_a_runtime_error() {
    let mut vm = Vm::new();
    assert!(vm
        .run(&[var("a", Some(name("a")))])
        .unwrap_err()
        .contains("Undefined variable"));
    assert_eq!(vm.gc_stats().live_objects, 0);
}

#[test]
fn assignment_to_a_local_inside_its_own_initializer_is_rejected() {
    let error = Vm::new()
        .run(&[block(vec![var("a", Some(assign("a", num(1.0))))])])
        .unwrap_err();
    assert!(error.contains("Compile error"));
    assert!(error.contains("initializer"));
}

#[test]
fn compilation_failure_preserves_prior_globals_and_does_not_apply_new_statements() {
    let mut vm = Vm::new();
    vm.set_gc_stress(true);
    vm.run(&[var("saved", Some(text("before")))]).unwrap();
    assert!(vm
        .run(&[
            expression(assign("saved", text("would-change"))),
            stmt(StmtKind::Return(None)),
        ])
        .is_err());
    assert_eq!(vm.run(&[print(name("saved"))]).unwrap(), ["before"]);
    assert_eq!(vm.gc_stats().live_objects, 2);
}

#[test]
fn prior_prints_in_a_failed_run_do_not_leak_into_the_next_result() {
    let mut vm = Vm::new();
    assert!(vm
        .run(&[print(text("discarded")), print(name("missing"))])
        .is_err());
    assert!(vm.run(&[]).unwrap().is_empty());
    assert_eq!(vm.run(&[print(text("only-this"))]).unwrap(), ["only-this"]);
    assert_eq!(vm.gc_stats().live_objects, 0);
}

#[test]
fn nested_loops_and_conditionals_balance_scope_and_operand_stacks() {
    let program = [block(vec![
        var("i", Some(num(0.0))),
        var("out", Some(text(""))),
        while_(
            binary(name("i"), Op::Less, num(3.0)),
            block(vec![
                var("j", Some(num(0.0))),
                while_(
                    binary(name("j"), Op::Less, num(2.0)),
                    block(vec![
                        if_(
                            binary(name("j"), Op::Equal, num(0.0)),
                            expression(assign("out", binary(name("out"), Op::Add, text("a")))),
                            Some(expression(assign(
                                "out",
                                binary(name("out"), Op::Add, text("b")),
                            ))),
                        ),
                        expression(assign("j", binary(name("j"), Op::Add, num(1.0)))),
                    ]),
                ),
                expression(assign("i", binary(name("i"), Op::Add, num(1.0)))),
            ]),
        ),
        print(name("out")),
    ])];
    let mut vm = Vm::with_gc_threshold(1);
    vm.set_gc_stress(true);
    assert_eq!(vm.run(&program).unwrap(), ["ababab"]);
    assert_eq!(vm.gc_stats().live_objects, 0);
}

#[test]
fn zero_gc_threshold_is_clamped_to_a_working_minimum() {
    let mut vm = Vm::with_gc_threshold(0);
    assert_eq!(
        vm.run(&[print(binary(text("a"), Op::Add, text("b")))])
            .unwrap(),
        ["ab"]
    );
    assert_eq!(vm.gc_stats().live_objects, 0);
    assert!(vm.gc_stats().collections > 1);
}
