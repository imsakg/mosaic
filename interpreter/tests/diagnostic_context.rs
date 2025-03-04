use interpreter::{args::Backend, session::Session};
use std::process::Command;

#[test]
fn session_diagnostics_include_the_source_line_and_prior_output() {
    let error = Session::new(Backend::Tree, false)
        .run("print 1;\nprint missing;")
        .unwrap_err();
    assert_eq!(error.code, 70);
    assert_eq!(error.output, ["1"]);
    assert!(error.messages[0].contains("\n   2 | print missing;"));
    for backend in [Backend::Tree, Backend::Vm] {
        let error = Session::new(backend, false).run("var x = ;").unwrap_err();
        assert_eq!(error.code, 65);
        assert!(error.messages[0].contains("\n   1 | var x = ;"));
    }
}

#[test]
fn check_mode_renders_crlf_and_unicode_source_without_ansi_codes() {
    let output = Command::new(env!("CARGO_BIN_EXE_interpreter"))
        .args(["--check", "-e", "// é\r\nvar x = ;\r\n"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(65));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("   2 | var x = ;"), "{stderr}");
    assert!(!stderr.contains('\r'));
    assert!(!stderr.contains('\u{1b}'));
}
