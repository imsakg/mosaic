use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    Tree,
    Vm,
}

/// An educational Lox-style language in Rust.
#[derive(Parser, Debug)]
#[command(name="mosaic",version,about,group(clap::ArgGroup::new("inspection").args(["tokens","ast","disassemble","check","compile_only"]).multiple(false)))]
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
    /// Compile VM bytecode and print statistics without executing
    #[arg(long)]
    pub compile_only: bool,
    /// Enable literal folding and constant pooling (VM only)
    #[arg(long)]
    pub optimize: bool,
    /// Trace VM instructions to stderr (requires an explicit source)
    #[arg(long, conflicts_with = "inspection")]
    pub trace: bool,
    /// Maximum trace events to retain (default 1024; range 1..=1000000)
    #[arg(long, requires = "trace", value_parser = clap::value_parser!(u32).range(1..=1_000_000))]
    pub trace_limit: Option<u32>,
    /// Collect before every VM string intern operation
    #[arg(long)]
    pub gc_stress: bool,
}
