use interpreter::error::Diagnostic;
use interpreter::tree::Interpreter;

fn output(source: &str) -> Vec<String> {
    Interpreter::new()
        .run(source)
        .unwrap_or_else(|errors| panic!("source: {source}\ndiagnostics: {errors:?}"))
}

fn runtime_error(source: &str) -> Diagnostic {
    let errors = Interpreter::new().run(source).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].phase, "Runtime");
    errors.into_iter().next().unwrap()
}

#[test]
fn abs_accepts_numbers_and_preserves_ieee_values() {
    assert_eq!(
        output(
            "print abs(-12.5); print abs(0); print abs(-1 / 0); print abs(0 / 0) == abs(0 / 0);"
        ),
        ["12.5", "0", "inf", "false"]
    );
}

#[test]
fn abs_reports_arity_types_and_call_site_lines() {
    for call in ["abs()", "abs(1, 2)"] {
        let error = runtime_error(&format!("\n\n{call};"));
        assert_eq!(error.line, 3);
        assert!(error.message.contains("abs"), "{}", error.message);
        assert!(error.message.contains("Expected 1"), "{}", error.message);
    }
    let error = runtime_error("\nabs(\"1\");");
    assert_eq!(error.line, 2);
    assert!(error.message.contains("abs argument 1 must be a number"));
}

#[test]
fn floor_ceil_and_trunc_handle_positive_and_negative_fractions() {
    assert_eq!(output("print floor(2.8); print floor(-2.8); print ceil(2.2); print ceil(-2.2); print trunc(2.8); print trunc(-2.8);"),
        ["2", "-3", "3", "-2", "2", "-2"]);
    assert_eq!(
        output(
            "print 1 / trunc(-0.5); print 1 / ceil(-0.5); print floor(1 / 0); print ceil(-1 / 0);"
        ),
        ["-inf", "-inf", "inf", "-inf"]
    );
}

#[test]
fn integral_rounding_rejects_non_numbers() {
    for name in ["floor", "ceil", "trunc"] {
        let error = runtime_error(&format!("{name}(true);"));
        assert!(
            error.message.contains("argument 1 must be a number"),
            "{error}"
        );
    }
}

#[test]
fn round_breaks_halfway_ties_away_from_zero() {
    assert_eq!(output("print round(2.5); print round(-2.5); print round(2.49); print round(-2.49); print round(3); print 1 / round(-0.1);"),
        ["3", "-3", "2", "-2", "3", "-inf"]);
}

#[test]
fn round_preserves_nonfinite_numbers_and_checks_arity() {
    assert_eq!(
        output("print round(1 / 0); print round(-1 / 0); print round(0 / 0) == 0;"),
        ["inf", "-inf", "false"]
    );
    assert!(runtime_error("round();").message.contains("Expected 1"));
    assert!(runtime_error("round(nil);")
        .message
        .contains("must be a number"));
}

#[test]
fn min_and_max_compare_exactly_two_numeric_values() {
    assert_eq!(
        output("print min(4, -2); print max(4, -2); print min(3, 3); print max(-1 / 0, 1 / 0);"),
        ["-2", "4", "3", "inf"]
    );
    for source in ["min(1);", "max(1, 2, 3);"] {
        assert!(runtime_error(source).message.contains("Expected 2"));
    }
    assert!(runtime_error("max(1, false);")
        .message
        .contains("argument 2 must be a number"));
}

#[test]
fn min_and_max_propagate_nan_and_choose_signed_zero_consistently() {
    assert_eq!(output("var nan = 0 / 0; print min(nan, 1) == 1; print min(1, nan) == 1; print max(nan, 1) == 1; print max(1, nan) == 1;"),
        ["false", "false", "false", "false"]);
    assert_eq!(output("print 1 / min(0, -0); print 1 / min(-0, 0); print 1 / max(0, -0); print 1 / max(-0, 0);"),
        ["-inf", "-inf", "inf", "inf"]);
}

#[test]
fn sqrt_accepts_nonnegative_numbers_and_preserves_special_values() {
    assert_eq!(output("print sqrt(9); print sqrt(0.25); print sqrt(1 / 0); print 1 / sqrt(-0); print sqrt(0 / 0) == 0;"),
        ["3", "0.5", "inf", "-inf", "false"]);
}

#[test]
fn sqrt_rejects_negative_domain_with_a_source_line() {
    for operand in ["-1", "-1 / 0"] {
        let error = runtime_error(&format!("\n\nsqrt({operand});"));
        assert_eq!(error.line, 3);
        assert!(
            error.message.contains("sqrt requires a nonnegative number"),
            "{error}"
        );
    }
    assert!(runtime_error("sqrt(\"9\");")
        .message
        .contains("must be a number"));
}

