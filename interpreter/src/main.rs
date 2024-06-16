use std::{
    io::{Read, Write},
    path::PathBuf,
};

use clap::Parser;
use interpreter::{args::Cli, build::prepare_build_info, lexer::Lexer, token::Token};
use rustyline::{error::ReadlineError, DefaultEditor, Editor};
use sysexits::ExitCode;

fn run_prompt(program: Option<PathBuf>) -> Result<(), ExitCode> {
    let mut rl = DefaultEditor::new().unwrap();

    println!("Mosaic Interpreter");
    println!("{}", prepare_build_info());
    println!(r#"Type "help", "copyright", "credits" or "license" for more information."#);

    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                match line.as_str() {
                    "exit" => break,
                    "help" => {
                        println!("Help");
                    }
                    "credits" => {
                        println!("Credits");
                    }
                    "license" => {
                        println!("License");
                    }
                    "copyrigth" => {
                        println!("Copyrigth");
                    }
                    _ => {
                        println!("Line: {}", line);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt").unwrap();
    Ok(())
}

fn run_file(path: &PathBuf) -> Result<(), ExitCode> {
    let mut buf = String::new();
    let mut file = std::fs::File::open(path).unwrap();
    file.read_to_string(&mut buf).unwrap();
    let mut lexer = Lexer::new(buf);
    let tokens = lexer.scan_tokens();
    for token in tokens {
        println!("{:?}", token);
    }
    Ok(())
}

fn main() -> Result<(), ExitCode> {
    let args = Cli::parse();

    if args.interpreter {
        match args.program {
            Some(p) => run_prompt(Some(p))?,
            None => match args.program_positional {
                Some(p) => run_prompt(Some(p))?,
                None => run_prompt(None)?,
            },
        };
    } else {
        match args.program {
            Some(p) => run_file(&p)?,
            None => match args.program_positional {
                Some(p) => run_file(&p)?,
                None => run_prompt(None)?,
            },
        };
    }

    // println!("{:#?}", args);
    Ok(())
}
