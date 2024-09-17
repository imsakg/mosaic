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