#[test]
fn pow_supports_fractional_and_negative_exponents() {
    assert_eq!(output("print pow(2, 10); print pow(9, 0.5); print pow(2, -3); print pow(-2, 3); print pow(0, 0);"),
        ["1024", "3", "0.125", "-8", "1"]);
}

#[test]
fn pow_exposes_ieee_domains_and_validates_both_operands() {
    assert_eq!(
        output("print pow(-1, 0.5) == 0; print pow(0, -1); print pow(0 / 0, 0);"),
        ["false", "inf", "1"]
    );
    for (source, position) in [("pow(false, 2);", 1), ("pow(2, nil);", 2)] {
        assert!(runtime_error(source)
            .message
            .contains(&format!("argument {position} must be a number")));
    }
    assert!(runtime_error("pow(2);").message.contains("Expected 2"));
}

#[test]
fn trigonometry_uses_radians_with_floating_point_tolerance() {
    assert_eq!(output("var pi = 3.141592653589793; print abs(sin(pi / 2) - 1) < 0.000000000001; print abs(cos(pi) + 1) < 0.000000000001; print abs(tan(pi / 4) - 1) < 0.000000000001; print sin(0); print cos(0); print tan(0);"),
        ["true", "true", "true", "0", "1", "0"]);
}

#[test]
fn trigonometry_handles_nan_infinity_and_wrong_types() {
    for name in ["sin", "cos", "tan"] {
        assert_eq!(
            output(&format!("var x = {name}(1 / 0); print x == x;")),
            ["false"]
        );
        assert!(runtime_error(&format!("{name}(false);"))
            .message
            .contains("must be a number"));
        assert!(runtime_error(&format!("{name}();"))
            .message
            .contains("Expected 1"));
    }
}

#[test]
fn log_and_exp_are_natural_inverses() {
    assert_eq!(output("print log(1); print exp(0); print abs(log(exp(2)) - 2) < 0.000000000001; print abs(exp(log(4)) - 4) < 0.000000000001; print exp(-1 / 0); print exp(1000); print log(1 / 0);"),
        ["0", "1", "true", "true", "0", "inf", "inf"]);
    assert_eq!(
        output("var x = log(0 / 0); print x == x; var y = exp(0 / 0); print y == y;"),
        ["false", "false"]
    );
}

#[test]
fn log_requires_a_positive_domain_and_both_functions_require_numbers() {
    for operand in ["0", "-0", "-1", "-1 / 0"] {
        let error = runtime_error(&format!("\nlog({operand});"));
        assert_eq!(error.line, 2);
        assert!(
            error.message.contains("log requires a positive number"),
            "{error}"
        );
    }
    for name in ["log", "exp"] {
        assert!(runtime_error(&format!("{name}(\"1\");"))
            .message
            .contains("must be a number"));
    }
}

#[test]
fn numeric_predicates_distinguish_nan_infinity_and_finite_values() {
    assert_eq!(output("print is_nan(0 / 0); print is_nan(1 / 0); print is_nan(42); print is_finite(42); print is_finite(-0); print is_finite(1 / 0); print is_finite(-1 / 0); print is_finite(0 / 0);"),
        ["true", "false", "false", "true", "true", "false", "false", "false"]);
}

#[test]
fn numeric_predicates_are_strict_instead_of_coercing() {
    for name in ["is_nan", "is_finite"] {
        for operand in ["nil", "false", "\"NaN\""] {
            assert!(runtime_error(&format!("{name}({operand});"))
                .message
                .contains("must be a number"));
        }
        assert!(runtime_error(&format!("{name}(1, 2);"))
            .message
            .contains("Expected 1"));
    }
}

#[test]
fn number_parses_complete_decimal_strings_and_preserves_numeric_inputs() {
    assert_eq!(
        output(
            r#"print number("42"); print number("+1.25e2"); print number(".5"); print number("5."); print number("-2E-2"); print 1 / number("-0"); print number(7); print is_nan(number(0 / 0)); print number(1 / 0);"#
        ),
        ["42", "125", "0.5", "5", "-0.02", "-inf", "7", "true", "inf"]
    );
}

