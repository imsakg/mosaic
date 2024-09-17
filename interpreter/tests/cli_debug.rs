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
