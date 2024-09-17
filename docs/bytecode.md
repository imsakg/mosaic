# Mosaic bytecode and literal optimization

The bytecode backend compiles a complete AST before executing anything. A `Chunk`
owns bytes, source lines and immutable literals, not AST nodes or runtime GC
handles. It can be reused in the same VM or in different VMs after collections.
Functions, calls, classes, properties, `this` and `super` remain unsupported;
compilation rejects them even in unreachable source. Use the tree backend for
those language features.

## Inspect without executing

From the repository root:

```sh
cargo run -q -p interpreter -- --backend vm --compile-only --optimize -e 'print (1+2)*3;'
cargo run -q -p interpreter -- --backend vm --disassemble -e 'print 1+2;'
```

`--compile-only` prints byte, instruction, constant and branch counts. It does not
execute instructions or require global names to exist. `--check` remains silent
on success; `--disassemble` prints the actual selected bytecode. Inspection modes
are mutually exclusive. Supply a file, `-` for a complete program on stdin, or
`-e source`.

## Encoding and source maps

- Every opcode is one byte. `CONSTANT`, `GET_NAME`, `DEFINE_NAME`, `SET_NAME`,
  `JUMP` and `JUMP_IF_FALSE` have a four-byte unsigned little-endian operand.
- Constant operands index `Chunk::constants()`. Name operands must reference
  string literals. `NIL`, `TRUE` and `FALSE` do not consume constant slots.
- Jump operands are absolute **byte offsets**, never instruction indices.
  Both instruction starts and the offset immediately after the last instruction
  are valid destinations. Operand interiors and offsets beyond the end are not.
- `JUMP_IF_FALSE` peeks without popping. Only `nil` and `false` are falsey.
  Compiled control flow explicitly pops conditions on the appropriate paths.
- The operand stack holds expression values. `ENTER_SCOPE`/`EXIT_SCOPE` manage
  lexical environments. `PRINT` consumes a value and adds its rendering to the
  per-run result; it does not write to host stdout.
- `line_at(offset)` returns the source line for any byte, including operand
  bytes, or `None` past the end. `offsets_for_line(line)` returns instruction
  starts only. Folding attributes the replacement to the expression's operator
  line; the following `PRINT` retains its statement line.

`instructions()` is a fused iterator of decoded `Instruction` results containing
`offset`, `next_offset`, `line`, `opcode` and an optional `operand`. `stats()`
counts encoded bytes, decoded instructions, constants and both kinds of jumps.
`OpCode::name()` exposes the disassembler's mnemonic.

This is an in-process educational format, not a versioned portable file format.
Opcode discriminants and the constant representation have no persistence ABI.

## Opt-in optimization contract

Existing `compile`, `disassemble` and `Vm::run` defaults remain **unoptimized**.
This preserves exact instructional listings and GC-stress allocation behavior.
Pass `CompileOptions { optimize: true }`, configure `Vm::set_compile_options`, or
use `--backend vm --optimize` to enable:

- Numeric constant pooling by `f64::to_bits()`. Positive/negative zero remain
  distinct slots. Equal NaN bit patterns share slots; different payloads do not.
- String pooling shared by literal strings and all name operands. The first
  encounter determines each index; randomized hash iteration never determines
  bytecode order.
- Recursive literal-only unary `!` and numeric negation folding.
- Numeric arithmetic and direct order comparisons using the same IEEE 754
  operations as execution. Division by zero, infinities, NaNs and signed zero
  are preserved. No reassociation, multiply-by-zero or similar identities occur.
- Literal equality/inequality, including strict cross-type comparisons, and
  literal string concatenation when the result is at most **4096 UTF-8 bytes**.
  Larger joins remain runtime operations, not compilation errors.
- Logical `and`/`or` folding only when **both** operands are valid, fully literal
  expressions. It returns the selected operand, not a coerced boolean.

Names and assignments are never substituted or removed. Invalid operator
positions still fail compilation; type errors stay runtime errors on their
original source lines. Unsupported syntax is still checked in short-circuited
expressions. Optimizing can reduce the number of instructions and allocations,
so instruction budgets and traces are specific to the selected compilation mode.

## Reusable compilation API

This Rust example is compiled and executed by `cargo test --doc`:

```rust
use interpreter::{parse, vm::{compile, compile_with_options, CompileOptions, Vm}};
let program = parse("print 1+2;").unwrap();
let original = compile(&program).unwrap();
let optimized = compile_with_options(&program, CompileOptions { optimize: true }).unwrap();
assert!(optimized.stats().unwrap().instruction_count < original.stats().unwrap().instruction_count);
let mut vm = Vm::new();
assert_eq!(vm.run_chunk(&optimized).unwrap(), ["3"]);
vm.collect_garbage();
assert_eq!(vm.run_chunk(&optimized).unwrap(), ["3"]);
```

`run_chunk` executes its supplied bytes without recompilation; VM compile options
only affect `run`. Globals persist between runs, while output, operand stacks and
lexical scopes are per-run. See [debugging and recovery](debugging.md) for bounds,
traces and error semantics.

`Chunk::from_parts(code, lines, constants)` and `validate()` check source-map
length, opcode encodings, complete operands, constant/name types and indices, and
branch boundaries. They do **not** prove stack balance, lexical balance, operand
types or termination. Runtime checks still apply; use an instruction budget for
untrusted or potentially looping bytecode. Structural validation is not a sandbox.

## Executed language examples

The integration suite reads the following actual `mosaic`/`output` fence pairs
and executes each with optimization off and on, including GC stress.

```mosaic
print (1+2)*3;
print "hi " + "🎼";
print false or "fallback";
```

```output
9
hi 🎼
fallback
```

```mosaic
var count = 0;
while (count < 3) {
    count = count + 1;
}
{ var count = 9; print count; }
print count;
```

```output
9
3
```