#[test]
fn number_rejects_whitespace_partial_and_nonfinite_string_conversions() {
    for text in [
        "", " ", " 1", "1 ", "1x", "1_000", "0x10", "NaN", "inf", "Infinity", "1e9999", "+", ".",
        "1e", "1e+", "--1", "١", "1\n", "1.2.3",
    ] {
        let error = runtime_error(&format!("\nnumber(\"{text}\");"));
        assert!(
            error
                .message
                .contains("number requires a finite decimal string"),
            "{text:?}: {error}"
        );
    }
    assert!(runtime_error("number(nil);")
        .message
        .contains("number argument 1 must be a number or string"));
    assert!(runtime_error("number();").message.contains("Expected 1"));
}

#[test]
fn trim_removes_unicode_whitespace_only_at_string_edges() {
    assert_eq!(output("print trim(\" \\t hello world \\r\\n\"); print trim(\"\u{2003}café\u{00a0}\"); print trim(\"a  b\"); print trim(\"\"); print trim(\" \\n\");"),
        ["hello world", "café", "a  b", "", ""]);
    assert_eq!(
        output("print trim(\"\u{200b}x\u{200b}\");"),
        ["\u{200b}x\u{200b}"]
    );
}

#[test]
fn trim_requires_one_string_without_implicit_conversion() {
    assert!(runtime_error("trim(1);")
        .message
        .contains("trim argument 1 must be a string"));
    assert!(runtime_error("trim();").message.contains("Expected 1"));
}

#[test]
fn lower_and_upper_apply_locale_independent_unicode_case_mapping() {
    assert_eq!(
        output(
            r#"print lower("CAFÉ Ω"); print upper("straße"); print lower("İ"); print lower("ΟΣ"); print upper("hello 🦀"); print lower(""); print upper("");"#
        ),
        ["café ω", "STRASSE", "i\u{307}", "ος", "HELLO 🦀", "", ""]
    );
}

#[test]
fn case_conversion_requires_strings_and_leaves_original_values_unchanged() {
    assert_eq!(
        output(
            r#"var original = "Straße"; var changed = upper(original); print original; print changed;"#
        ),
        ["Straße", "STRASSE"]
    );
    for name in ["lower", "upper"] {
        assert!(runtime_error(&format!("{name}(false);"))
            .message
            .contains("must be a string"));
        assert!(runtime_error(&format!("{name}(\"x\", \"y\");"))
            .message
            .contains("Expected 1"));
    }
}

#[test]
fn prefix_and_suffix_predicates_match_literal_unicode_strings() {
    assert_eq!(
        output(
            r#"print starts_with("café 🦀", "café"); print ends_with("café 🦀", "🦀"); print starts_with("Hello", "he"); print ends_with("abc", "bcx"); print starts_with("", ""); print ends_with("abc", ""); print starts_with("é", "e");"#
        ),
        ["true", "true", "false", "false", "true", "true", "false"]
    );
}

#[test]
fn prefix_and_suffix_require_two_strings() {
    for name in ["starts_with", "ends_with"] {
        assert!(runtime_error(&format!("{name}(\"a\");"))
            .message
            .contains("Expected 2"));
        assert!(runtime_error(&format!("{name}(1, \"a\");"))
            .message
            .contains("argument 1 must be a string"));
        assert!(runtime_error(&format!("{name}(\"a\", nil);"))
            .message
            .contains("argument 2 must be a string"));
    }
}

#[test]
fn contains_searches_literal_case_sensitive_substrings() {
    assert_eq!(
        output(
            r#"print contains("a.b 🦀 café", ".b"); print contains("a.b 🦀 café", "🦀"); print contains("abc", ".*"); print contains("Hello", "hello"); print contains("abc", ""); print contains("", "x"); print contains("", "");"#
        ),
        ["true", "true", "false", "false", "true", "false", "true"]
    );
}

#[test]
fn contains_checks_arity_and_each_string_argument() {
    assert!(runtime_error("contains();").message.contains("Expected 2"));
    assert!(runtime_error("contains(nil, \"x\");")
        .message
        .contains("argument 1 must be a string"));
    assert!(runtime_error("contains(\"x\", 1);")
        .message
        .contains("argument 2 must be a string"));
}

#[test]
fn replace_substitutes_all_nonoverlapping_literal_matches() {
    assert_eq!(
        output(
            r#"print replace("aaaaa", "aa", "b"); print replace("a.b.a", ".", "$1"); print replace("🦀x🦀", "🦀", "café"); print replace("abc", "z", "x"); print replace("banana", "na", "");"#
        ),
        ["bba", "a$1b$1a", "caféxcafé", "abc", "ba"]
    );
}

