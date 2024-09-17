# Debugging Mosaic VM execution

Debugging is opt-in. Normal VM execution still has no instruction limit, records
no traces, and uses unoptimized bytecode. See [the bytecode contract](bytecode.md)
for encoding, validation and literal optimization.

## CLI controls

Run these commands from the repository root:

```sh
cargo run -q -p interpreter -- --backend vm --trace --trace-limit 8 -e 'print 1+2;'
cargo run -q -p interpreter -- --backend vm --instruction-limit 24 --trace --trace-limit 3 -e 'while (true) {}'
cargo run -q -p interpreter -- --backend vm --optimize --compile-only -e 'print 1+2;'
```

The second command intentionally exits with **70**, reporting an exhausted
instruction budget. Invalid syntax or unsupported VM features exit with **65**;
invalid CLI combinations exit with **64**. Successful execution/inspection exits
with **0**.

- `--trace` writes instruction metadata to **stderr**; language output stays on
  **stdout**. It requires `--backend vm` and an explicit file, `-` (complete source
  from stdin), or `-e source`. It is not a REPL display mode.
- `--trace-limit N` requires `--trace`. Its default is **1024**, and explicit
  values must be in **1..=1000000**. A truncation line reports the retained event
  count when execution attempted more events than fit.
- `--instruction-limit N` requires `--backend vm`. It allows at most N attempted
  instructions **per source unit**, including each submitted REPL program. Zero
  permits an empty program but prevents the first instruction of a nonempty one.
  Omitting the flag leaves execution unlimited.
- Trace/budget flags conflict with inspection (`--tokens`, `--ast`, `--check`,
  `--disassemble`, `--compile-only`): inspection never executes instructions.
- `--optimize` can be combined with tracing or budgets. Folding changes instruction
  counts and source-line coverage; compare like-for-like compilation modes.

Example trace record:

```text
trace 0000 line=1 CONSTANT stack=0 scopes=0
```

Offsets are byte offsets. Stack and lexical-scope depths are captured **before**
the named instruction. A runtime-failing instruction is included in both the
trace and the attempted-instruction count. A budget-blocked next instruction is
not included in either. Trace truncation never stops execution; use a budget to
interrupt a loop. The error line points at the next instruction the budget
prevented from running.

## Embedding: bounded diagnostics and recovery

`Vm::set_trace_limit(limit)` retains the **first** events up to that bound, not a
rolling tail. Zero disables it. `trace()` borrows the last run's events;
`trace_truncated()` tells whether events were omitted. Events contain only byte
position, source line, opcode and depths, never string values or GC handles.
Changing the trace limit discards the previous buffer. Each run resets trace,
truncation and `instructions_executed()`, including empty runs and compile errors.

`Vm::set_instruction_limit(Some(limit))` configures a persistent per-run bound;
`None` disables it. `instructions_executed()` saturates at `u64::MAX` in an
unlimited run. A budget is checked immediately before the next instruction, so
exactly fitting programs succeed. Compile and parse work is outside the budget.
`Session` forwards these trace/budget controls and clears old diagnostics even
when lexing or parsing fails before the VM sees an AST. Embedders using their own
parser can call `Vm::clear_debug_state()` at that boundary.

This example is compiled and executed by `cargo test --doc`:

```rust
use interpreter::{ast::Literal, parse, vm::Vm};
let mut vm = Vm::with_gc_threshold(1);
vm.set_gc_stress(true);
vm.set_trace_limit(3);
vm.set_instruction_limit(Some(24));
let looping = parse(r#"var kept="safe"; while (true) { var local="temporary"; }"#).unwrap();
assert!(vm.run(&looping).unwrap_err().contains("Instruction budget exhausted"));
assert_eq!(vm.instructions_executed(), 24);
assert_eq!(vm.trace().len(), 3);
assert!(vm.trace_truncated());
assert_eq!(vm.global_snapshot().unwrap(), [("kept".into(), Literal::String("safe".into()))]);
assert_eq!(vm.gc_stats().live_objects, 2); // global name and string value
vm.set_instruction_limit(None);
assert_eq!(vm.run(&parse("print kept;").unwrap()).unwrap(), ["safe"]);
assert_eq!(vm.instructions_executed(), 2);
assert!(!vm.trace_truncated());
```

### State after failure

Runtime errors and budget exhaustion are **not transactions**. Completed global
definitions/assignments persist; later instructions do not run. Every exit path
clears operand stacks and lexical environments and collects unreachable strings.
Global keys/values remain roots. The same VM and compiled chunks can be reused
after a failure. Compilation completes before any execution, so compilation
failure has no program effects.

`global_snapshot()` returns a fallible, sorted vector of `(String, Literal)`
pairs. It excludes locals, copies strings into owned Rust storage, preserves
number bits, and does not intern or allocate VM heap objects. An old snapshot
stays valid after globals change, GC runs, or the VM is dropped. It is for
inspection, not a restore/checkpoint API.

A successful run returns its own output vector. The existing VM error API returns
only an error string, so buffered output from a failed run is discarded (unlike
the tree backend's partial-output reporting). Traces on failures are available
separately and must not be mistaken for language output.

### Resource boundaries

A trace is memory-bounded in terms of its configured event count. An instruction
budget is **not** a wall-clock deadline, parser limit, heap quota, output quota or
security sandbox. Individual instructions can allocate or manipulate large
strings; parsing, compiling, validating chunks and taking global snapshots are
not charged against the execution budget. Structurally valid raw chunks can still
fail stack/type checks or loop forever without a budget.

## Tested outcome examples

The integration suite reads each actual `mosaic`, `budget` and `outcome` fence
below and exercises both compile modes under GC stress. `outcome` contains a
kind (`output`, `runtime_error` or `compile_error`) followed by the expected output
or diagnostic substring. This fixture metadata is not Mosaic source syntax.

```mosaic
print (1+2)*2;
```

```budget
16
```

```outcome
output
6
```

```mosaic
while (true) { var temporary = "discard"; }
```

```budget
24
```

```outcome
runtime_error
Instruction budget exhausted.
```

```mosaic
fun unsupported() {}
```

```budget
0
```

```outcome
compile_error
Functions are not supported
```
