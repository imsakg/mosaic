//! Backend-neutral persistent execution state for CLI and embedding.
use crate::{args::Backend, tree::Interpreter, vm::Vm};

#[derive(Debug)]
pub struct RunError {
    pub code: u8,
    pub messages: Vec<String>,
    pub output: Vec<String>,
}
pub struct Session {
    backend: Backend,
    tree: Interpreter,
    vm: Vm,
}
impl Session {
    pub fn new(backend: Backend, gc_stress: bool) -> Self {
        let mut vm = Vm::new();
        vm.set_gc_stress(gc_stress);
        Self {
            backend,
            tree: Interpreter::new(),
            vm,
        }
    }
    pub fn set_compile_options(&mut self, options: crate::vm::CompileOptions) {
        self.vm.set_compile_options(options);
    }
    pub fn set_trace_limit(&mut self, limit: usize) {
        self.vm.set_trace_limit(limit);
    }
    pub fn trace(&self) -> &[crate::vm::TraceEvent] {
        self.vm.trace()
    }
    pub fn trace_truncated(&self) -> bool {
        self.vm.trace_truncated()
    }
    pub fn set_instruction_limit(&mut self, limit: Option<u64>) {
        self.vm.set_instruction_limit(limit);
    }
    pub fn instructions_executed(&self) -> u64 {
        self.vm.instructions_executed()
    }
    pub fn run(&mut self, source: &str) -> Result<Vec<String>, RunError> {
        self.vm.clear_debug_state();
        match self.backend {
            Backend::Tree => self.tree.run(source).map_err(|errors| RunError {
                code: if errors.iter().any(|e| e.phase == "Runtime") {
                    70
                } else {
                    65
                },
                messages: errors.iter().map(ToString::to_string).collect(),
                output: self.tree.output().to_vec(),
            }),
            Backend::Vm => {
                let statements = crate::parse(source).map_err(compile_errors)?;
                self.vm.run(&statements).map_err(|message| RunError {
                    code: if message.contains("Compile error:") {
                        65
                    } else {
                        70
                    },
                    messages: vec![message],
                    output: Vec::new(),
                })
            }
        }
    }
    pub fn stats(&self) -> String {
        match self.backend {
            Backend::Tree => format!("tree heap: {:?}", self.tree.heap_stats()),
            Backend::Vm => format!("VM strings: {:?}", self.vm.gc_stats()),
        }
    }
}
pub fn compile_errors(errors: Vec<crate::error::Diagnostic>) -> RunError {
    RunError {
        code: 65,
        messages: errors.iter().map(ToString::to_string).collect(),
        output: Vec::new(),
    }
}
