//! Recursive descent for the Lox grammar from Crafting Interpreters.
use crate::ast::{Expr, ExprKind as E, Function, Literal, Op, Stmt, StmtKind as S};
use crate::error::Diagnostic;
use crate::token::{Token, TokenType as T};

type Result<T> = std::result::Result<T, Diagnostic>;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    nesting: usize,
}
impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if !tokens.last().is_some_and(|t| t.token_type == T::EOF) {
            let line = tokens.last().map_or(1, |t| t.line);
            tokens.push(Token {
                token_type: T::EOF,
                lexeme: String::new(),
                literal: None,
                line,
            });
        }
        Self {
            tokens,
            current: 0,
            nesting: 0,
        }
    }
    pub fn parse(&mut self) -> std::result::Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        let mut errors = Vec::new();
        while !self.check(&T::EOF) {
            match self.declaration() {
                Ok(s) => statements.push(s),
                Err(e) => {
                    errors.push(e);
                    self.synchronize();
                }
            }
        }
        if errors.is_empty() {
            Ok(statements)
        } else {
            Err(errors)
        }
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        if t.token_type != T::EOF {
            self.current += 1;
        }
        t
    }
    fn check(&self, t: &T) -> bool {
        std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(t)
    }
    fn take(&mut self, t: &T) -> bool {
        if self.check(t) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic::new("Parse", self.peek().line, message)
    }
    fn expect(&mut self, t: &T, message: &str) -> Result<Token> {
        if self.check(t) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }
    fn name(&mut self, message: &str) -> Result<String> {
        if let T::Identifier(name) = self.peek().token_type.clone() {
            self.advance();
            Ok(name)
        } else {
            Err(self.error(message))
        }
    }
    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.nesting >= 64 {
            return Err(self.error("Syntax nesting limit exceeded."));
        }
        self.nesting += 1;
        let result = f(self);
        self.nesting -= 1;
        result
    }
    fn declaration(&mut self) -> Result<Stmt> {
        self.nested(|p| {
            let line = p.peek().line;
            let kind = if p.take(&T::Class) {
                let name = p.name("Expected class name.")?;
                let superclass = if p.take(&T::Less) {
                    let line = p.peek().line;
                    Some(Expr {
                        kind: E::Variable(p.name("Expected superclass name.")?),
                        line,
                    })
                } else {
                    None
                };
                p.expect(&T::LeftBrace, "Expected '{' before class body.")?;
                let mut methods = Vec::new();
                while !p.check(&T::RightBrace) && !p.check(&T::EOF) {
                    methods.push(p.function()?);
                }
                p.expect(&T::RightBrace, "Expected '}' after class body.")?;
                S::Class(name, superclass, methods)
            } else if p.take(&T::Fun) {
                S::Function(p.function()?)
            } else if p.take(&T::Var) {
                return p.variable(line);
            } else {
                return p.statement();
            };
            Ok(Stmt { kind, line })
        })
    }
    fn function(&mut self) -> Result<Function> {
        let line = self.peek().line;
        let name = self.name("Expected function name.")?;
        self.expect(&T::LeftParen, "Expected '(' after function name.")?;
        let mut params = Vec::new();
        if !self.check(&T::RightParen) {
            loop {
                if params.len() >= 255 {
                    return Err(self.error("Cannot have more than 255 parameters."));
                }
                params.push(self.name("Expected parameter name.")?);
                if !self.take(&T::Comma) {
                    break;
                }
            }
        }
        self.expect(&T::RightParen, "Expected ')' after parameters.")?;
        self.expect(&T::LeftBrace, "Expected '{' before function body.")?;
        let body = self.block()?;
        Ok(Function {
            name,
            params,
            body,
            line,
        })
    }
    fn variable(&mut self, line: usize) -> Result<Stmt> {
        let name = self.name("Expected variable name.")?;
        let init = if self.take(&T::Equal) {
            Some(self.expression()?)
        } else {
            None
        };
        self.expect(&T::Semicolon, "Expected ';' after variable declaration.")?;
        Ok(Stmt {
            kind: S::Var(name, init),
            line,
        })
    }
    fn block(&mut self) -> Result<Vec<Stmt>> {
        let mut body = Vec::new();
        while !self.check(&T::RightBrace) && !self.check(&T::EOF) {
            body.push(self.declaration()?);
        }
        self.expect(&T::RightBrace, "Expected '}' after block.")?;
        Ok(body)
    }
    fn statement(&mut self) -> Result<Stmt> {
        self.nested(|p| {
            let line = p.peek().line;
            let kind = if p.take(&T::Print) {
                let value = p.expression()?;
                p.expect(&T::Semicolon, "Expected ';' after value.")?;
                S::Print(value)
            } else if p.take(&T::LeftBrace) {
                S::Block(p.block()?)
            } else if p.take(&T::If) {
                let condition = p.condition()?;
                let then = Box::new(p.statement()?);
                let otherwise = if p.take(&T::Else) {
                    Some(Box::new(p.statement()?))
                } else {
                    None
                };
                S::If(condition, then, otherwise)
            } else if p.take(&T::While) {
                S::While(p.condition()?, Box::new(p.statement()?))
            } else if p.take(&T::For) {
                return p.for_loop(line);
            } else if p.take(&T::Return) {
                let value = if p.check(&T::Semicolon) {
                    None
                } else {
                    Some(p.expression()?)
                };
                p.expect(&T::Semicolon, "Expected ';' after return value.")?;
                S::Return(value)
            } else {
                let value = p.expression()?;
                p.expect(&T::Semicolon, "Expected ';' after expression.")?;
                S::Expression(value)
            };
            Ok(Stmt { kind, line })
        })
    }
    fn condition(&mut self) -> Result<Expr> {
        self.expect(&T::LeftParen, "Expected '(' before condition.")?;
        let condition = self.expression()?;
        self.expect(&T::RightParen, "Expected ')' after condition.")?;
        Ok(condition)
    }
    fn for_loop(&mut self, line: usize) -> Result<Stmt> {
        self.expect(&T::LeftParen, "Expected '(' after 'for'.")?;
        let initializer = if self.take(&T::Semicolon) {
            None
        } else if self.take(&T::Var) {
            Some(self.variable(line)?)
        } else {
            let value = self.expression()?;
            self.expect(&T::Semicolon, "Expected ';' after loop initializer.")?;
            Some(Stmt {
                kind: S::Expression(value),
                line,
            })
        };
        let condition = if self.check(&T::Semicolon) {
            Expr {
                kind: E::Literal(Literal::Bool(true)),
                line,
            }
        } else {
            self.expression()?
        };
        self.expect(&T::Semicolon, "Expected ';' after loop condition.")?;
        let increment = if self.check(&T::RightParen) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&T::RightParen, "Expected ')' after loop clauses.")?;
        let mut body = self.statement()?;
        if let Some(inc) = increment {
            body = Stmt {
                kind: S::Block(vec![
                    body,
                    Stmt {
                        kind: S::Expression(inc),
                        line,
                    },
                ]),
                line,
            };
        }
        body = Stmt {
            kind: S::While(condition, Box::new(body)),
            line,
        };
        if let Some(init) = initializer {
            body = Stmt {
                kind: S::Block(vec![init, body]),
                line,
            };
        }
        Ok(body)
    }
    fn expression(&mut self) -> Result<Expr> {
        self.nested(|p| p.assignment())
    }
    fn assignment(&mut self) -> Result<Expr> {
        let expression = self.or()?;
        if self.take(&T::Equal) {
            let value = Box::new(self.expression()?);
            let line = expression.line;
            let kind = match expression.kind {
                E::Variable(name) => E::Assign(name, value),
                E::Get(object, name) => E::Set(object, name, value),
                _ => return Err(Diagnostic::new("Parse", line, "Invalid assignment target.")),
            };
            Ok(Expr { kind, line })
        } else {
            Ok(expression)
        }
    }
    fn or(&mut self) -> Result<Expr> {
        self.binary(Self::and, &[(T::Or, Op::Or)], true)
    }
    fn and(&mut self) -> Result<Expr> {
        self.binary(Self::equality, &[(T::And, Op::And)], true)
    }
    fn equality(&mut self) -> Result<Expr> {
        self.binary(
            Self::comparison,
            &[(T::EqualEqual, Op::Equal), (T::BangEqual, Op::NotEqual)],
            false,
        )
    }
    fn comparison(&mut self) -> Result<Expr> {
        self.binary(
            Self::term,
            &[
                (T::Greater, Op::Greater),
                (T::GreaterEqual, Op::GreaterEqual),
                (T::Less, Op::Less),
                (T::LessEqual, Op::LessEqual),
            ],
            false,
        )
    }
    fn term(&mut self) -> Result<Expr> {
        self.binary(
            Self::factor,
            &[(T::Plus, Op::Add), (T::Minus, Op::Subtract)],
            false,
        )
    }
    fn factor(&mut self) -> Result<Expr> {
        self.binary(
            Self::unary,
            &[(T::Star, Op::Multiply), (T::Slash, Op::Divide)],
            false,
        )
    }
    fn binary(
        &mut self,
        next: fn(&mut Self) -> Result<Expr>,
        ops: &[(T, Op)],
        logical: bool,
    ) -> Result<Expr> {
        let mut left = next(self)?;
        while let Some((_, op)) = ops.iter().find(|(t, _)| self.check(t)) {
            let op = *op;
            let line = self.advance().line;
            let right = Box::new(next(self)?);
            left = Expr {
                kind: if logical {
                    E::Logical(Box::new(left), op, right)
                } else {
                    E::Binary(Box::new(left), op, right)
                },
                line,
            };
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Expr> {
        if self.check(&T::Bang) || self.check(&T::Minus) {
            let t = self.advance();
            let op = if t.token_type == T::Bang {
                Op::Not
            } else {
                Op::Negate
            };
            let operand = self.nested(|p| p.unary())?;
            Ok(Expr {
                kind: E::Unary(op, Box::new(operand)),
                line: t.line,
            })
        } else {
            self.call()
        }
    }
    fn call(&mut self) -> Result<Expr> {
        let mut callee = self.primary()?;
        loop {
            if self.take(&T::LeftParen) {
                let mut args = Vec::new();
                if !self.check(&T::RightParen) {
                    loop {
                        if args.len() >= 255 {
                            return Err(self.error("Cannot have more than 255 arguments."));
                        }
                        args.push(self.expression()?);
                        if !self.take(&T::Comma) {
                            break;
                        }
                    }
                }
                let line = self
                    .expect(&T::RightParen, "Expected ')' after arguments.")?
                    .line;
                callee = Expr {
                    kind: E::Call(Box::new(callee), args),
                    line,
                };
            } else if self.take(&T::Dot) {
                let line = self.peek().line;
                let name = self.name("Expected property name after '.'.")?;
                callee = Expr {
                    kind: E::Get(Box::new(callee), name),
                    line,
                };
            } else {
                break;
            }
        }
        Ok(callee)
    }
    fn primary(&mut self) -> Result<Expr> {
        let t = self.advance();
        let kind = match t.token_type {
            T::Nil => E::Literal(Literal::Nil),
            T::True => E::Literal(Literal::Bool(true)),
            T::False => E::Literal(Literal::Bool(false)),
            T::Number(n) => E::Literal(Literal::Number(n)),
            T::String(s) => E::Literal(Literal::String(s)),
            T::Identifier(name) => E::Variable(name),
            T::This => E::This,
            T::Super => {
                self.expect(&T::Dot, "Expected '.' after 'super'.")?;
                E::Super(self.name("Expected superclass method name.")?)
            }
            T::LeftParen => {
                let e = self.expression()?;
                self.expect(&T::RightParen, "Expected ')' after expression.")?;
                return Ok(e);
            }
            _ => return Err(Diagnostic::new("Parse", t.line, "Expected expression.")),
        };
        Ok(Expr { kind, line: t.line })
    }
    fn synchronize(&mut self) {
        if self.check(&T::EOF) {
            return;
        }
        self.advance();
        while !self.check(&T::EOF) {
            if self.tokens[self.current - 1].token_type == T::Semicolon {
                return;
            }
            if matches!(
                self.peek().token_type,
                T::Class | T::Fun | T::Var | T::For | T::If | T::While | T::Print | T::Return
            ) {
                return;
            }
            self.advance();
        }
    }
}
