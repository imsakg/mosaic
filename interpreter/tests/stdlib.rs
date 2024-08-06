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

// More regression cases follow.
