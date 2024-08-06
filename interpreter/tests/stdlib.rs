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

// More regression cases follow.
