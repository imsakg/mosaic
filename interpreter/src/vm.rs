//! AST-to-bytecode compiler and stack VM. Execution never walks the AST.
//!
//! Opcodes are single bytes; constant indices and absolute byte-offset jump
//! targets are four-byte little-endian operands. Constants are immutable compiler
//! data, not runtime heap objects. All runtime strings and environment keys are
//! GC handles. Only `nil` and `false` are falsey; division follows IEEE 754.

use crate::ast::{Expr, ExprKind, Literal, Op, Stmt, StmtKind};
use crate::gc::{GcStats, Handle, Heap};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

macro_rules! opcodes {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u8)]
        pub enum OpCode { $($variant),+ }
        impl OpCode {
            fn from_byte(byte: u8) -> Option<Self> {
                match byte {
                    $(x if x == Self::$variant as u8 => Some(Self::$variant),)+
                    _ => None,
                }
            }
            pub fn name(self) -> &'static str {
                match self { $(Self::$variant => $name,)+ }
            }
        }
    };
}

opcodes! {
    Constant => "CONSTANT", Nil => "NIL", True => "TRUE", False => "FALSE",
    Pop => "POP", GetName => "GET_NAME", DefineName => "DEFINE_NAME", SetName => "SET_NAME",
    EnterScope => "ENTER_SCOPE", ExitScope => "EXIT_SCOPE",
    Add => "ADD", Subtract => "SUBTRACT", Multiply => "MULTIPLY", Divide => "DIVIDE",
    Not => "NOT", Negate => "NEGATE", Equal => "EQUAL", NotEqual => "NOT_EQUAL",
    Greater => "GREATER", GreaterEqual => "GREATER_EQUAL", Less => "LESS", LessEqual => "LESS_EQUAL",
    Print => "PRINT", Jump => "JUMP", JumpIfFalse => "JUMP_IF_FALSE",
}

impl OpCode {
    fn has_operand(self) -> bool {
        matches!(
            self,
            Self::Constant
                | Self::GetName
                | Self::DefineName
                | Self::SetName
                | Self::Jump
                | Self::JumpIfFalse
        )
    }
}

/// A compiled program owns no AST nodes and no runtime GC handles.
#[derive(Clone, Debug, Default)]
pub struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: Vec<Literal>,
}

/// One decoded instruction; offsets are byte offsets, not instruction indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub offset: usize,
    pub next_offset: usize,
    pub line: usize,
    pub opcode: OpCode,
    pub operand: Option<usize>,
}

pub struct Instructions<'a> {
    chunk: &'a Chunk,
    offset: usize,
}

impl Iterator for Instructions<'_> {
    type Item = Result<Instruction, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.chunk.code.len() {
            return None;
        }
        let offset = self.offset;
        Some(match self.chunk.decode(offset) {
            Ok((opcode, operand, next_offset)) => {
                self.offset = next_offset;
                Ok(Instruction {
                    offset,
                    next_offset,
                    opcode,
                    line: self.chunk.lines.get(offset).copied().unwrap_or(0),
                    operand: opcode.has_operand().then_some(operand),
                })
            }
            Err(error) => {
                self.offset = self.chunk.code.len();
                Err(error)
            }
        })
    }
}

impl std::iter::FusedIterator for Instructions<'_> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChunkStats {
    pub byte_len: usize,
    pub constant_count: usize,
    pub instruction_count: usize,
    pub branch_count: usize,
}

impl Chunk {
    /// Construct immutable bytecode after structural validation. This checks
    /// encoding, metadata, constants and jumps, not stack or type correctness.
    pub fn from_parts(
        code: Vec<u8>,
        lines: Vec<usize>,
        constants: Vec<Literal>,
    ) -> Result<Self, String> {
        let chunk = Self {
            code,
            lines,
            constants,
        };
        chunk.validate()?;
        Ok(chunk)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.lines.len() != self.code.len() {
            return Err(runtime_error(
                0,
                "Source map length does not match bytecode.",
            ));
        }
        let instructions = self.instructions().collect::<Result<Vec<_>, _>>()?;
        let boundaries: HashSet<_> = instructions
            .iter()
            .map(|i| i.offset)
            .chain(std::iter::once(self.code.len()))
            .collect();
        for instruction in instructions {
            let Some(operand) = instruction.operand else {
                continue;
            };
            match instruction.opcode {
                OpCode::Jump | OpCode::JumpIfFalse => {
                    if !boundaries.contains(&operand) {
                        return Err(runtime_error(
                            instruction.line,
                            "Jump target is not an instruction boundary.",
                        ));
                    }
                }
                OpCode::Constant => {
                    if self.constants.get(operand).is_none() {
                        return Err(runtime_error(instruction.line, "Invalid constant index."));
                    }
                }
                _ => {
                    if !matches!(self.constants.get(operand), Some(Literal::String(_))) {
                        return Err(runtime_error(instruction.line, "Invalid name constant."));
                    }
                }
            }
        }
        Ok(())
    }

