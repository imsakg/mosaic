#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Variable(String),
    Assign(String, Box<Expr>),
    Unary(Op, Box<Expr>),
    Binary(Box<Expr>, Op, Box<Expr>),
    Logical(Box<Expr>, Op, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Get(Box<Expr>, String),
    Set(Box<Expr>, String, Box<Expr>),
    This,
    Super(String),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub line: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Expression(Expr),
    Print(Expr),
    Var(String, Option<Expr>),
    Block(Vec<Stmt>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Function(Function),
    Return(Option<Expr>),
    Class(String, Option<Expr>, Vec<Function>),
}
