# Mosaic

![Mosaic logo](resources/logo.jpg)

Mosaic Script is an educational programming language inspired by Bob Nystrom's
[Crafting Interpreters](https://craftinginterpreters.com/). Its source files use
`.msa`. The Rust workspace contains a scanner, parser, AST, lexical resolver,
tree interpreter, bytecode compiler/VM, tracing collectors and persistent REPL.
The native standard-library functions are implemented in Rust. This is a learning
project, not a production language or an untrusted-code security sandbox.

## Build and run

Install a stable Rust toolchain, then run from this checkout:

```sh
cargo build --workspace
cargo run -- -e 'print sqrt(81);'
cargo run -- examples/collections.msa
cargo run -- examples/stdlib-strings.msa
cargo run --
```

With no source argument the REPL preserves globals between inputs. Use `:help`
for its commands and `= expression` to print an expression. Source can also come
from a file, `-` for stdin, or `-e` for an inline program:

```sh
printf 'print 2 + 3;\n' | cargo run -- -
cargo run -- --check examples/collections.msa
cargo run -- --tokens -e 'print 1;'
cargo run -- --ast -e 'var answer = 42;'
```

## Language and backends

The **tree backend is the default**. It supports lexical variables, blocks,
conditionals, loops, closures, functions, classes, inheritance, bound methods,
first-class native functions, immutable tuples and string-keyed maps. Only `nil`
and `false` are falsey. Numbers are IEEE 754 binary64 values.

```msa
var words = split("pear apple pear", " ");
var counts = map(tuple("pear", 2), tuple("apple", 1));
print get(words, 0);              // pear
print keys(counts);               // (apple, pear)
print join(words, " / ");         // pear / apple / pear
print type_of(tuple(1, true));    // tuple
```

The **bytecode backend** runs literals, arithmetic, comparisons, logical
expressions, variable bindings, blocks and control flow. It does **not** support
function calls, functions, classes, properties or collections yet. Unsupported
syntax is a compile error, including in unreachable branches; execution never
falls back to the tree interpreter. Optimization is opt-in.

```sh
cargo run -- --backend vm --optimize -e 'print (2 + 3) * 4;'
cargo run -- --backend vm --optimize --compile-only -e 'print 2 + 3;'
cargo run -- --backend vm --disassemble -e 'print 2 + 3;'
cargo run -- --backend vm --trace --trace-limit 8 --instruction-limit 1000 \
  -e 'var n = 0; while (n < 3) { print n; n = n + 1; }'
```

VM traces go to stderr, leaving program output on stdout. Trace buffers and
instruction budgets are bounded and reset per source unit. The embedding API can
reuse validated chunks and inspect decoded instructions, source mappings,
statistics and sorted global snapshots.

## Library and debugging reference

- [Tuples, maps, slicing, text bridges and type inspection](docs/collections.md)
- [Math, number conversion, classification and assertions](docs/stdlib-math.md)
- [Unicode string operations and string allocation guards](docs/stdlib-strings.md)
- [Bytecode encoding, validation and optimizer contracts](docs/bytecode.md)
- [VM tracing, instruction budgets and embedding APIs](docs/debugging.md)

Structured scan/parse and tree-session errors include bounded, plain-text source
excerpts. Parse errors distinguish unexpected tokens from end-of-input. Tree
runtime errors retain the original failure line and up to sixteen innermost-first
named call frames; recursive traces show when further frames are omitted.
`Diagnostic`'s compact `Display` remains available to embedders separately from
its source-aware `render` method. The REPL can recover after compile/runtime errors.

Exit codes: `0` success, `64` invalid usage, `65` scan/parse/compile error,
`70` runtime error and `74` I/O error. Partial output emitted before a tree-runtime
error is preserved. Runtime effects before an error are not rolled back.

## Verify changes

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --release
```

The tests execute the checked-in examples, compare optimized and unoptimized VM
semantics, cover IEEE edge cases and Unicode boundaries, validate malformed
bytecode, and check GC roots, cycles, allocation bounds and post-error recovery.
