use std::process::Command;
#[test]
fn recursive_expression_frames_produce_a_diagnostic_not_a_host_abort() {
    let code = format!("fun f(){{return {}f();}} f();", "!".repeat(32));
    let out = Command::new(env!("CARGO_BIN_EXE_interpreter"))
        .args(["-e", &code])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(70),
        "{:?}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("depth limit"));
}
