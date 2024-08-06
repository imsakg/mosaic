# Native numbers and assertions

These functions are installed as global callable values in the **tree backend**.
They need no imports. They can be aliased, passed to functions, captured by
closures, or shadowed by ordinary declarations. The bytecode VM does not support
native calls yet; select `--backend tree` (the default).

## Calling and failures

- All numeric operations require numbers; strings and booleans are not coerced.
- Arity is exact, except `assert`, which accepts one or two arguments.
- Arguments are evaluated once, left to right, before native validation. A bad
  call does not undo side effects from evaluating its arguments.
- Errors are `Runtime` diagnostics tagged with the call's source line. Arity
  errors name the function and expected/actual counts. Type errors name the
  function and the one-based argument position.
- Numbers are IEEE 754 binary64 values. Division can produce infinities and NaN;
  these functions do not generally restrict results to finite numbers.

## Numeric API

- `abs(x)`: absolute value, including `abs(-0) == 0` with a positive sign.
- `floor(x)`: greatest integral value less than or equal to `x`.
- `ceil(x)`: least integral value greater than or equal to `x`.
- `trunc(x)`: remove the fractional part, toward zero.
- `round(x)`: nearest integral value, with halfway ties **away from zero**.
  Thus `round(2.5)` is `3` and `round(-2.5)` is `-3`, not bankers' rounding.
- `min(a, b)` / `max(a, b)`: compare exactly two values; not variadic.
  Either NaN input produces NaN. For equal zero inputs, `min` chooses negative
  zero if either input is negative; `max` chooses positive zero if either is
  positive. Two negative zeros remain negative in both functions.
- `sqrt(x)`: square root. Negative inputs, including negative infinity, are
  domain errors. Negative zero is allowed and preserved; NaN propagates and
  positive infinity returns positive infinity.
- `pow(base, exponent)`: real floating-point power. Negative bases with fractional
  exponents produce NaN, zero to a negative power can produce infinity, and
  zero to the zero power is `1`. This is not integer or complex arithmetic.
- `sin(x)` / `cos(x)` / `tan(x)`: trigonometric functions in **radians**.
  Infinite inputs produce NaN. Singularities and rounding follow host binary64
  math; do not expect an exact infinity for `tan` near half of pi.
- `log(x)`: natural logarithm. Zero, negative zero, and negative inputs are domain
  errors. Positive infinity returns infinity; NaN propagates.
- `exp(x)`: natural exponential. Overflow returns positive infinity; sufficiently
  negative inputs underflow to zero. `exp(-infinity)` is zero.
- `is_nan(x)`: whether a numeric input is NaN.
- `is_finite(x)`: whether a numeric input is neither NaN nor either infinity.
  Both predicates reject non-numbers rather than returning false for them.
- `number(value)`: explicit strict decimal conversion, described below.

Except for the stated domain errors, rounding and arithmetic functions retain
IEEE special values. Negative zero can be observed through `1 / x`. Results of
transcendental functions may differ in their least significant bits across
platforms; compare with a suitable tolerance, not formatted decimal equality.

```msa
print abs(-12.5); // => 12.5
print floor(-2.8); // => -3
print ceil(-2.8); // => -2
print trunc(-2.8); // => -2
print round(-2.5); // => -3
print min(4, -2); // => -2
print max(4, -2); // => 4
print sqrt(81); // => 9
print pow(2, -3); // => 0.125
print abs(sin(3.141592653589793 / 2) - 1) < 0.000000000001; // => true
print cos(0); // => 1
print tan(0); // => 0
print log(1); // => 0
print exp(0); // => 1
print is_nan(0 / 0); // => true
print is_finite(1 / 0); // => false
```

## Strict conversion

`number(x)` returns numeric inputs unchanged, including signed zero, infinity,
and NaN. A string must be a **complete finite ASCII decimal**. An optional sign,
a decimal point, and an exponent with an optional sign are supported. At least
one mantissa digit is required, and an exponent requires at least one digit.
The accepted syntax is:

`[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?`

Leading/trailing whitespace, empty strings, hex, digit separators, trailing
junk, `NaN`, `inf`, and overflow to infinity are errors. Trim whitespace explicitly
with `trim` when wanted. Underflow to signed zero is allowed; conversion rounds
to binary64 and does not guarantee decimal or large-integer precision. Booleans,
`nil`, functions, classes, and instances are not accepted.

```msa
print number("+1.25e2"); // => 125
print number(".5"); // => 0.5
print number("5."); // => 5
print number("-2E-2"); // => -0.02
print 1 / number("-0"); // => -inf
print is_nan(number(0 / 0)); // => true
```

## Assertions

`assert(condition)` or `assert(condition, message)` returns `nil` if the condition
is truthy. Mosaic treats only `false` and `nil` as falsey: zero and empty strings
pass. Failure produces `Assertion failed.` or `Assertion failed: <message>` at
the call's source line. When supplied, `message` must be a string even if the
condition is truthy. The message is evaluated eagerly, not lazily.

```msa
assert(abs(sqrt(2) * sqrt(2) - 2) < 0.000000000001, "square-root identity");
print assert(0); // => nil
print assert(""); // => nil
```

The existing `clock()` remains a zero-argument native returning seconds since the
Unix epoch from the system wall clock. It is not a monotonic timer.

## Running and testing

From the repository root:

```sh
cargo run -p interpreter -- --backend tree -e 'print sqrt(81);'
cargo test --test stdlib
cargo test --test stdlib_examples
```

The integration tests execute every `msa` code block above and verify each
`// =>` output annotation. See [the string API](stdlib-strings.md) for Unicode
text operations and resource limits.
