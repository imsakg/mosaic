use clap::{Arg, ArgMatches, Command, Parser};

#[derive(Parser, Debug)]
#[clap(name = "Mosaic Interpreter")]
#[clap(about = "A interpreter for mosaic language.", long_about = None)]
#[command(about, version, author)]
pub struct Cli {
    #[clap(name = "program", short = 'p', long = "prog")]
    pub program: Option<std::path::PathBuf>,
    #[clap(index = 1)]
    pub program_positional: Option<std::path::PathBuf>,
    #[clap(name = "interpreter", short = 'i', long = "interpreter")]
    pub interpreter: bool,
}
