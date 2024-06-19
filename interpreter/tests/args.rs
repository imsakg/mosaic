use clap::Parser;
use interpreter::args::{Backend, Cli};
#[test]
fn cli_selects_backends_inline_programs_and_inspection_modes() {
    let cli =
        Cli::try_parse_from(["mosaic", "--backend", "vm", "-e", "print 1;", "--check"]).unwrap();
    assert_eq!(cli.backend, Backend::Vm);
    assert_eq!(cli.eval.as_deref(), Some("print 1;"));
    assert!(cli.check);
}
#[test]
fn mutually_exclusive_inputs_and_modes_are_rejected() {
    for args in [
        vec!["mosaic", "-e", "1;", "file.msa"],
        vec!["mosaic", "--tokens", "--ast"],
        vec!["mosaic", "--backend", "unknown"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}