    /// Source line for any byte, including operand bytes; None past the chunk.
    pub fn line_at(&self, offset: usize) -> Option<usize> {
        self.lines.get(offset).copied()
    }

    /// Instruction starts attributed to a source line (not operand byte offsets).
    pub fn offsets_for_line(&self, line: usize) -> Result<Vec<usize>, String> {
        let mut offsets = Vec::new();
        for instruction in self.instructions() {
            let instruction = instruction?;
            if instruction.line == line {
                offsets.push(instruction.offset);
            }
        }
        Ok(offsets)
    }

    pub fn instructions(&self) -> Instructions<'_> {
        Instructions {
            chunk: self,
            offset: 0,
        }
    }

    pub fn stats(&self) -> Result<ChunkStats, String> {
        let mut stats = ChunkStats {
            byte_len: self.code.len(),
            constant_count: self.constants.len(),
            ..ChunkStats::default()
        };
        for instruction in self.instructions() {
            let instruction = instruction?;
            stats.instruction_count += 1;
            stats.branch_count += usize::from(matches!(
                instruction.opcode,
                OpCode::Jump | OpCode::JumpIfFalse
            ));
        }
        Ok(stats)
    }

    pub fn code(&self) -> &[u8] {
        &self.code
    }
    pub fn constants(&self) -> &[Literal] {
        &self.constants
    }

    pub fn disassemble(&self) -> Result<String, String> {
        self.validate()?;
        let mut result = String::new();
        let mut offset = 0;
        while offset < self.code.len() {
            let (opcode, operand, next) = self.decode(offset)?;
            let line = self.lines[offset];
            // Writing into a String cannot fail.
            let _ = write!(result, "{offset:04} {line:4} {}", opcode.name());
            if opcode.has_operand() {
                if matches!(opcode, OpCode::Jump | OpCode::JumpIfFalse) {
                    let _ = write!(result, " -> {operand:04}");
                } else {
                    let value = self
                        .constants
                        .get(operand)
                        .ok_or_else(|| runtime_error(line, "Invalid constant index."))?;
                    let _ = write!(result, " {operand} {value:?}");
                }
            }
            result.push('\n');
            offset = next;
        }
        Ok(result)
    }

    fn decode(&self, offset: usize) -> Result<(OpCode, usize, usize), String> {
        let line = self.lines.get(offset).copied().unwrap_or(0);
        let byte = self
            .code
            .get(offset)
            .copied()
            .ok_or_else(|| runtime_error(line, "Missing opcode."))?;
        let opcode =
            OpCode::from_byte(byte).ok_or_else(|| runtime_error(line, "Unknown opcode."))?;
        if opcode.has_operand() {
            let bytes = self
                .code
                .get(offset + 1..offset + 5)
                .ok_or_else(|| runtime_error(line, "Truncated instruction."))?;
            let operand = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            Ok((opcode, operand, offset + 5))
        } else {
            Ok((opcode, 0, offset + 1))
        }
    }
}

fn compile_error(line: usize, message: &str) -> String {
    format!("[line {line}] Compile error: {message}")
}
fn runtime_error(line: usize, message: &str) -> String {
    format!("[line {line}] Runtime error: {message}")
}

const MAX_FOLDED_STRING_BYTES: usize = 4096;

