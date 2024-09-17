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
fn instruction_limit_interrupts_cli_loops_and_preserves_exit_codes() {
    let looped = mosaic(&[
        "--backend",
        "vm",
        "--instruction-limit",
        "19",
        "--trace",
        "--trace-limit",
        "2",
        "-e",
        "while (true) { print 1; }",
    ]);
    assert_eq!(looped.status.code(), Some(70), "{}", stderr(&looped));
    assert!(stdout(&looped).is_empty());
    assert!(stderr(&looped).contains("Instruction budget exhausted"));
    assert!(stderr(&looped).contains("trace truncated after 2 instructions"));
    let exact = mosaic(&[
        "--backend",
        "vm",
        "--instruction-limit",
        "2",
        "-e",
        "print 1;",
    ]);
    assert_eq!(exact.status.code(), Some(0));
    assert_eq!(stdout(&exact), "1\n");
    let zero = mosaic(&[
        "--backend",
        "vm",
        "--instruction-limit",
        "0",
        "-e",
        "print 1;",
    ]);
    assert_eq!(zero.status.code(), Some(70));
    let empty = mosaic(&["--backend", "vm", "--instruction-limit", "0", "-e", ""]);
    assert_eq!(empty.status.code(), Some(0));
    let unsupported = mosaic(&[
        "--backend",
        "vm",
        "--instruction-limit",
        "0",
        "-e",
        "fun f() {}",
    ]);
    assert_eq!(unsupported.status.code(), Some(65));
    for args in [
        vec!["--instruction-limit", "1", "-e", "print 1;"],
        vec![
            "--backend",
            "vm",
            "--instruction-limit",
            "1",
            "--compile-only",
            "-e",
            "print 1;",
        ],
        vec![
            "--backend",
            "vm",
            "--instruction-limit",
            "-1",
            "-e",
            "print 1;",
        ],
    ] {
        assert_eq!(mosaic(&args).status.code(), Some(64), "{args:?}");
    }
}

#[test]
fn session_instruction_limits_reset_on_each_source_unit_including_parse_errors() {
    use interpreter::{args::Backend, session::Session};
    let mut session = Session::new(Backend::Vm, true);
    session.set_instruction_limit(Some(2));
    assert_eq!(session.run("print 1;").unwrap(), ["1"]);
    assert_eq!(session.instructions_executed(), 2);
    assert_eq!(session.run("print 2;").unwrap(), ["2"]);
    assert!(session.run("print ;").is_err());
    assert_eq!(session.instructions_executed(), 0);
    assert_eq!(session.run("while (true) {}").unwrap_err().code, 70);
    assert_eq!(session.instructions_executed(), 2);
    session.set_instruction_limit(None);
    assert_eq!(session.run("print 1+2;").unwrap(), ["3"]);
}

#[test]
fn documented_stdin_source_supports_optimized_tracing() {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_interpreter"))
        .args([
            "--backend",
            "vm",
            "--trace",
            "--optimize",
            "--instruction-limit",
            "2",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"print 1+2;")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "3\n");
    assert!(stderr(&output).contains("CONSTANT"));
    assert!(!stderr(&output).contains("ADD"));
    assert!(!stderr(&output).contains("truncated"));
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