#[test]
fn replace_empty_pattern_uses_unicode_scalar_boundaries() {
    assert_eq!(
        output(r#"print replace("é🦀", "", "-"); print replace("", "", "x");"#),
        ["-é-🦀-", "x"]
    );
    assert!(runtime_error("replace(\"x\", \"x\");")
        .message
        .contains("Expected 3"));
    for (source, position) in [
        ("replace(1, \"x\", \"y\");", 1),
        ("replace(\"x\", nil, \"y\");", 2),
        ("replace(\"x\", \"x\", false);", 3),
    ] {
        assert!(runtime_error(source)
            .message
            .contains(&format!("argument {position} must be a string")));
    }
}

#[test]
fn repeat_accepts_nonnegative_integral_counts_and_preserves_utf8() {
    assert_eq!(
        output(
            r#"print repeat("é🦀", 2); print repeat("x", 0); print repeat("", 3); print repeat("x", -0); print repeat("", number("1e300"));"#
        ),
        ["é🦀é🦀", "", "", "", ""]
    );
}

#[test]
fn repeat_rejects_invalid_counts_before_allocating() {
    for operand in ["-1", "1.5", "1 / 0", "-1 / 0", "0 / 0"] {
        let error = runtime_error(&format!("\nrepeat(\"x\", {operand});"));
        assert_eq!(error.line, 2);
        assert!(
            error
                .message
                .contains("repeat count must be a finite nonnegative integer"),
            "{error}"
        );
    }
    assert!(runtime_error("repeat(\"x\", \"2\");")
        .message
        .contains("argument 2 must be a number"));
}

#[test]
fn repeat_enforces_a_one_mebibyte_utf8_output_limit() {
    let ascii = output("print repeat(\"a\", 1048576);");
    assert_eq!(ascii[0].len(), 1_048_576);
    let unicode = output("print repeat(\"🦀\", 262144);");
    assert_eq!(unicode[0].len(), 1_048_576);
    for call in [
        "repeat(\"a\", 1048577)",
        "repeat(\"🦀\", 262145)",
        "repeat(\"a\", number(\"1e300\"))",
    ] {
        assert!(runtime_error(&format!("{call};"))
            .message
            .contains("exceeds 1048576 UTF-8 bytes"));
    }
}

#[test]
fn char_at_indexes_unicode_scalars_not_utf8_bytes_or_graphemes() {
    assert_eq!(
        output(
            r#"print char_at("aé🦀", 0); print char_at("aé🦀", 1); print char_at("aé🦀", 2); print char_at("aé🦀", 3); print char_at("é", 1); print char_at("", 0); print char_at("x", number("1e300")); print char_at("x", -0);"#
        ),
        ["a", "é", "🦀", "nil", "\u{301}", "nil", "nil", "x"]
    );
}

#[test]
fn char_at_rejects_invalid_indices_and_argument_types() {
    for operand in ["-1", "0.5", "1 / 0", "-1 / 0", "0 / 0"] {
        let error = runtime_error(&format!("\nchar_at(\"é\", {operand});"));
        assert_eq!(error.line, 2);
        assert!(
            error
                .message
                .contains("char_at index must be a finite nonnegative integer"),
            "{error}"
        );
    }
    assert!(runtime_error("char_at(nil, 0);")
        .message
        .contains("argument 1 must be a string"));
    assert!(runtime_error("char_at(\"a\", false);")
        .message
        .contains("argument 2 must be a number"));
    assert!(runtime_error("char_at(\"a\");")
        .message
        .contains("Expected 2"));
}

#[test]
fn assert_returns_nil_and_uses_mosaic_truthiness() {
    assert_eq!(
        output(
            r#"print assert(true); print assert(0); print assert(""); print assert("ok", "unused");"#
        ),
        ["nil", "nil", "nil", "nil"]
    );
}

#[test]
fn assert_failures_report_optional_messages_at_the_call_site() {
    for condition in ["false", "nil"] {
        let error = runtime_error(&format!("\n\nassert({condition});"));
        assert_eq!(error.line, 3);
        assert_eq!(error.message, "Assertion failed.");
    }
    let error = runtime_error("\nassert(false, \"expected café 🦀\");");
    assert_eq!(error.line, 2);
    assert_eq!(error.message, "Assertion failed: expected café 🦀");
}

#[test]
fn assert_validates_optional_message_even_when_condition_is_truthy() {
    for condition in ["true", "false"] {
        assert!(runtime_error(&format!("assert({condition}, 42);"))
            .message
            .contains("assert argument 2 must be a string"));
    }
    for source in ["assert();", "assert(true, \"x\", nil);"] {
        assert!(runtime_error(source).message.contains("Expected 1 or 2"));
    }
}

// More regression cases follow.