// Only recursively literal expressions may be folded. Returning None leaves
// both compile-time validation and runtime type errors on their original path.
fn fold_literal(expression: &Expr) -> Option<Literal> {
    match &expression.kind {
        ExprKind::Literal(value) => Some(value.clone()),
        ExprKind::Binary(left, operator, right) => {
            let (left, right) = (fold_literal(left)?, fold_literal(right)?);
            match operator {
                Op::Equal => return Some(Literal::Bool(left == right)),
                Op::NotEqual => return Some(Literal::Bool(left != right)),
                Op::Add => {
                    if let (Literal::String(left), Literal::String(right)) = (&left, &right) {
                        if left.len().checked_add(right.len())? > MAX_FOLDED_STRING_BYTES {
                            return None;
                        }
                        return Some(Literal::String(format!("{left}{right}")));
                    }
                }
                _ => {}
            }
            let (Literal::Number(left), Literal::Number(right)) = (left, right) else {
                return None;
            };
            Some(match operator {
                Op::Add => Literal::Number(left + right),
                Op::Subtract => Literal::Number(left - right),
                Op::Multiply => Literal::Number(left * right),
                Op::Divide => Literal::Number(left / right),
                Op::Greater => Literal::Bool(left > right),
                Op::GreaterEqual => Literal::Bool(left >= right),
                Op::Less => Literal::Bool(left < right),
                Op::LessEqual => Literal::Bool(left <= right),
                _ => return None,
            })
        }
        ExprKind::Logical(left, operator, right) => {
            // Inspect both sides even when the left operand would short-circuit.
            // Unsupported/invalid syntax must still reach normal compilation.
            let (left, right) = (fold_literal(left)?, fold_literal(right)?);
            let truthy = !matches!(left, Literal::Nil | Literal::Bool(false));
            match operator {
                Op::And => Some(if truthy { right } else { left }),
                Op::Or => Some(if truthy { left } else { right }),
                _ => None,
            }
        }
        ExprKind::Unary(operator, operand) => {
            let value = fold_literal(operand)?;
            match (operator, value) {
                (Op::Not, value) => Some(Literal::Bool(matches!(
                    value,
                    Literal::Nil | Literal::Bool(false)
                ))),
                (Op::Negate, Literal::Number(value)) => Some(Literal::Number(-value)),
                _ => None,
            }
        }
        _ => None,
    }
}

#[derive(Debug)]
struct Local {
    name: String,
    depth: usize,
    initialized: bool,
}

/// Optimization is opt-in to preserve instruction-level debugging and GC behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompileOptions {
    pub optimize: bool,
}

#[derive(Default)]
struct Compiler {
    chunk: Chunk,
    depth: usize,
    locals: Vec<Local>,
    options: CompileOptions,
    numbers: HashMap<u64, usize>,
    strings: HashMap<String, usize>,
}

/// Compile the complete program before any execution or global mutation.
pub fn compile(statements: &[Stmt]) -> Result<Chunk, String> {
    compile_with_options(statements, CompileOptions::default())
}

pub fn compile_with_options(statements: &[Stmt], options: CompileOptions) -> Result<Chunk, String> {
    let mut compiler = Compiler {
        options,
        ..Compiler::default()
    };
    for statement in statements {
        compiler.statement(statement)?;
    }
    compiler.chunk.validate()?;
    Ok(compiler.chunk)
}

pub fn disassemble(statements: &[Stmt]) -> Result<String, String> {
    compile(statements)?.disassemble()
}

impl Compiler {
    fn emit(&mut self, opcode: OpCode, line: usize) -> usize {
        let offset = self.chunk.code.len();
        self.chunk.code.push(opcode as u8);
        self.chunk.lines.push(line);
        offset
    }

    fn operand(&mut self, opcode: OpCode, operand: usize, line: usize) -> Result<usize, String> {
        let operand = u32::try_from(operand)
            .map_err(|_| compile_error(line, "Program exceeds bytecode operand limit."))?;
        let offset = self.emit(opcode, line);
        self.chunk.code.extend_from_slice(&operand.to_le_bytes());
        self.chunk.lines.extend_from_slice(&[line; 4]);
        Ok(offset)
    }

