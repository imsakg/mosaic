use clap::Parser;
use interpreter::{
    args::{Backend, Cli},
    session::{compile_errors_in, RunError, Session},
    vm::{compile_with_options, CompileOptions},
};
use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

fn bytecode_error(message: String) -> RunError {
    RunError {
        code: 65,
        messages: vec![message],
        output: Vec::new(),
    }
}

fn inspected(source: &str, args: &Cli) -> Result<Option<Vec<String>>, RunError> {
    if args.tokens {
        let mut lexer = interpreter::lexer::Lexer::new(source.into());
        let ts = lexer.scan_tokens();
        if !lexer.errors().is_empty() {
            return Err(compile_errors_in(lexer.errors().to_vec(), source));
        }
        return Ok(Some(
            ts.iter()
                .map(|t| format!("{} {:?} {:?}", t.line, t.token_type, t.lexeme))
                .collect(),
        ));
    }
    if args.ast || args.check || args.disassemble || args.compile_only {
        let ss = interpreter::parse(source).map_err(|errors| compile_errors_in(errors, source))?;
        if args.ast {
            return Ok(Some(vec![format!("{ss:#?}")]));
        }
        let options = CompileOptions {
            optimize: args.optimize,
        };
        if args.disassemble || args.compile_only {
            let chunk = compile_with_options(&ss, options).map_err(bytecode_error)?;
            let text = if args.compile_only {
                let stats = chunk.stats().map_err(bytecode_error)?;
                format!(
                    "{} bytes, {} instructions, {} constants, {} branches",
                    stats.byte_len,
                    stats.instruction_count,
                    stats.constant_count,
                    stats.branch_count
                )
            } else {
                chunk.disassemble().map_err(bytecode_error)?
            };
            return Ok(Some(vec![text]));
        }
        match args.backend {
            Backend::Tree => {
                interpreter::resolver::resolve(ss)
                    .map_err(|errors| compile_errors_in(errors, source))?;
            }
            Backend::Vm => {
                compile_with_options(&ss, options).map_err(bytecode_error)?;
            }
        }
        return Ok(Some(Vec::new()));
    }
    Ok(None)
}
fn app(args: Cli) -> io::Result<u8> {
    for (enabled, flag) in [
        (args.gc_stress, "--gc-stress"),
        (args.compile_only, "--compile-only"),
        (args.optimize, "--optimize"),
        (args.trace, "--trace"),
        (args.instruction_limit.is_some(), "--instruction-limit"),
    ] {
        if enabled && args.backend != Backend::Vm {
            writeln!(io::stderr(), "{flag} requires --backend vm")?;
            return Ok(64);
        }
    }
    let mut session = Session::new(args.backend, args.gc_stress);
    session.set_instruction_limit(args.instruction_limit);
    session.set_compile_options(CompileOptions {
        optimize: args.optimize,
    });
    if args.trace {
        session.set_trace_limit(args.trace_limit.unwrap_or(1024) as usize);
    }
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
        if args.trace {
            writeln!(
                io::stderr(),
                "Tracing requires a source file, - or -e source."
            )?;
            return Ok(64);
        }
        if args.tokens || args.ast || args.disassemble || args.check || args.compile_only {
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
    if args.trace {
        for event in session.trace() {
            writeln!(
                io::stderr(),
                "trace {:04} line={} {} stack={} scopes={}",
                event.offset,
                event.line,
                event.opcode.name(),
                event.stack_depth,
                event.scope_depth
            )?;
        }
        if session.trace_truncated() {
            writeln!(
                io::stderr(),
                "trace truncated after {} instructions",
                session.trace().len()
            )?;
        }
    }
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
