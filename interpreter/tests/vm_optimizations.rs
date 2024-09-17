mod common;
use common::*;
use interpreter::ast::*;
use interpreter::vm::{compile, compile_with_options, Chunk, CompileOptions, Vm};

fn optimized(program: &[Stmt]) -> Chunk {
    compile_with_options(program, CompileOptions { optimize: true }).unwrap()
}

fn optimized_vm() -> Vm {
    let mut vm = Vm::new();
    vm.set_compile_options(CompileOptions { optimize: true });
    vm
}

#[test]
fn text_literals_and_name_operands_share_a_deterministic_constant_pool() {
    let program = vec![
        var("shared", Some(text("shared"))),
        print(name("shared")),
        block(vec![
            var("shared", Some(text("changed"))),
            print(name("shared")),
        ]),
        expression(assign("shared", text("changed"))),
        print(name("shared")),
        print(text("shared")),
    ];
    assert_eq!(
        optimized(&program).constants(),
        [
            Literal::String("shared".into()),
            Literal::String("changed".into())
        ]
    );
    assert!(compile(&program).unwrap().constants().len() > 2);
    let mut vm = optimized_vm();
    vm.set_gc_stress(true);
    assert_eq!(
        vm.run(&program).unwrap(),
        ["shared", "changed", "changed", "shared"]
    );
    assert_eq!(vm.gc_stats().live_objects, 2);
}

#[test]
fn unary_literal_folding_removes_only_safe_operations() {
    let program = vec![
        print(unary(Op::Negate, num(0.0))),
        print(unary(Op::Negate, unary(Op::Negate, num(2.0)))),
        print(unary(Op::Not, nil())),
        print(unary(Op::Not, text(""))),
        print(unary(Op::Not, boolean(false))),
    ];
    let chunk = optimized(&program);
    let listing = chunk.disassemble().unwrap();
    assert!(!listing.contains("NEGATE"), "{listing}");
    assert!(!listing.contains("NOT\n"), "{listing}");
    assert_eq!(
        optimized_vm().run(&program).unwrap(),
        ["-0", "2", "true", "false", "true"]
    );
    assert!(compile(&program).unwrap().code().len() > chunk.code().len());
    let invalid = vec![print(unary(Op::Negate, nil()))];
    assert!(optimized(&invalid)
        .disassemble()
        .unwrap()
        .contains("NEGATE"));
    assert_eq!(optimized_vm().run(&invalid), Vm::new().run(&invalid));
    let invalid_operator = print(unary(Op::Not, unary(Op::Add, num(1.0))));
    assert!(
        compile_with_options(&[invalid_operator], CompileOptions { optimize: true })
            .unwrap_err()
            .contains("Invalid unary operator")
    );
}

#[test]
fn unary_optimizer_does_not_remove_assignments_or_self_initializer_errors() {
    let program = vec![
        var("x", Some(boolean(false))),
        print(unary(Op::Not, assign("x", boolean(true)))),
        print(name("x")),
    ];
    assert_eq!(optimized_vm().run(&program).unwrap(), ["false", "true"]);
    let invalid = block(vec![var("x", Some(unary(Op::Not, name("x"))))]);
    assert!(optimized_vm()
        .run(&[invalid])
        .unwrap_err()
        .contains("initializer"));
}

#[test]
fn numeric_pool_deduplicates_by_bits_without_merging_signed_zero_or_nan_payloads() {
    let nan = f64::from_bits(0x7ff8_0000_0000_0042);
    let other_nan = f64::from_bits(0x7ff8_0000_0000_0043);
    let values = [1.5, 1.5, 0.0, -0.0, nan, nan, other_nan];
    let program: Vec<_> = values.into_iter().map(|n| print(num(n))).collect();
    let chunk = optimized(&program);
    let bits: Vec<_> = chunk
        .constants()
        .iter()
        .map(|value| match value {
            Literal::Number(n) => n.to_bits(),
            other => panic!("unexpected constant: {other:?}"),
        })
        .collect();
    assert_eq!(
        bits,
        [
            1.5f64.to_bits(),
            0.0f64.to_bits(),
            (-0.0f64).to_bits(),
            nan.to_bits(),
            other_nan.to_bits()
        ]
    );
    assert_eq!(compile(&program).unwrap().constants().len(), values.len());
    assert_eq!(
        optimized_vm().run(&program).unwrap(),
        ["1.5", "1.5", "0", "-0", "NaN", "NaN", "NaN"]
    );
}
