use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    Tree,
    Vm,
}

/// An educational Lox-style language in Rust.
#[derive(Parser, Debug)]
#[command(name="mosaic",version,about,group(clap::ArgGroup::new("inspection").args(["tokens","ast","disassemble","check"]).multiple(false)))]
pub struct Cli {
    /// Source file, or '-' to read a complete program from stdin
    #[arg(conflicts_with = "eval")]
    pub program: Option<PathBuf>,
    /// Evaluate inline source instead of opening the REPL
    #[arg(short = 'e', long, conflicts_with = "program")]
    pub eval: Option<String>,
    #[arg(long, value_enum, default_value = "tree")]
    pub backend: Backend,
    /// Print scanned tokens without executing
    #[arg(long)]
    pub tokens: bool,
    /// Print the parsed syntax tree without executing
    #[arg(long)]
    pub ast: bool,
    /// Print byte offsets, opcodes and constants without executing
    #[arg(long)]
    pub disassemble: bool,
    /// Validate the selected backend without executing
    #[arg(long)]
    pub check: bool,
    /// Collect before every VM string intern operation
    #[arg(long)]
    pub gc_stress: bool,
}
