# Tuples, maps and text sequences

Collections are native values in the **tree backend** (the default). These are
function APIs, not new literal syntax: parentheses still group expressions, and
`tuple` and `map` are ordinary, shadowable native globals. The bytecode backend
rejects calls at compilation rather than silently falling back to tree execution.

Run the tested example:

```sh
cargo run -- examples/collections.msa
```

## Tuples

- `tuple(...)` constructs an ordered, immutable sequence. A call accepts at most
  255 arguments, like other Mosaic calls. `append` and `concat` build larger ones.
- `len(value)` counts tuple elements, map entries or Unicode scalar values in a
  string. It does not count UTF-8 bytes or grapheme clusters.
- `get(tuple, index)` uses a finite, nonnegative integer index, starting at zero.
  An out-of-range index returns `nil`; negative, fractional, NaN, infinite or
  nonnumeric indices are runtime errors. Large valid indices do not overflow.
- `append(tuple, value)` returns a new tuple with the value appended.
- `concat(left, right)` joins two tuples, preserving order and nested values.
- `slice(tuple_or_string, start, end)` uses a half-open interval. Both bounds must
  be finite, nonnegative integers. Bounds beyond the end are clamped. A start
  greater than the end is an error, even if both would otherwise clamp equally.
  String slicing uses Unicode scalar positions, not UTF-8 offsets.

```msa
var a = tuple(1, 2);
var b = append(a, 3);
print a;                     // (1, 2)
print b;                     // (1, 2, 3)
print get(b, 99);             // nil
print slice("aé🦀z", 1, 3);   // é🦀
```

Printing an empty tuple produces `()`. A singleton has a trailing comma, e.g.
`(1,)`. Both empty tuples and empty maps are truthy, like empty strings and zero.
Only `nil` and `false` are falsey.

## Maps

`map(...)` accepts zero or more two-element tuples: `map(tuple("key", value))`.
Keys must be strings. Duplicate keys use the last supplied value. All traversal
uses lexicographic string-key order, independent of insertion order and locale.

- `get(map, key)` returns the associated value, or `nil` if absent.
- `has(map, key)` distinguishes absence from a stored `nil`.
- `put(map, key, value)` returns a new map, replacing or inserting that key.
- `remove(map, key)` returns a new map without the key; absence is not an error.
- `keys(map)` returns an ordered tuple of strings.
- `values(map)` returns the corresponding ordered tuple of values.
- `entries(map)` returns a tuple of two-element key/value tuples, in the same order.

```msa
var original = map(tuple("z", 1), tuple("a", nil));
var changed = put(original, "a", 2);
print get(original, "a");     // nil
print get(changed, "a");      // 2
print has(original, "a");     // true
print has(original, "absent"); // false
print keys(original);         // (a, z)
print values(changed);        // (2, 1)
```

Map output quotes keys and renders values using normal Mosaic display rules.
Strings inside tuples or map values remain unquoted. Display is for humans,
**not** a serialization format or a round-trip source representation.

## Equality, aliasing and collection

Tuples and maps compare structurally, including nested collections. Map entry
order does not affect equality. Numbers retain IEEE behavior: a nested NaN does
not equal another NaN. Functions, instances and classes retain object-identity
semantics; containers do not copy their object graphs.

An update never mutates the old container. However, storing an instance preserves
its identity: changing that instance's fields is visible through either container.
This is container immutability, not a deep freeze. The tracing collector follows
object references inside nested tuples and maps, including bound methods and
cycles through instance fields, so collection-only roots remain live across runs.

## Text bridges and inspection

- `split(text, separator)` treats a nonempty separator literally, not as a regex.
  Adjacent or trailing separators retain empty fields. An empty separator splits
  into Unicode scalar values with no artificial leading/trailing empty fields;
  `split("", "")` is an empty tuple.
- `join(tuple_of_strings, separator)` inserts the separator between strings. It
  rejects nonstring elements rather than coercing them. Joining an empty tuple
  produces an empty string.
- `type_of(value)` returns `nil`, `boolean`, `number`, `string`, `tuple`, `map`,
  `function`, `class` or `instance` for source-level values. Bound methods and
  native callables report `function`.
- `str(value)` returns the human-facing display of any value, without executing it.
  Other string functions do not implicitly call `str`.

All fixed-signature functions enforce their arity and argument types. All
arguments are evaluated left-to-right before native validation, including calls
that fail. Native functions are first-class values and can be aliased or shadowed.

## Resource bounds

Every constructed collection is limited to 65,536 direct items, 64 nested
collection levels and 1,048,576 expanded value nodes. Expanded size counts repeated
occurrences even when immutable storage is shared. This prevents a tiny shared
binary graph from causing exponential display, equality or tracing work.
Transforms such as `entries` are subject to the same bounds and can fail when they
add a nesting layer. A failed update leaves existing aliases unchanged.

`join` and `repeat` limit results to 1,048,576 UTF-8 bytes. `split` is bounded by the
collection item limit. These are targeted guards, not a global memory quota for
all language strings, output, globals or heap objects.

See [the string library](stdlib-strings.md), [numeric library](stdlib-math.md) and
[VM debugging contract](debugging.md) for the other native APIs and backend limits.
