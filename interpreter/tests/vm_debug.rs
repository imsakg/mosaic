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
fn raw_chunk_validation_rejects_malformed_operands_and_branch_interiors() {
    use interpreter::vm::Chunk;
    let raw =
        |code: Vec<u8>, constants| Chunk::from_parts(code.clone(), vec![7; code.len()], constants);
    assert!(Chunk::from_parts(vec![OpCode::Nil as u8], vec![], vec![])
        .unwrap_err()
        .contains("Source map length"));
    assert!(raw(vec![u8::MAX], vec![])
        .unwrap_err()
        .contains("Unknown opcode"));
    assert!(raw(vec![OpCode::Constant as u8, 0, 0], vec![])
        .unwrap_err()
        .contains("Truncated instruction"));
    assert!(raw(vec![OpCode::Constant as u8, 0, 0, 0, 0], vec![])
        .unwrap_err()
        .contains("Invalid constant index"));
    assert!(raw(
        vec![OpCode::GetName as u8, 0, 0, 0, 0],
        vec![Literal::Number(1.0)]
    )
    .unwrap_err()
    .contains("Invalid name constant"));
    for opcode in [OpCode::Jump, OpCode::JumpIfFalse] {
        for target in [1, 4, 6, u32::MAX] {
            let mut code = vec![opcode as u8];
            code.extend_from_slice(&target.to_le_bytes());
            assert!(raw(code, vec![])
                .unwrap_err()
                .contains("instruction boundary"));
        }
        for target in [0u32, 5] {
            let mut code = vec![opcode as u8];
            code.extend_from_slice(&target.to_le_bytes());
            raw(code, vec![]).unwrap().validate().unwrap();
        }
    }
    let chunk = compile(&[while_(boolean(false), block(vec![print(text("skip"))]))]).unwrap();
    chunk.validate().unwrap();
    let rebuilt = Chunk::from_parts(
        chunk.code().to_vec(),
        (0..chunk.code().len())
            .map(|i| chunk.line_at(i).unwrap())
            .collect(),
        chunk.constants().to_vec(),
    )
    .unwrap();
    assert_eq!(rebuilt.disassemble(), chunk.disassemble());
}

#[test]
fn compiled_chunks_are_reusable_across_runs_heaps_and_collections() {
    let initialization = compile(&[var("count", Some(num(0.0)))]).unwrap();
    let increment = compile(&[print(assign(
        "count",
        binary(name("count"), interpreter::ast::Op::Add, num(1.0)),
    ))])
    .unwrap();
    for _ in 0..2 {
        let mut vm = Vm::with_gc_threshold(1);
        vm.set_gc_stress(true);
        vm.run_chunk(&initialization).unwrap();
        assert_eq!(vm.run_chunk(&increment).unwrap(), ["1"]);
        vm.collect_garbage();
        assert_eq!(vm.run_chunk(&increment).unwrap(), ["2"]);
    }
    let literal = compile(&[print(text("retained by chunk, not heap"))]).unwrap();
    let mut vm = Vm::new();
    for _ in 0..3 {
        assert_eq!(
            vm.run_chunk(&literal).unwrap(),
            ["retained by chunk, not heap"]
        );
        assert_eq!(vm.gc_stats().live_objects, 0);
    }
}

#[test]
fn chunk_runtime_failures_clean_transient_state_and_allow_recovery() {
    use interpreter::vm::Chunk;
    let mut vm = Vm::with_gc_threshold(1);
    vm.set_gc_stress(true);
    let failure = compile(&[
        var("kept", Some(text("global"))),
        block(vec![
            var("local", Some(text("secret"))),
            print(name("missing")),
        ]),
    ])
    .unwrap();
    assert!(vm
        .run_chunk(&failure)
        .unwrap_err()
        .contains("Undefined variable"));
    assert_eq!(vm.gc_stats().live_objects, 2);
    assert_eq!(
        vm.run_chunk(&compile(&[print(name("kept"))]).unwrap())
            .unwrap(),
        ["global"]
    );
    for (opcode, expected) in [
        (OpCode::Pop, "Operand stack underflow"),
        (OpCode::ExitScope, "Lexical scope underflow"),
        (OpCode::EnterScope, "Unbalanced bytecode state"),
    ] {
        let chunk = Chunk::from_parts(vec![opcode as u8], vec![23], vec![]).unwrap();
        let error = vm.run_chunk(&chunk).unwrap_err();
        assert!(error.contains(expected), "{error}");
        assert!(error.contains("[line 23]"));
        assert!(vm.run_chunk(&Chunk::default()).unwrap().is_empty());
        assert_eq!(vm.gc_stats().live_objects, 2);
    }
}

#[test]
fn traces_are_opt_in_bounded_and_record_pre_instruction_state() {
    let program = [block(vec![print(num(3.0))])];
    let mut vm = Vm::new();
    vm.run(&program).unwrap();
    assert!(vm.trace().is_empty());
    assert!(!vm.trace_truncated());
    vm.set_trace_limit(16);
    assert_eq!(vm.run(&program).unwrap(), ["3"]);
    let trace = vm.trace();
    assert_eq!(trace.len(), 4);
    assert_eq!(
        (
            trace[0].opcode,
            trace[0].offset,
            trace[0].stack_depth,
            trace[0].scope_depth
        ),
        (OpCode::EnterScope, 0, 0, 0)
    );
    assert_eq!(
        (trace[1].opcode, trace[1].stack_depth, trace[1].scope_depth),
        (OpCode::Constant, 0, 1)
    );
    assert_eq!(
        (trace[2].opcode, trace[2].stack_depth, trace[2].scope_depth),
        (OpCode::Print, 1, 1)
    );
    assert_eq!(trace[3].opcode, OpCode::ExitScope);
    assert!(!vm.trace_truncated());
    vm.set_trace_limit(2);
    vm.run(&program).unwrap();
    assert_eq!(vm.trace().len(), 2);
    assert!(vm.trace_truncated());
    vm.run(&[]).unwrap();
    assert!(vm.trace().is_empty());
    assert!(!vm.trace_truncated());
    vm.set_trace_limit(0);
    vm.run(&program).unwrap();
    assert!(vm.trace().is_empty());
    assert!(!vm.trace_truncated());
}

#[test]
fn trace_includes_failing_instructions_and_resets_on_compile_failure() {
    let mut vm = Vm::new();
    vm.set_trace_limit(4);
    let mut missing = name("missing");
    missing.line = 42;
    assert!(vm.run(&[print(missing)]).is_err());
    assert_eq!(
        (vm.trace()[0].opcode, vm.trace()[0].line),
        (OpCode::GetName, 42)
    );
    let saved = vm.trace().to_vec();
    vm.collect_garbage();
    assert_eq!(saved, vm.trace());
    let invalid = expression(unary(interpreter::ast::Op::Add, num(1.0)));
    assert!(vm.run(&[invalid]).unwrap_err().contains("Compile error"));
    assert!(vm.trace().is_empty());
    assert!(!vm.trace_truncated());
    vm.run(&[print(boolean(true))]).unwrap();
    assert_eq!(vm.trace().len(), 2);
    assert_eq!(vm.trace()[0].opcode, OpCode::True);
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
