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
fn numeric_folding_uses_ieee_operations_and_preserves_expression_order() {
    let pairs = [
        (7.0, 2.0),
        (0.0, -0.0),
        (1.0, 0.0),
        (0.0, 0.0),
        (f64::INFINITY, f64::NEG_INFINITY),
        (f64::MAX, 2.0),
        (f64::NAN, 1.0),
    ];
    for (left, right) in pairs {
        for (operator, expected) in [
            (Op::Add, left + right),
            (Op::Subtract, left - right),
            (Op::Multiply, left * right),
            (Op::Divide, left / right),
        ] {
            let program = [print(binary(num(left), operator, num(right)))];
            let chunk = optimized(&program);
            assert_eq!(chunk.code().len(), 6, "{operator:?}");
            let Literal::Number(actual) = chunk.constants()[0] else {
                panic!("not a number")
            };
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "{operator:?}: {left} {right}"
            );
            assert_eq!(optimized_vm().run(&program), Vm::new().run(&program));
        }
        for operator in [Op::Less, Op::LessEqual, Op::Greater, Op::GreaterEqual] {
            let program = [print(binary(num(left), operator, num(right)))];
            assert_eq!(optimized(&program).code().len(), 2);
            assert_eq!(optimized_vm().run(&program), Vm::new().run(&program));
        }
    }
    let nested = [print(binary(
        binary(num(1e16), Op::Add, num(-1e16)),
        Op::Add,
        num(1.0),
    ))];
    assert_eq!(optimized_vm().run(&nested), Vm::new().run(&nested));
}

#[test]
fn numeric_optimizer_does_not_apply_algebraic_shortcuts_or_hide_invalid_operators() {
    let program = [print(binary(num(0.0), Op::Multiply, name("missing")))];
    assert_eq!(optimized_vm().run(&program), Vm::new().run(&program));
    assert!(optimized_vm()
        .run(&program)
        .unwrap_err()
        .contains("Undefined variable"));
    let effects = [
        var("x", Some(num(0.0))),
        print(binary(assign("x", num(2.0)), Op::Multiply, num(0.0))),
        print(name("x")),
    ];
    assert_eq!(optimized_vm().run(&effects), Vm::new().run(&effects));
    let invalid = [print(binary(
        num(0.0),
        Op::Add,
        binary(num(1.0), Op::And, num(2.0)),
    ))];
    assert!(
        compile_with_options(&invalid, CompileOptions { optimize: true })
            .unwrap_err()
            .contains("Invalid binary operator")
    );
}

#[test]
fn bounded_string_concatenation_and_literal_equality_are_folded() {
    let joined = [print(binary(text("hi "), Op::Add, text("🎼")))];
    assert_eq!(optimized(&joined).code().len(), 6);
    assert_eq!(optimized_vm().run(&joined).unwrap(), ["hi 🎼"]);
    let values = [
        nil(),
        boolean(false),
        num(0.0),
        num(-0.0),
        num(f64::NAN),
        text("same"),
        text("different"),
    ];
    for left in &values {
        for right in &values {
            for operator in [Op::Equal, Op::NotEqual] {
                let program = [print(binary(left.clone(), operator, right.clone()))];
                assert_eq!(
                    optimized(&program).code().len(),
                    2,
                    "{operator:?}: {left:?} {right:?}"
                );
                assert_eq!(optimized_vm().run(&program), Vm::new().run(&program));
            }
        }
    }
    let large = [print(binary(text(&"🎼".repeat(1024)), Op::Add, text("x")))];
    assert!(optimized(&large).disassemble().unwrap().contains("ADD"));
    assert_eq!(optimized_vm().run(&large), Vm::new().run(&large));
    let invalid = [print(binary(text("one"), Op::Add, num(1.0)))];
    assert_eq!(optimized_vm().run(&invalid), Vm::new().run(&invalid));
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