    fn constant(&mut self, value: Literal, line: usize) -> Result<usize, String> {
        if self.options.optimize {
            let existing = match &value {
                Literal::Number(number) => self.numbers.get(&number.to_bits()),
                Literal::String(text) => self.strings.get(text),
                _ => None,
            };
            if let Some(index) = existing {
                return Ok(*index);
            }
        }
        let index = self.chunk.constants.len();
        u32::try_from(index).map_err(|_| compile_error(line, "Too many constants."))?;
        if self.options.optimize {
            match &value {
                Literal::Number(number) => {
                    self.numbers.insert(number.to_bits(), index);
                }
                Literal::String(text) => {
                    self.strings.insert(text.clone(), index);
                }
                _ => {}
            }
        }
        self.chunk.constants.push(value);
        Ok(index)
    }

    fn named(&mut self, opcode: OpCode, name: &str, line: usize) -> Result<(), String> {
        let index = self.constant(Literal::String(name.to_owned()), line)?;
        self.operand(opcode, index, line)?;
        Ok(())
    }

    fn patch_jump(&mut self, offset: usize, line: usize) -> Result<(), String> {
        let target = u32::try_from(self.chunk.code.len())
            .map_err(|_| compile_error(line, "Jump target exceeds bytecode limit."))?;
        self.chunk.code[offset + 1..offset + 5].copy_from_slice(&target.to_le_bytes());
        Ok(())
    }

    fn check_initialized(&self, name: &str, line: usize) -> Result<(), String> {
        if self
            .locals
            .iter()
            .rev()
            .find(|local| local.name == name)
            .is_some_and(|local| !local.initialized)
        {
            return Err(compile_error(
                line,
                "Cannot read or assign a local variable in its own initializer.",
            ));
        }
        Ok(())
    }

    fn statement(&mut self, statement: &Stmt) -> Result<(), String> {
        let line = statement.line;
        match &statement.kind {
            StmtKind::Expression(value) | StmtKind::Print(value) => {
                self.expression(value)?;
                self.emit(
                    if matches!(&statement.kind, StmtKind::Print(_)) {
                        OpCode::Print
                    } else {
                        OpCode::Pop
                    },
                    line,
                );
            }
            StmtKind::Var(name, initializer) => {
                if self.depth > 0 {
                    if self
                        .locals
                        .iter()
                        .rev()
                        .take_while(|local| local.depth == self.depth)
                        .any(|local| local.name == *name)
                    {
                        return Err(compile_error(
                            line,
                            &format!("Variable '{name}' already declared in this scope."),
                        ));
                    }
                    self.locals.push(Local {
                        name: name.clone(),
                        depth: self.depth,
                        initialized: false,
                    });
                }
                if let Some(value) = initializer {
                    self.expression(value)?;
                } else {
                    self.emit(OpCode::Nil, line);
                }
                self.named(OpCode::DefineName, name, line)?;
                if self.depth > 0 {
                    if let Some(local) = self.locals.last_mut() {
                        local.initialized = true;
                    }
                }
            }
            StmtKind::Block(statements) => {
                self.depth += 1;
                self.emit(OpCode::EnterScope, line);
                for statement in statements {
                    self.statement(statement)?;
                }
                self.locals.retain(|local| local.depth < self.depth);
                self.depth -= 1;
                self.emit(OpCode::ExitScope, line);
            }
            StmtKind::If(condition, then_branch, else_branch) => {
                self.expression(condition)?;
                let otherwise = self.operand(OpCode::JumpIfFalse, 0, line)?;
                self.emit(OpCode::Pop, line);
                self.statement(then_branch)?;
                let end = self.operand(OpCode::Jump, 0, line)?;
                self.patch_jump(otherwise, line)?;
                self.emit(OpCode::Pop, line);
                if let Some(branch) = else_branch {
                    self.statement(branch)?;
                }
                self.patch_jump(end, line)?;
            }
            StmtKind::While(condition, body) => {
                let start = self.chunk.code.len();
                self.expression(condition)?;
                let exit = self.operand(OpCode::JumpIfFalse, 0, line)?;
                self.emit(OpCode::Pop, line);
                self.statement(body)?;
                self.operand(OpCode::Jump, start, line)?;
                self.patch_jump(exit, line)?;
                self.emit(OpCode::Pop, line);
            }
            StmtKind::Function(_) => {
                return Err(compile_error(
                    line,
                    "Functions are not supported by the bytecode backend.",
                ))
            }
            StmtKind::Return(_) => {
                return Err(compile_error(
                    line,
                    "Return is not supported by the bytecode backend.",
                ))
            }
            StmtKind::Class(_, _, _) => {
                return Err(compile_error(
                    line,
                    "Classes are not supported by the bytecode backend.",
                ))
            }
        }
        Ok(())
    }

