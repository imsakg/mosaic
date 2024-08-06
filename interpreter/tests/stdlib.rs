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

// More regression cases follow.
