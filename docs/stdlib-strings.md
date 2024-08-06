# Native strings

The tree backend installs these functions as ordinary global callable values.
No import is required. Native functions can be aliased or shadowed like other
values. See [numbers and assertions](stdlib-math.md) for calling conventions,
source-lined diagnostics, strict conversion, and `assert`.

## Shared text semantics

- Inputs must be strings unless a numeric count/index is specified. No implicit
  number, boolean, or object-to-text conversion occurs.
- Strings are immutable UTF-8 values. Transformations return strings without
  changing their inputs.
- Matching is literal and case-sensitive: no regular expressions, wildcard
  expansion, Unicode normalization, or locale-aware collation.
- Unicode scalar values are not bytes or grapheme clusters. A visible glyph can
  contain multiple scalars, such as an `e` followed by a combining accent.
- Arity is exact. Type errors identify the function and one-based argument
  position; runtime failures carry the call's source line.

## Trimming and case mapping

- `trim(text)`: remove Unicode whitespace at both ends, not internal spaces.
  This uses Rust's Unicode whitespace definition; zero-width space U+200B is
  not whitespace for this operation.
- `lower(text)` / `upper(text)`: locale-independent Unicode lowercase/uppercase
  mapping. Mappings can change scalar and byte counts: `upper("straße")` is
  `"STRASSE"`, and `lower("İ")` is `i` followed by combining dot U+0307.
  These operations are not Unicode case folding and are not locale-specific
  Turkish casing rules.

```msa
print trim(" \t café \r\n"); // => café
print lower("CAFÉ Ω"); // => café ω
print upper("straße"); // => STRASSE
var original = "Straße";
var changed = upper(original);
print original; // => Straße
print changed; // => STRASSE
```

## Literal matching

- `starts_with(text, prefix)`: whether `text` begins with `prefix`.
- `ends_with(text, suffix)`: whether `text` ends with `suffix`.
- `contains(text, pattern)`: whether `pattern` appears anywhere in `text`.

An empty prefix, suffix, or pattern matches every string, including an empty
one. A nonempty pattern does not match an empty string. Canonically equivalent
but differently encoded strings need not match: precomposed `é` and `e` followed
by a combining accent remain distinct.

```msa
print starts_with("café 🦀", "café"); // => true
print ends_with("café 🦀", "🦀"); // => true
print contains("a.b", "."); // => true
print contains("a.b", ".*"); // => false
print contains("Hello", "hello"); // => false
print starts_with("", ""); // => true
print contains("", "x"); // => false
```

## Replacement

`replace(text, pattern, replacement)` replaces all **nonoverlapping** occurrences,
scanning left to right. Replacement text is literal: `$1` is not a capture
reference and inserted text is not searched again. Missing patterns return the
same text content. An empty replacement removes matches.

An empty pattern inserts the replacement at the start, end, and between Unicode
scalars. An empty input with an empty pattern therefore produces one replacement.

```msa
print replace("aaaaa", "aa", "b"); // => bba
print replace("a.b.a", ".", "$1"); // => a$1b$1a
print replace("banana", "na", ""); // => ba
print replace("é🦀", "", "-"); // => -é-🦀-
print replace("", "", "x"); // => x
```

## Bounded repetition

`repeat(text, count)` requires a finite, nonnegative integral number. Fractional
counts, negative values, NaN, and either infinity are runtime errors. Negative
zero counts as zero. A numeric string such as `"2"` is not a count.

Each result is limited to **1,048,576 UTF-8 bytes (1 MiB)**, inclusive. The size
check happens before converting to a machine-sized count or allocating, so even
a huge finite count cannot overflow a size calculation. This is a byte limit,
not a scalar limit: `repeat("🦀", 262144)` reaches it exactly.

Repeating an empty string or using a zero count yields an empty string. Empty
strings allow arbitrarily large valid finite integral counts because they need
no output allocation. Invalid counts still fail for empty inputs. The limit is
per repetition result, not a total program memory limit; other transformations
and concatenations are not covered by it.

```msa
print repeat("é🦀", 2); // => é🦀é🦀
print repeat("x", 0) == ""; // => true
print repeat("", number("1e300")) == ""; // => true
```

## Unicode scalar indexing

`char_at(text, index)` uses a zero-based Unicode scalar index. It returns a
one-scalar string, or `nil` when the index is out of range. Empty inputs always
return `nil` for valid indices. Like repetition counts, indices must be finite,
nonnegative integers; numeric strings, fractions, negatives, NaN, and infinity
are errors. Negative zero is index zero. Huge valid finite indices return `nil`,
not a host integer overflow or a panic.

Indexing scans up to the requested scalar (linear time), not a UTF-8 byte offset.
Combining marks and components of multi-scalar emoji can be returned separately;
this API does not segment user-perceived characters.

```msa
print char_at("aé🦀", 1); // => é
print char_at("aé🦀", 2); // => 🦀
print char_at("aé🦀", 3); // => nil
print char_at("", 0); // => nil
print char_at("x", number("1e300")); // => nil
```

## Running and testing

From the repository root:

```sh
cargo run -p interpreter -- --backend tree -e 'print upper(trim(" café "));'
cargo test --test stdlib
cargo test --test stdlib_examples
```

Integration tests execute all `msa` blocks in this document, checking their
`// =>` output annotations. Separate runnable programs live in
`examples/stdlib-*.msa` and are also exercised through the tree CLI.