    fn literal(&mut self, value: &Literal, line: usize) -> Result<(), String> {
        match value {
            Literal::Nil => {
                self.emit(OpCode::Nil, line);
            }
            Literal::Bool(value) => {
                self.emit(if *value { OpCode::True } else { OpCode::False }, line);
            }
            value => {
                let index = self.constant(value.clone(), line)?;
                self.operand(OpCode::Constant, index, line)?;
            }
        }
        Ok(())
    }

    fn expression(&mut self, expression: &Expr) -> Result<(), String> {
        let line = expression.line;
        if self.options.optimize {
            if let Some(value) = fold_literal(expression) {
                return self.literal(&value, line);
            }
        }
        match &expression.kind {
            ExprKind::Literal(value) => self.literal(value, line)?,
            ExprKind::Variable(name) => {
                self.check_initialized(name, line)?;
                self.named(OpCode::GetName, name, line)?;
            }
            ExprKind::Assign(name, value) => {
                self.check_initialized(name, line)?;
                self.expression(value)?;
                self.named(OpCode::SetName, name, line)?;
            }
            ExprKind::Unary(operator, value) => {
                let opcode = match operator {
                    Op::Not => OpCode::Not,
                    Op::Negate => OpCode::Negate,
                    _ => return Err(compile_error(line, "Invalid unary operator.")),
                };
                self.expression(value)?;
                self.emit(opcode, line);
            }
            ExprKind::Binary(left, operator, right) => {
                let opcode = match operator {
                    Op::Add => OpCode::Add,
                    Op::Subtract => OpCode::Subtract,
                    Op::Multiply => OpCode::Multiply,
                    Op::Divide => OpCode::Divide,
                    Op::Equal => OpCode::Equal,
                    Op::NotEqual => OpCode::NotEqual,
                    Op::Greater => OpCode::Greater,
                    Op::GreaterEqual => OpCode::GreaterEqual,
                    Op::Less => OpCode::Less,
                    Op::LessEqual => OpCode::LessEqual,
                    _ => return Err(compile_error(line, "Invalid binary operator.")),
                };
                self.expression(left)?;
                self.expression(right)?;
                self.emit(opcode, line);
            }
            ExprKind::Logical(left, operator, right) => {
                if !matches!(operator, Op::And | Op::Or) {
                    return Err(compile_error(line, "Invalid logical operator."));
                }
                self.expression(left)?;
                let short = self.operand(OpCode::JumpIfFalse, 0, line)?;
                if *operator == Op::Or {
                    let end = self.operand(OpCode::Jump, 0, line)?;
                    self.patch_jump(short, line)?;
                    self.emit(OpCode::Pop, line);
                    self.expression(right)?;
                    self.patch_jump(end, line)?;
                } else {
                    self.emit(OpCode::Pop, line);
                    self.expression(right)?;
                    self.patch_jump(short, line)?;
                }
            }
            ExprKind::Call(_, _) => {
                return Err(compile_error(
                    line,
                    "Calls are not supported by the bytecode backend.",
                ))
            }
            ExprKind::Get(_, _) | ExprKind::Set(_, _, _) => {
                return Err(compile_error(
                    line,
                    "Properties are not supported by the bytecode backend.",
                ))
            }
            ExprKind::This => {
                return Err(compile_error(
                    line,
                    "'this' is not supported by the bytecode backend.",
                ))
            }
            ExprKind::Super(_) => {
                return Err(compile_error(
                    line,
                    "'super' is not supported by the bytecode backend.",
                ))
            }
        }
        Ok(())
    }
}

/// Metadata captured immediately before an instruction. No heap handles or
/// user string payloads are retained by tracing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceEvent {
    pub offset: usize,
    pub line: usize,
    pub opcode: OpCode,
    pub stack_depth: usize,
    pub scope_depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(Handle),
}

