use interpreter::tree::Interpreter;
use std::path::PathBuf;
use std::process::Command;

fn examples() -> impl Iterator<Item = PathBuf> {
    [
        "stdlib-math.msa",
        "stdlib-strings.msa",
        "stdlib-functions.msa",
    ]
    .into_iter()
    .map(|name| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../examples")
            .join(name)
    })
}

fn expected_output(source: &str) -> Vec<&str> {
    let expected: Vec<_> = source
        .lines()
        .filter_map(|line| line.split_once("// => ").map(|(_, value)| value))
        .collect();
    assert!(
        !expected.is_empty(),
        "an executable example needs output assertions"
    );
    expected
}

#[test]
fn standard_library_examples_run_through_the_interpreter_api() {
    for path in examples() {
        let source = std::fs::read_to_string(&path).expect("checked-in example source");
        let actual = Interpreter::new().run(&source).unwrap_or_else(|errors| {
            panic!("{}: {errors:?}", path.display());
        });
        assert_eq!(actual, expected_output(&source), "{}", path.display());
    }
}

#[test]
fn standard_library_examples_run_through_the_tree_cli() {
    for path in examples() {
        let source = std::fs::read_to_string(&path).expect("checked-in example source");
        let result = Command::new(env!("CARGO_BIN_EXE_interpreter"))
            .arg("--backend")
            .arg("tree")
            .arg(&path)
            .output()
            .expect("run the built Mosaic CLI");
        assert!(
            result.status.success(),
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stderr.is_empty(), "{}", path.display());
        let stdout = String::from_utf8(result.stdout).expect("UTF-8 example output");
        assert_eq!(
            stdout.lines().collect::<Vec<_>>(),
            expected_output(&source),
            "{}",
            path.display()
        );
    }
}

#[test]
fn standard_library_examples_pass_cli_check_without_executing() {
    for path in examples() {
        let result = Command::new(env!("CARGO_BIN_EXE_interpreter"))
            .arg("--backend")
            .arg("tree")
            .arg("--check")
            .arg(&path)
            .output()
            .expect("check with the built Mosaic CLI");
        assert!(
            result.status.success(),
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stdout.is_empty(), "check must not execute prints");
        assert!(result.stderr.is_empty(), "{}", path.display());
    }
}
