use clap::Parser;
use interpreter::{
    args::{Backend, Cli},
    session::{compile_errors, RunError, Session},
};
use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

fn inspected(source: &str, args: &Cli) -> Result<Option<Vec<String>>, RunError> {
    if args.tokens {
        let mut lexer = interpreter::lexer::Lexer::new(source.into());
        let ts = lexer.scan_tokens();
        if !lexer.errors().is_empty() {
            return Err(compile_errors(lexer.errors().to_vec()));
        }
        return Ok(Some(
            ts.iter()
                .map(|t| format!("{} {:?} {:?}", t.line, t.token_type, t.lexeme))
                .collect(),
        ));
    }
    if args.ast || args.check || args.disassemble {
        let ss = interpreter::parse(source).map_err(compile_errors)?;
        if args.ast {
            return Ok(Some(vec![format!("{ss:#?}")]));
        }
        if args.disassemble {
            return interpreter::vm::disassemble(&ss)
                .map(|s| Some(vec![s]))
                .map_err(|m| RunError {
                    code: 65,
                    messages: vec![m],
                    output: Vec::new(),
                });
        }
        match args.backend {
            Backend::Tree => {
                interpreter::resolver::resolve(ss).map_err(compile_errors)?;
            }
            Backend::Vm => {
                interpreter::vm::compile(&ss).map_err(|m| RunError {
                    code: 65,
                    messages: vec![m],
                    output: Vec::new(),
                })?;
            }
        }
        return Ok(Some(Vec::new()));
    }
    Ok(None)
}
fn app(args: Cli) -> io::Result<u8> {
    if args.gc_stress && args.backend != Backend::Vm {
        writeln!(io::stderr(), "--gc-stress requires --backend vm")?;
        return Ok(64);
    }
    let mut session = Session::new(args.backend, args.gc_stress);
    let source = if let Some(source) = args.eval.as_ref() {
        Some(source.clone())
    } else if let Some(path) = args.program.as_ref() {
        Some(if path.as_os_str() == "-" {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            source
        } else {
            std::fs::read_to_string(path)?
        })
    } else {
        None
    };
    let Some(source) = source else {
        if args.tokens || args.ast || args.disassemble || args.check {
            writeln!(
                io::stderr(),
                "Inspection requires a source file, - or -e source."
            )?;
            return Ok(64);
        }
        let prompt = io::stdin().is_terminal();
        if prompt {
            writeln!(
                io::stdout(),
                "Mosaic {} — :help for commands",
                env!("CARGO_PKG_VERSION")
            )?;
        }
        interpreter::repl::run(
            io::stdin().lock(),
            io::stdout().lock(),
            io::stderr().lock(),
            &mut session,
            prompt,
        )?;
        return Ok(0);
    };
    let result = match inspected(&source, &args) {
        Ok(Some(lines)) => Ok(lines),
        Ok(None) => session.run(&source),
        Err(e) => Err(e),
    };
    match result {
        Ok(lines) => {
            for line in lines {
                writeln!(io::stdout(), "{line}")?;
            }
            Ok(0)
        }
        Err(e) => {
            for line in e.output {
                writeln!(io::stdout(), "{line}")?;
            }
            for line in e.messages {
                writeln!(io::stderr(), "{line}")?;
            }
            Ok(e.code)
        }
    }
}
fn main() -> ExitCode {
    let args = match Cli::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let code = if error.use_stderr() { 64 } else { 0 };
            let _ = error.print();
            return ExitCode::from(code);
        }
    };
    match app(args) {
        Ok(code) => ExitCode::from(code),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("I/O error: {e}");
            ExitCode::from(74)
        }
    }
}