impl Value {
    fn truthy(self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }
    fn handle(self) -> Option<Handle> {
        if let Self::String(handle) = self {
            Some(handle)
        } else {
            None
        }
    }
}

type Environment = HashMap<Handle, Value>;

/// Persistent-global bytecode VM; no AST evaluator is present.
///
/// ```
/// use interpreter::ast::{Expr, ExprKind, Literal, Stmt, StmtKind};
/// use interpreter::vm::Vm;
/// let print = Stmt {
///     kind: StmtKind::Print(Expr {
///         kind: ExprKind::Literal(Literal::String("hello".into())), line: 1,
///     }),
///     line: 1,
/// };
/// let mut vm = Vm::new();
/// assert_eq!(vm.run(&[print]).unwrap(), ["hello"]);
/// assert!(vm.run(&[]).unwrap().is_empty());
/// ```
#[derive(Debug)]
pub struct Vm {
    heap: Heap,
    globals: Environment,
    scopes: Vec<Environment>,
    stack: Vec<Value>,
    minimum_gc: usize,
    next_gc: usize,
    gc_stress: bool,
    compile_options: CompileOptions,
    trace_limit: usize,
    trace: Vec<TraceEvent>,
    trace_truncated: bool,
    instruction_limit: Option<u64>,
    instructions_executed: u64,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self::with_gc_threshold(128)
    }

    /// Collect before interning when this many objects are live (minimum one).
    /// After each collection the threshold grows with the surviving live set.
    pub fn with_gc_threshold(objects: usize) -> Self {
        let minimum_gc = objects.max(1);
        Self {
            heap: Heap::new(),
            globals: HashMap::new(),
            scopes: Vec::new(),
            stack: Vec::new(),
            minimum_gc,
            next_gc: minimum_gc,
            gc_stress: false,
            compile_options: CompileOptions::default(),
            trace_limit: 0,
            trace: Vec::new(),
            trace_truncated: false,
            instruction_limit: None,
            instructions_executed: 0,
        }
    }

    /// Force collection before every intern operation, even when reusing a string.
    pub fn set_gc_stress(&mut self, enabled: bool) {
        self.gc_stress = enabled;
    }
    pub fn set_compile_options(&mut self, options: CompileOptions) {
        self.compile_options = options;
    }
    /// Retain the first `limit` events per run; zero disables tracing.
    pub fn set_trace_limit(&mut self, limit: usize) {
        self.trace_limit = limit;
        self.trace = Vec::new();
        self.trace_truncated = false;
    }
    pub fn trace(&self) -> &[TraceEvent] {
        &self.trace
    }
    pub fn trace_truncated(&self) -> bool {
        self.trace_truncated
    }

    /// Clear per-run diagnostics when a caller fails before compiling an AST.
    pub fn clear_debug_state(&mut self) {
        self.instructions_executed = 0;
        self.trace.clear();
        self.trace_truncated = false;
    }

    /// Maximum attempted instructions per run; None is unlimited. Exhaustion
    /// occurs before the next instruction, without rolling back earlier globals.
    pub fn set_instruction_limit(&mut self, limit: Option<u64>) {
        self.instruction_limit = limit;
    }
    /// Includes an instruction that fails at runtime, but not a budget-blocked one.
    pub fn instructions_executed(&self) -> u64 {
        self.instructions_executed
    }

    /// Deterministic, owned debug values; no GC handles escape the VM. Lexical
    /// locals are intentionally excluded. Taking a snapshot does not intern.
    pub fn global_snapshot(&self) -> Result<Vec<(String, Literal)>, String> {
        let mut snapshot = Vec::with_capacity(self.globals.len());
        for (name, value) in &self.globals {
            let name = self.string(*name, 0)?.to_owned();
            let value = match value {
                Value::Nil => Literal::Nil,
                Value::Bool(value) => Literal::Bool(*value),
                Value::Number(value) => Literal::Number(*value),
                Value::String(handle) => Literal::String(self.string(*handle, 0)?.to_owned()),
            };
            snapshot.push((name, value));
        }
        snapshot.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        Ok(snapshot)
    }

    pub fn gc_stats(&self) -> GcStats {
        self.heap.stats()
    }

    pub fn collect_garbage(&mut self) {
        let mut roots: Vec<_> = self
            .stack
            .iter()
            .filter_map(|value| value.handle())
            .collect();
        for environment in std::iter::once(&self.globals).chain(self.scopes.iter()) {
            for (name, value) in environment {
                roots.push(*name);
                if let Some(handle) = value.handle() {
                    roots.push(handle);
                }
            }
        }
        self.heap.collect(roots);
        self.next_gc = self
            .heap
            .stats()
            .live_objects
            .saturating_mul(2)
            .max(self.minimum_gc);
    }

    /// Globals survive runs and runtime errors. Output belongs only to this run.
    /// Compilation errors execute nothing. On all errors, lexical environments
    /// and the operand stack are discarded, then unreachable strings are swept.
    pub fn run(&mut self, statements: &[Stmt]) -> Result<Vec<String>, String> {
        self.begin_run();
        let result = compile_with_options(statements, self.compile_options)
            .and_then(|chunk| self.execute(&chunk));
        self.finish_run(result)
    }

    /// Execute reusable, heap-independent bytecode. Compile options affect only
    /// `run`; this method executes the supplied chunk without recompiling it.
    pub fn run_chunk(&mut self, chunk: &Chunk) -> Result<Vec<String>, String> {
        self.begin_run();
        let result = chunk.validate().and_then(|()| self.execute(chunk));
        self.finish_run(result)
    }

    fn begin_run(&mut self) {
        self.clear_debug_state();
        self.stack.clear();
        self.scopes.clear();
    }

    fn finish_run(&mut self, result: Result<Vec<String>, String>) -> Result<Vec<String>, String> {
        self.stack.clear();
        self.scopes.clear();
        self.collect_garbage();
        result
    }

    fn intern(&mut self, text: &str) -> Handle {
        if self.gc_stress || self.heap.stats().live_objects >= self.next_gc {
            self.collect_garbage();
        }
        self.heap.intern(text)
    }

    fn pop(&mut self, line: usize) -> Result<Value, String> {
        self.stack
            .pop()
            .ok_or_else(|| runtime_error(line, "Operand stack underflow."))
    }
    fn peek(&self, line: usize) -> Result<Value, String> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| runtime_error(line, "Operand stack underflow."))
    }

    fn string(&self, handle: Handle, line: usize) -> Result<&str, String> {
        self.heap
            .get(handle)
            .ok_or_else(|| runtime_error(line, "Invalid string handle."))
    }

    fn render(&self, value: Value, line: usize) -> Result<String, String> {
        Ok(match value {
            Value::Nil => "nil".to_owned(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            Value::String(handle) => self.string(handle, line)?.to_owned(),
        })
    }

    fn load_constant(&mut self, constant: &Literal) -> Value {
        match constant {
            Literal::Nil => Value::Nil,
            Literal::Bool(value) => Value::Bool(*value),
            Literal::Number(value) => Value::Number(*value),
            Literal::String(text) => Value::String(self.intern(text)),
        }
    }

    fn execute(&mut self, chunk: &Chunk) -> Result<Vec<String>, String> {
        let mut output = Vec::new();
        let mut ip = 0;
        while ip < chunk.code.len() {
            if self
                .instruction_limit
                .is_some_and(|limit| self.instructions_executed >= limit)
            {
                return Err(runtime_error(
                    chunk.lines[ip],
                    "Instruction budget exhausted.",
                ));
            }
            let (opcode, operand, next) = chunk.decode(ip)?;
            self.instructions_executed = self.instructions_executed.saturating_add(1);
            let line = chunk.lines[ip];
            if self.trace_limit > 0 {
                if self.trace.len() < self.trace_limit {
                    self.trace.push(TraceEvent {
                        offset: ip,
                        line,
                        opcode,
                        stack_depth: self.stack.len(),
                        scope_depth: self.scopes.len(),
                    });
                } else {
                    self.trace_truncated = true;
                }
            }
            ip = next;
            match opcode {
                OpCode::Constant => {
                    let constant = chunk
                        .constants
                        .get(operand)
                        .ok_or_else(|| runtime_error(line, "Invalid constant index."))?;
                    let value = self.load_constant(constant);
                    self.stack.push(value);
                }
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.pop(line)?;
                }
                OpCode::GetName | OpCode::DefineName | OpCode::SetName => {
                    let Some(Literal::String(name)) = chunk.constants.get(operand) else {
                        return Err(runtime_error(line, "Invalid name constant."));
                    };
                    // Allocate/intern while any initializer/assigned value is
                    // still on the operand stack and therefore a GC root.
                    let handle = self.intern(name);
                    match opcode {
                        OpCode::DefineName => {
                            let value = self.pop(line)?;
                            self.scopes
                                .last_mut()
                                .unwrap_or(&mut self.globals)
                                .insert(handle, value);
                        }
                        OpCode::GetName => {
                            let value = self
                                .scopes
                                .iter()
                                .rev()
                                .chain(std::iter::once(&self.globals))
                                .find_map(|environment| environment.get(&handle).copied())
                                .ok_or_else(|| {
                                    runtime_error(line, &format!("Undefined variable '{name}'."))
                                })?;
                            self.stack.push(value);
                        }
                        OpCode::SetName => {
                            let value = self.peek(line)?;
                            let slot = self
                                .scopes
                                .iter_mut()
                                .rev()
                                .chain(std::iter::once(&mut self.globals))
                                .find_map(|environment| environment.get_mut(&handle))
                                .ok_or_else(|| {
                                    runtime_error(line, &format!("Undefined variable '{name}'."))
                                })?;
                            *slot = value;
                        }
                        _ => unreachable!(),
                    }
                }
                OpCode::EnterScope => self.scopes.push(HashMap::new()),
                OpCode::ExitScope => {
                    self.scopes
                        .pop()
                        .ok_or_else(|| runtime_error(line, "Lexical scope underflow."))?;
                }
                OpCode::Not => {
                    let value = self.pop(line)?;
                    self.stack.push(Value::Bool(!value.truthy()));
                }
                OpCode::Negate => {
                    let Value::Number(value) = self.pop(line)? else {
                        return Err(runtime_error(line, "Operand must be a number."));
                    };
                    self.stack.push(Value::Number(-value));
                }
                OpCode::Print => {
                    let value = self.pop(line)?;
                    output.push(self.render(value, line)?);
                }
                OpCode::Jump => ip = operand,
                OpCode::JumpIfFalse => {
                    if !self.peek(line)?.truthy() {
                        ip = operand;
                    }
                }
                _ => self.binary(opcode, line)?,
            }
        }
        if !self.stack.is_empty() || !self.scopes.is_empty() {
            return Err(runtime_error(
                chunk.lines.last().copied().unwrap_or(0),
                "Unbalanced bytecode state.",
            ));
        }
        Ok(output)
    }

    fn binary(&mut self, opcode: OpCode, line: usize) -> Result<(), String> {
        let right = self.pop(line)?;
        let left = self.pop(line)?;
        let result = match (opcode, left, right) {
            (OpCode::Equal, left, right) => Value::Bool(left == right),
            (OpCode::NotEqual, left, right) => Value::Bool(left != right),
            (OpCode::Add, Value::String(left), Value::String(right)) => {
                // Copy both payloads before allocation may collect the popped
                // operands. Their contents are now safely in this owned buffer.
                let joined = format!("{}{}", self.string(left, line)?, self.string(right, line)?);
                Value::String(self.intern(&joined))
            }
            (opcode, Value::Number(left), Value::Number(right)) => match opcode {
                OpCode::Add => Value::Number(left + right),
                OpCode::Subtract => Value::Number(left - right),
                OpCode::Multiply => Value::Number(left * right),
                OpCode::Divide => Value::Number(left / right),
                OpCode::Greater => Value::Bool(left > right),
                OpCode::GreaterEqual => Value::Bool(left >= right),
                OpCode::Less => Value::Bool(left < right),
                OpCode::LessEqual => Value::Bool(left <= right),
                _ => return Err(runtime_error(line, "Invalid binary opcode.")),
            },
            (OpCode::Add, _, _) => {
                return Err(runtime_error(
                    line,
                    "Operands must be two numbers or two strings.",
                ))
            }
            _ => return Err(runtime_error(line, "Operands must be numbers.")),
        };
        self.stack.push(result);
        Ok(())
    }
}
