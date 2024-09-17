use std::process::{Command, Output};

fn mosaic(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_interpreter"))
        .args(args)
        .output()
        .unwrap()
}
fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}
fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn cli_traces_are_bounded_on_stderr_and_include_runtime_failures() {
    let output = mosaic(&[
        "--backend",
        "vm",
        "--trace",
        "--trace-limit",
        "2",
        "-e",
        "print 1+2;",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "3\n");
    let trace = stderr(&output);
    assert!(
        trace.contains("trace 0000 line=1 CONSTANT stack=0 scopes=0"),
        "{trace}"
    );
    assert_eq!(
        trace
            .lines()
            .filter(|line| line.starts_with("trace 0"))
            .count(),
        2
    );
    assert!(trace.contains("trace truncated after 2 instructions"));
    let failure = mosaic(&["--backend", "vm", "--trace", "-e", "print missing;"]);
    assert_eq!(failure.status.code(), Some(70));
    assert!(stdout(&failure).is_empty());
    assert!(stderr(&failure).contains("GET_NAME"));
    assert!(stderr(&failure).contains("Undefined variable"));
    let optimized = mosaic(&[
        "--backend",
        "vm",
        "--trace",
        "--optimize",
        "-e",
        "print 1+2;",
    ]);
    assert!(!stderr(&optimized).contains("ADD"));
    assert!(!stderr(&optimized).contains("truncated"));
}

#[test]
fn trace_flags_require_vm_execution_a_source_and_a_bounded_positive_limit() {
    for args in [
        vec!["--trace", "-e", "print 1;"],
        vec!["--backend", "vm", "--trace"],
        vec!["--backend", "vm", "--trace-limit", "2", "-e", "print 1;"],
        vec![
            "--backend",
            "vm",
            "--trace",
            "--trace-limit",
            "0",
            "-e",
            "print 1;",
        ],
        vec![
            "--backend",
            "vm",
            "--trace",
            "--trace-limit",
            "1000001",
            "-e",
            "print 1;",
        ],
        vec!["--backend", "vm", "--trace", "--check", "-e", "print 1;"],
    ] {
        assert_eq!(mosaic(&args).status.code(), Some(64), "{args:?}");
    }
}

#[test]
fn session_parse_errors_clear_traces_from_previous_runs() {
    use interpreter::{args::Backend, session::Session};
    let mut session = Session::new(Backend::Vm, false);
    session.set_trace_limit(8);
    assert_eq!(session.run("print 1;").unwrap(), ["1"]);
    assert_eq!(session.trace().len(), 2);
    assert!(session.run("print ;").is_err());
    assert!(session.trace().is_empty());
    assert!(!session.trace_truncated());
}

#[test]
fn compile_only_reports_bytecode_without_executing_or_resolving_globals() {
    let output = mosaic(&["--backend", "vm", "--compile-only", "-e", "print missing;"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "6 bytes, 2 instructions, 1 constants, 0 branches\n"
    );
    assert!(stderr(&output).is_empty());
    let unsupported = mosaic(&["--backend", "vm", "--compile-only", "-e", "fun f() {}"]);
    assert_eq!(unsupported.status.code(), Some(65));
    assert!(stderr(&unsupported).contains("Functions are not supported"));
    for args in [
        vec!["--compile-only", "-e", "print 1;"],
        vec!["--backend", "vm", "--compile-only"],
        vec![
            "--backend",
            "vm",
            "--compile-only",
            "--ast",
            "-e",
            "print 1;",
        ],
    ] {
        assert_eq!(mosaic(&args).status.code(), Some(64));
    }
}

#[test]
fn optimize_flag_applies_to_compilation_execution_and_disassembly_only_when_requested() {
    let original = mosaic(&["--backend", "vm", "--compile-only", "-e", "print 1+2;"]);
    let optimized = mosaic(&[
        "--backend",
        "vm",
        "--compile-only",
        "--optimize",
        "-e",
        "print 1+2;",
    ]);
    assert_eq!(
        stdout(&original),
        "12 bytes, 4 instructions, 2 constants, 0 branches\n"
    );
    assert_eq!(
        stdout(&optimized),
        "6 bytes, 2 instructions, 1 constants, 0 branches\n"
    );
    let listing = mosaic(&[
        "--backend",
        "vm",
        "--disassemble",
        "--optimize",
        "-e",
        "print 1+2;",
    ]);
    assert_eq!(listing.status.code(), Some(0));
    assert!(!stdout(&listing).contains("ADD"));
    let execution = mosaic(&["--backend", "vm", "--optimize", "-e", "print 1+2;"]);
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(stdout(&execution), "3\n");
    assert!(stderr(&execution).is_empty());
    assert_eq!(
        mosaic(&["--optimize", "-e", "print 1;"]).status.code(),
        Some(64)
    );
}
