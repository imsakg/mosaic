mod common;
use common::*;
use interpreter::ast::Literal;
use interpreter::vm::{compile, OpCode, Vm};

#[test]
fn source_mapping_covers_operand_bytes_and_folded_operator_locations() {
    let mut left = num(1.0);
    left.line = 10;
    let mut right = num(2.0);
    right.line = 11;
    let mut value = binary(left, interpreter::ast::Op::Add, right);
    value.line = 12;
    let mut statement = print(value);
    statement.line = 13;
    let original = compile(&[statement.clone()]).unwrap();
    assert_eq!(original.line_at(0), Some(10));
    assert_eq!(original.line_at(4), Some(10));
    assert_eq!(original.line_at(5), Some(11));
    assert_eq!(original.line_at(10), Some(12));
    assert_eq!(original.line_at(original.code().len()), None);
    assert_eq!(original.line_at(usize::MAX), None);
    assert_eq!(original.offsets_for_line(10).unwrap(), [0]);
    assert!(original.offsets_for_line(99).unwrap().is_empty());
    let folded = interpreter::vm::compile_with_options(
        &[statement],
        interpreter::vm::CompileOptions { optimize: true },
    )
    .unwrap();
    assert_eq!(folded.line_at(0), Some(12));
    assert_eq!(folded.line_at(4), Some(12));
    assert_eq!(folded.offsets_for_line(13).unwrap(), [5]);
    assert_eq!(
        folded
            .instructions()
            .map(|i| i.unwrap().line)
            .collect::<Vec<_>>(),
        [12, 13]
    );
}

#[test]
fn instruction_iterator_and_statistics_use_real_byte_offsets() {
    let chunk = compile(&[print(num(2.0)), print(boolean(true))]).unwrap();
    let mut instructions = chunk.instructions();
    let first = instructions.next().unwrap().unwrap();
    assert_eq!((first.offset, first.next_offset, first.line), (0, 5, 1));
    assert_eq!(first.opcode, OpCode::Constant);
    assert_eq!(first.opcode.name(), "CONSTANT");
    assert_eq!(first.operand, Some(0));
    let rest: Vec<_> = instructions.by_ref().collect::<Result<_, _>>().unwrap();
    assert_eq!(rest.iter().map(|i| i.offset).collect::<Vec<_>>(), [5, 6, 7]);
    assert!(rest.iter().all(|i| i.operand.is_none()));
    assert!(instructions.next().is_none());
    assert!(instructions.next().is_none());
    let stats = chunk.stats().unwrap();
    assert_eq!(
        (
            stats.byte_len,
            stats.constant_count,
            stats.instruction_count,
            stats.branch_count
        ),
        (8, 1, 4, 0)
    );
    let branches = compile(&[if_(boolean(true), print(nil()), Some(print(nil())))]).unwrap();
    assert_eq!(branches.stats().unwrap().branch_count, 2);
    assert_eq!(compile(&[]).unwrap().stats().unwrap().instruction_count, 0);
    assert_eq!(Vm::new().run(&[print(num(2.0))]).unwrap(), ["2"]);
    assert_eq!(chunk.constants(), [Literal::Number(2.0)]);
}
