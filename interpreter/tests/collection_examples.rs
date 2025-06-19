use interpreter::tree::Interpreter;
use std::process::Command;

const EXAMPLE: &str = include_str!("../../examples/collections.msa");
const EXPECTED: &[&str] = &[
    "(apple, pear)",
    "(1, 2)",
    "((apple, 1), (pear, 2))",
    "pear|apple|pear",
    "é🦀",
    "map",
];

#[test]
fn collection_example_runs_in_the_interpreter_and_cli() {
    assert_eq!(Interpreter::new().run(EXAMPLE).unwrap(), EXPECTED);
    let output = Command::new(env!("CARGO_BIN_EXE_interpreter"))
        .args(["-e", EXAMPLE])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        EXPECTED
    );
}

#[test]
fn vm_reports_unsupported_collection_calls_instead_of_falling_back() {
    for flags in [
        vec!["--backend", "vm"],
        vec!["--backend", "vm", "--optimize"],
        vec!["--backend", "vm", "--check"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_interpreter"))
            .args(flags)
            .args(["-e", EXAMPLE])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(65));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("Calls are not supported"));
    }
}
