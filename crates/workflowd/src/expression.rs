// SPDX-License-Identifier: AGPL-3.0-or-later
//! A small, deterministic expression compiler and evaluator.
//!
//! This is deliberately not JavaScript.  It accepts only the language frozen
//! by ADR 0057 and has no host, network, clock, random, mutation, method, or
//! reflection access.

use serde_json::{Map, Number, Value};
use std::{fmt, str::FromStr};

const MAX_TOKENS: usize = 512;
const MAX_NODES: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionError {
    pub code: String,
    pub message: String,
    pub offset: usize,
}

impl ExpressionError {
    fn new(code: &'static str, message: impl Into<String>, offset: usize) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            offset,
        }
    }

    fn syntax(message: impl Into<String>, offset: usize) -> Self {
        Self::new("canopy.expression.syntax", message, offset)
    }

    fn unsupported(message: impl Into<String>, offset: usize) -> Self {
        Self::new("canopy.expression.unsupported", message, offset)
    }

    fn type_error(message: impl Into<String>, offset: usize) -> Self {
        Self::new("canopy.expression.type", message, offset)
    }

    fn missing(message: impl Into<String>, offset: usize) -> Self {
        Self::new("canopy.expression.missing", message, offset)
    }

    fn overflow(message: impl Into<String>, offset: usize) -> Self {
        Self::new("canopy.expression.overflow", message, offset)
    }

    fn limit(message: impl Into<String>, offset: usize) -> Self {
        Self::new("canopy.expression.resource_limit", message, offset)
    }
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}: {}",
            self.code, self.offset, self.message
        )
    }
}

impl std::error::Error for ExpressionError {}

#[derive(Clone, Debug, PartialEq)]
pub enum EvalValue {
    Json(Value),
    Missing,
}

impl EvalValue {
    pub fn into_option(self) -> Option<Value> {
        match self {
            Self::Json(value) => Some(value),
            Self::Missing => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    source: String,
    root: Expr,
}

impl Program {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn evaluate(&self, input: &Value, item_index: u64) -> Result<EvalValue, ExpressionError> {
        self.root.eval(input, item_index)
    }
}

pub fn compile(source: &str) -> Result<Program, ExpressionError> {
    let normalized = normalize_source(source)?;
    if normalized.trim().is_empty() {
        return Err(ExpressionError::syntax("expression is empty", 0));
    }
    let tokens = Lexer::new(normalized).lex()?;
    let mut parser = Parser::new(tokens);
    let root = parser.parse()?;
    Ok(Program {
        source: normalized.to_owned(),
        root,
    })
}

fn normalize_source(source: &str) -> Result<&str, ExpressionError> {
    let trimmed = source.trim();
    if let Some(inner) = trimmed.strip_prefix("={{") {
        if let Some(inner) = inner.strip_suffix("}}") {
            return Ok(inner.trim());
        }
        return Err(ExpressionError::syntax(
            "expression wrapper must end with }}",
            trimmed.len(),
        ));
    }
    if trimmed.starts_with("=") {
        return Err(ExpressionError::unsupported(
            "only the plain expression form or ={{...}} is accepted",
            0,
        ));
    }
    Ok(trimmed)
}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Number(String),
    String(String),
    Identifier(String),
    True,
    False,
    Null,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    StrictEqual,
    StrictNotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Coalesce,
    Question,
    Colon,
    Dot,
    Comma,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    End,
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    offset: usize,
}

struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    index: usize,
    byte_offsets: Vec<usize>,
    token_count: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        let mut byte_offsets = source
            .char_indices()
            .map(|(offset, _)| offset)
            .collect::<Vec<_>>();
        byte_offsets.push(source.len());
        Self {
            source,
            chars: source.chars().collect(),
            index: 0,
            byte_offsets,
            token_count: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, ExpressionError> {
        let mut tokens = Vec::new();
        while self.index < self.chars.len() {
            self.skip_whitespace();
            if self.index >= self.chars.len() {
                break;
            }
            if self.token_count >= MAX_TOKENS {
                return Err(ExpressionError::limit(
                    "expression token limit exceeded",
                    self.byte_offset(),
                ));
            }
            let offset = self.byte_offset();
            let kind = self.next_token(offset)?;
            self.token_count += 1;
            tokens.push(Token { kind, offset });
        }
        tokens.push(Token {
            kind: TokenKind::End,
            offset: self.source.len(),
        });
        Ok(tokens)
    }

    fn skip_whitespace(&mut self) {
        while self.index < self.chars.len() && self.chars[self.index].is_whitespace() {
            self.index += 1;
        }
    }

    fn byte_offset(&self) -> usize {
        self.byte_offsets[self.index]
    }

    fn take_if(&mut self, expected: char) -> bool {
        if self.chars.get(self.index) == Some(&expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn next_token(&mut self, offset: usize) -> Result<TokenKind, ExpressionError> {
        let character = self.chars[self.index];
        let kind = match character {
            '+' => {
                self.index += 1;
                TokenKind::Plus
            }
            '-' => {
                self.index += 1;
                TokenKind::Minus
            }
            '*' => {
                self.index += 1;
                TokenKind::Star
            }
            '/' => {
                self.index += 1;
                TokenKind::Slash
            }
            '%' => {
                self.index += 1;
                TokenKind::Percent
            }
            '!' => {
                self.index += 1;
                if self.take_if('=') {
                    if self.take_if('=') {
                        TokenKind::StrictNotEqual
                    } else {
                        return Err(ExpressionError::unsupported(
                            "loose != equality is not supported; use !==",
                            offset,
                        ));
                    }
                } else {
                    TokenKind::Bang
                }
            }
            '=' => {
                self.index += 1;
                if self.take_if('=') && self.take_if('=') {
                    TokenKind::StrictEqual
                } else {
                    return Err(ExpressionError::unsupported(
                        "assignment and loose equality are not supported",
                        offset,
                    ));
                }
            }
            '<' => {
                self.index += 1;
                if self.take_if('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                self.index += 1;
                if self.take_if('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '&' => {
                self.index += 1;
                if self.take_if('&') {
                    TokenKind::And
                } else {
                    return Err(ExpressionError::unsupported(
                        "single ampersand is not supported; use &&",
                        offset,
                    ));
                }
            }
            '|' => {
                self.index += 1;
                if self.take_if('|') {
                    TokenKind::Or
                } else {
                    return Err(ExpressionError::unsupported(
                        "single pipe is not supported; use ||",
                        offset,
                    ));
                }
            }
            '?' => {
                self.index += 1;
                if self.take_if('?') {
                    TokenKind::Coalesce
                } else {
                    TokenKind::Question
                }
            }
            ':' => {
                self.index += 1;
                TokenKind::Colon
            }
            '.' => {
                self.index += 1;
                TokenKind::Dot
            }
            ',' => {
                self.index += 1;
                TokenKind::Comma
            }
            '(' => {
                self.index += 1;
                TokenKind::LeftParen
            }
            ')' => {
                self.index += 1;
                TokenKind::RightParen
            }
            '[' => {
                self.index += 1;
                TokenKind::LeftBracket
            }
            ']' => {
                self.index += 1;
                TokenKind::RightBracket
            }
            '{' => {
                self.index += 1;
                TokenKind::LeftBrace
            }
            '}' => {
                self.index += 1;
                TokenKind::RightBrace
            }
            '"' => self.string_literal(offset)?,
            character if character.is_ascii_digit() => self.number_literal(offset)?,
            '$' | '_' | 'a'..='z' | 'A'..='Z' => self.identifier(offset),
            _ => {
                return Err(ExpressionError::unsupported(
                    format!("character {character:?} is not in the safe expression language"),
                    offset,
                ));
            }
        };
        Ok(kind)
    }

    fn string_literal(&mut self, offset: usize) -> Result<TokenKind, ExpressionError> {
        let start = self.index;
        self.index += 1;
        let mut escaped = false;
        while self.index < self.chars.len() {
            let character = self.chars[self.index];
            self.index += 1;
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                let end = self.byte_offset();
                let raw = &self.source[self.byte_offsets[start]..end];
                let value = serde_json::from_str::<String>(raw).map_err(|error| {
                    ExpressionError::syntax(format!("invalid string literal: {error}"), offset)
                })?;
                return Ok(TokenKind::String(value));
            }
        }
        Err(ExpressionError::syntax(
            "unterminated string literal",
            offset,
        ))
    }

    fn number_literal(&mut self, offset: usize) -> Result<TokenKind, ExpressionError> {
        let start = self.index;
        while self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
            self.index += 1;
        }
        if self.take_if('.') {
            if !self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                return Err(ExpressionError::syntax(
                    "a decimal point must be followed by digits",
                    offset,
                ));
            }
            while self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                self.index += 1;
            }
        }
        if self
            .chars
            .get(self.index)
            .is_some_and(|character| *character == 'e' || *character == 'E')
        {
            self.index += 1;
            if self
                .chars
                .get(self.index)
                .is_some_and(|character| *character == '+' || *character == '-')
            {
                self.index += 1;
            }
            if !self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                return Err(ExpressionError::syntax(
                    "an exponent must contain digits",
                    offset,
                ));
            }
            while self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                self.index += 1;
            }
        }
        let end = self.byte_offset();
        let raw = self.source[self.byte_offsets[start]..end].to_owned();
        let parsed = serde_json::from_str::<Value>(&raw).map_err(|error| {
            ExpressionError::syntax(format!("invalid number literal: {error}"), offset)
        })?;
        if !parsed.is_number() {
            return Err(ExpressionError::syntax("invalid number literal", offset));
        }
        Ok(TokenKind::Number(raw))
    }

    fn identifier(&mut self, _offset: usize) -> TokenKind {
        let start = self.index;
        self.index += 1;
        while self.chars.get(self.index).is_some_and(|character| {
            character.is_ascii_alphanumeric() || *character == '_' || *character == '$'
        }) {
            self.index += 1;
        }
        let end = self.byte_offset();
        let identifier = self.source[self.byte_offsets[start]..end].to_owned();
        match identifier.as_str() {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(identifier),
        }
    }
}

#[derive(Clone, Debug)]
enum Expr {
    Literal(Value),
    Json,
    ItemIndex,
    Access(Box<Expr>, Accessor),
    Unary(UnaryOp, Box<Expr>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
}

#[derive(Clone, Debug)]
enum Accessor {
    Property(String),
    Index(Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum UnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug)]
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    StrictEqual,
    StrictNotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Coalesce,
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    nodes: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            index: 0,
            nodes: 0,
        }
    }

    fn parse(&mut self) -> Result<Expr, ExpressionError> {
        let expression = self.parse_conditional()?;
        if !matches!(self.peek(), TokenKind::End) {
            return Err(ExpressionError::syntax(
                format!("unexpected token {}", describe_token(self.peek())),
                self.current_offset(),
            ));
        }
        Ok(expression)
    }

    fn parse_conditional(&mut self) -> Result<Expr, ExpressionError> {
        let condition = self.parse_binary(1)?;
        if !self.consume_if(|kind| matches!(kind, TokenKind::Question)) {
            return Ok(condition);
        }
        let when_true = self.parse_conditional()?;
        self.expect(
            |kind| matches!(kind, TokenKind::Colon),
            "expected : in ternary expression",
        )?;
        let when_false = self.parse_conditional()?;
        self.node(Expr::Conditional(
            Box::new(condition),
            Box::new(when_true),
            Box::new(when_false),
        ))
    }

    fn parse_binary(&mut self, minimum_precedence: u8) -> Result<Expr, ExpressionError> {
        let mut left = self.parse_unary()?;
        loop {
            let Some((precedence, operator)) = binary_operator(self.peek()) else {
                break;
            };
            if precedence < minimum_precedence {
                break;
            }
            self.index += 1;
            let right = self.parse_binary(precedence + 1)?;
            left = self.node(Expr::Binary(Box::new(left), operator, Box::new(right)))?;
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ExpressionError> {
        let operator = match self.peek() {
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Negate),
            _ => None,
        };
        if let Some(operator) = operator {
            self.index += 1;
            let value = self.parse_unary()?;
            return self.node(Expr::Unary(operator, Box::new(value)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ExpressionError> {
        let mut expression = self.parse_primary()?;
        loop {
            match self.peek() {
                TokenKind::Dot => {
                    self.index += 1;
                    let offset = self.current_offset();
                    let property = match self.take() {
                        TokenKind::Identifier(value) => value,
                        other => {
                            return Err(ExpressionError::unsupported(
                                format!(
                                    "property name expected after ., found {}",
                                    describe_token(&other)
                                ),
                                offset,
                            ));
                        }
                    };
                    expression = self.node(Expr::Access(
                        Box::new(expression),
                        Accessor::Property(property),
                    ))?;
                }
                TokenKind::LeftBracket => {
                    self.index += 1;
                    let index = self.parse_conditional()?;
                    self.expect(
                        |kind| matches!(kind, TokenKind::RightBracket),
                        "expected ] after index expression",
                    )?;
                    expression = self.node(Expr::Access(
                        Box::new(expression),
                        Accessor::Index(Box::new(index)),
                    ))?;
                }
                TokenKind::LeftParen => {
                    return Err(ExpressionError::unsupported(
                        "function and method calls are not supported",
                        self.current_offset(),
                    ));
                }
                _ => break,
            }
        }
        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expr, ExpressionError> {
        let token = self.take();
        match token {
            TokenKind::Number(raw) => {
                let value = serde_json::from_str::<Value>(&raw).map_err(|error| {
                    ExpressionError::syntax(
                        format!("invalid number literal: {error}"),
                        self.current_offset(),
                    )
                })?;
                self.node(Expr::Literal(value))
            }
            TokenKind::String(value) => self.node(Expr::Literal(Value::String(value))),
            TokenKind::True => self.node(Expr::Literal(Value::Bool(true))),
            TokenKind::False => self.node(Expr::Literal(Value::Bool(false))),
            TokenKind::Null => self.node(Expr::Literal(Value::Null)),
            TokenKind::Identifier(identifier) if identifier == "$json" => self.node(Expr::Json),
            TokenKind::Identifier(identifier) if identifier == "$itemIndex" => {
                self.node(Expr::ItemIndex)
            }
            TokenKind::Identifier(identifier) => Err(ExpressionError::unsupported(
                format!("identifier {identifier:?} is not allowed"),
                self.previous_offset(),
            )),
            TokenKind::LeftParen => {
                let expression = self.parse_conditional()?;
                self.expect(|kind| matches!(kind, TokenKind::RightParen), "expected )")?;
                Ok(expression)
            }
            TokenKind::LeftBracket => self.parse_array(),
            TokenKind::LeftBrace => self.parse_object(),
            other => Err(ExpressionError::syntax(
                format!("expected an expression, found {}", describe_token(&other)),
                self.previous_offset(),
            )),
        }
    }

    fn parse_array(&mut self) -> Result<Expr, ExpressionError> {
        let mut values = Vec::new();
        if self.consume_if(|kind| matches!(kind, TokenKind::RightBracket)) {
            return self.node(Expr::Array(values));
        }
        loop {
            values.push(self.parse_conditional()?);
            if self.consume_if(|kind| matches!(kind, TokenKind::RightBracket)) {
                break;
            }
            self.expect(
                |kind| matches!(kind, TokenKind::Comma),
                "expected comma in array literal",
            )?;
            if self.consume_if(|kind| matches!(kind, TokenKind::RightBracket)) {
                break;
            }
        }
        self.node(Expr::Array(values))
    }

    fn parse_object(&mut self) -> Result<Expr, ExpressionError> {
        let mut values = Vec::new();
        if self.consume_if(|kind| matches!(kind, TokenKind::RightBrace)) {
            return self.node(Expr::Object(values));
        }
        loop {
            let key = match self.take() {
                TokenKind::String(value) | TokenKind::Identifier(value) => value,
                other => {
                    return Err(ExpressionError::syntax(
                        format!("object key expected, found {}", describe_token(&other)),
                        self.previous_offset(),
                    ));
                }
            };
            self.expect(
                |kind| matches!(kind, TokenKind::Colon),
                "expected : after object key",
            )?;
            values.push((key, self.parse_conditional()?));
            if self.consume_if(|kind| matches!(kind, TokenKind::RightBrace)) {
                break;
            }
            self.expect(
                |kind| matches!(kind, TokenKind::Comma),
                "expected comma in object literal",
            )?;
            if self.consume_if(|kind| matches!(kind, TokenKind::RightBrace)) {
                break;
            }
        }
        self.node(Expr::Object(values))
    }

    fn node(&mut self, expression: Expr) -> Result<Expr, ExpressionError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(ExpressionError::limit(
                "expression AST node limit exceeded",
                self.current_offset(),
            ));
        }
        Ok(expression)
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.index].kind
    }

    fn current_offset(&self) -> usize {
        self.tokens[self.index].offset
    }

    fn previous_offset(&self) -> usize {
        self.tokens[self.index.saturating_sub(1)].offset
    }

    fn take(&mut self) -> TokenKind {
        let token = self.tokens[self.index].kind.clone();
        self.index += 1;
        token
    }

    fn consume_if(&mut self, predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
        if predicate(self.peek()) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn expect(
        &mut self,
        predicate: impl FnOnce(&TokenKind) -> bool,
        message: &'static str,
    ) -> Result<(), ExpressionError> {
        if self.consume_if(predicate) {
            Ok(())
        } else {
            Err(ExpressionError::syntax(message, self.current_offset()))
        }
    }
}

fn binary_operator(kind: &TokenKind) -> Option<(u8, BinaryOp)> {
    Some(match kind {
        TokenKind::Coalesce => (1, BinaryOp::Coalesce),
        TokenKind::Or => (2, BinaryOp::Or),
        TokenKind::And => (3, BinaryOp::And),
        TokenKind::StrictEqual => (4, BinaryOp::StrictEqual),
        TokenKind::StrictNotEqual => (4, BinaryOp::StrictNotEqual),
        TokenKind::Less => (5, BinaryOp::Less),
        TokenKind::LessEqual => (5, BinaryOp::LessEqual),
        TokenKind::Greater => (5, BinaryOp::Greater),
        TokenKind::GreaterEqual => (5, BinaryOp::GreaterEqual),
        TokenKind::Plus => (6, BinaryOp::Add),
        TokenKind::Minus => (6, BinaryOp::Subtract),
        TokenKind::Star => (7, BinaryOp::Multiply),
        TokenKind::Slash => (7, BinaryOp::Divide),
        TokenKind::Percent => (7, BinaryOp::Remainder),
        _ => return None,
    })
}

fn describe_token(token: &TokenKind) -> String {
    match token {
        TokenKind::Number(value) => format!("number {value}"),
        TokenKind::String(_) => "string".into(),
        TokenKind::Identifier(value) => format!("identifier {value}"),
        TokenKind::True => "true".into(),
        TokenKind::False => "false".into(),
        TokenKind::Null => "null".into(),
        TokenKind::End => "end of expression".into(),
        other => format!("{other:?}"),
    }
}

impl Expr {
    fn eval(&self, input: &Value, item_index: u64) -> Result<EvalValue, ExpressionError> {
        match self {
            Self::Literal(value) => Ok(EvalValue::Json(value.clone())),
            Self::Json => Ok(EvalValue::Json(input.clone())),
            Self::ItemIndex => Ok(EvalValue::Json(Value::Number(Number::from(item_index)))),
            Self::Array(values) => {
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    output.push(require_json(value.eval(input, item_index)?, 0)?);
                }
                Ok(EvalValue::Json(Value::Array(output)))
            }
            Self::Object(values) => {
                let mut output = Map::new();
                for (key, value) in values {
                    output.insert(
                        key.clone(),
                        require_json(value.eval(input, item_index)?, 0)?,
                    );
                }
                Ok(EvalValue::Json(Value::Object(output)))
            }
            Self::Access(base, accessor) => {
                let base = base.eval(input, item_index)?;
                access(base, accessor, input, item_index)
            }
            Self::Unary(operator, value) => {
                let value = value.eval(input, item_index)?;
                match operator {
                    UnaryOp::Not => Ok(EvalValue::Json(Value::Bool(!require_bool(value, 0)?))),
                    UnaryOp::Negate => {
                        let number = require_number(value, 0)?;
                        numeric_unary_minus(number, 0).map(EvalValue::Json)
                    }
                }
            }
            Self::Binary(left, BinaryOp::And, right) => {
                let left = require_bool(left.eval(input, item_index)?, 0)?;
                if !left {
                    return Ok(EvalValue::Json(Value::Bool(false)));
                }
                Ok(EvalValue::Json(Value::Bool(require_bool(
                    right.eval(input, item_index)?,
                    0,
                )?)))
            }
            Self::Binary(left, BinaryOp::Or, right) => {
                let left = require_bool(left.eval(input, item_index)?, 0)?;
                if left {
                    return Ok(EvalValue::Json(Value::Bool(true)));
                }
                Ok(EvalValue::Json(Value::Bool(require_bool(
                    right.eval(input, item_index)?,
                    0,
                )?)))
            }
            Self::Binary(left, BinaryOp::Coalesce, right) => {
                let left = left.eval(input, item_index)?;
                if matches!(left, EvalValue::Missing | EvalValue::Json(Value::Null)) {
                    right.eval(input, item_index)
                } else {
                    Ok(left)
                }
            }
            Self::Binary(left, operator, right) => {
                let left = left.eval(input, item_index)?;
                let right = right.eval(input, item_index)?;
                evaluate_binary(*operator, left, right, 0)
            }
            Self::Conditional(condition, when_true, when_false) => {
                if require_bool(condition.eval(input, item_index)?, 0)? {
                    when_true.eval(input, item_index)
                } else {
                    when_false.eval(input, item_index)
                }
            }
        }
    }
}

fn access(
    base: EvalValue,
    accessor: &Accessor,
    input: &Value,
    item_index: u64,
) -> Result<EvalValue, ExpressionError> {
    let base = match base {
        EvalValue::Missing => return Ok(EvalValue::Missing),
        EvalValue::Json(value) => value,
    };
    match accessor {
        Accessor::Property(property) => match base {
            Value::Object(object) => Ok(object
                .get(property)
                .cloned()
                .map(EvalValue::Json)
                .unwrap_or(EvalValue::Missing)),
            Value::Null => Err(ExpressionError::type_error(
                "cannot read a property from null",
                0,
            )),
            other => Err(ExpressionError::type_error(
                format!(
                    "property access requires an object, got {}",
                    value_type(&other)
                ),
                0,
            )),
        },
        Accessor::Index(index) => {
            let index = index.eval(input, item_index)?;
            let index = match index {
                EvalValue::Json(Value::Number(number)) => number.as_u64().ok_or_else(|| {
                    ExpressionError::type_error("array index must be a non-negative integer", 0)
                })?,
                EvalValue::Json(Value::String(key)) => {
                    return match base {
                        Value::Object(object) => Ok(object
                            .get(&key)
                            .cloned()
                            .map(EvalValue::Json)
                            .unwrap_or(EvalValue::Missing)),
                        _ => Err(ExpressionError::type_error(
                            "string indexing requires an object",
                            0,
                        )),
                    };
                }
                EvalValue::Missing => return Ok(EvalValue::Missing),
                EvalValue::Json(other) => {
                    return Err(ExpressionError::type_error(
                        format!(
                            "index must be a number or string, got {}",
                            value_type(&other)
                        ),
                        0,
                    ));
                }
            };
            match base {
                Value::Array(array) => Ok(array
                    .get(index as usize)
                    .cloned()
                    .map(EvalValue::Json)
                    .unwrap_or(EvalValue::Missing)),
                Value::Object(object) => Ok(object
                    .get(&index.to_string())
                    .cloned()
                    .map(EvalValue::Json)
                    .unwrap_or(EvalValue::Missing)),
                Value::Null => Err(ExpressionError::type_error("cannot index null", 0)),
                other => Err(ExpressionError::type_error(
                    format!(
                        "indexing requires an array or object, got {}",
                        value_type(&other)
                    ),
                    0,
                )),
            }
        }
    }
}

fn evaluate_binary(
    operator: BinaryOp,
    left: EvalValue,
    right: EvalValue,
    offset: usize,
) -> Result<EvalValue, ExpressionError> {
    if matches!(left, EvalValue::Missing) || matches!(right, EvalValue::Missing) {
        return Err(ExpressionError::missing(
            "a missing value can only be consumed by ??",
            offset,
        ));
    }
    let (EvalValue::Json(left), EvalValue::Json(right)) = (left, right) else {
        unreachable!()
    };
    match operator {
        BinaryOp::StrictEqual => Ok(EvalValue::Json(Value::Bool(strict_equal(&left, &right)))),
        BinaryOp::StrictNotEqual => Ok(EvalValue::Json(Value::Bool(!strict_equal(&left, &right)))),
        BinaryOp::Add => numeric_or_string_add(left, right, offset).map(EvalValue::Json),
        BinaryOp::Subtract => {
            numeric_operation(left, right, NumericOperation::Subtract, offset).map(EvalValue::Json)
        }
        BinaryOp::Multiply => {
            numeric_operation(left, right, NumericOperation::Multiply, offset).map(EvalValue::Json)
        }
        BinaryOp::Divide => {
            numeric_operation(left, right, NumericOperation::Divide, offset).map(EvalValue::Json)
        }
        BinaryOp::Remainder => {
            numeric_operation(left, right, NumericOperation::Remainder, offset).map(EvalValue::Json)
        }
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            compare(left, right, operator, offset).map(EvalValue::Json)
        }
        BinaryOp::And | BinaryOp::Or | BinaryOp::Coalesce => unreachable!(),
    }
}

fn require_json(value: EvalValue, offset: usize) -> Result<Value, ExpressionError> {
    match value {
        EvalValue::Json(value) => Ok(value),
        EvalValue::Missing => Err(ExpressionError::missing(
            "missing cannot be embedded in an array or object literal",
            offset,
        )),
    }
}

fn require_bool(value: EvalValue, offset: usize) -> Result<bool, ExpressionError> {
    match value {
        EvalValue::Json(Value::Bool(value)) => Ok(value),
        EvalValue::Missing => Err(ExpressionError::missing(
            "a missing value cannot be used as a boolean",
            offset,
        )),
        EvalValue::Json(value) => Err(ExpressionError::type_error(
            format!("expected boolean, got {}", value_type(&value)),
            offset,
        )),
    }
}

fn require_number(value: EvalValue, offset: usize) -> Result<Number, ExpressionError> {
    match value {
        EvalValue::Json(Value::Number(value)) => Ok(value),
        EvalValue::Missing => Err(ExpressionError::missing(
            "a missing value cannot be used as a number",
            offset,
        )),
        EvalValue::Json(value) => Err(ExpressionError::type_error(
            format!("expected number, got {}", value_type(&value)),
            offset,
        )),
    }
}

fn strict_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => left == right,
        (Value::Object(left), Value::Object(right)) => left == right,
        _ => left == right,
    }
}

fn numeric_or_string_add(
    left: Value,
    right: Value,
    offset: usize,
) -> Result<Value, ExpressionError> {
    match (left, right) {
        (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
        (Value::String(left), Value::Number(right)) => {
            let right = canonical_integer_string(&right).ok_or_else(|| {
                ExpressionError::type_error(
                    "string concatenation only converts JSON integers",
                    offset,
                )
            })?;
            Ok(Value::String(left + &right))
        }
        (Value::String(_), right) => Err(ExpressionError::type_error(
            format!(
                "string concatenation requires a string or JSON integer, got {}",
                value_type(&right)
            ),
            offset,
        )),
        (left, Value::String(_)) => Err(ExpressionError::type_error(
            format!(
                "numeric-to-string conversion is only allowed on the right operand, got {}",
                value_type(&left)
            ),
            offset,
        )),
        (left, right) => numeric_operation(left, right, NumericOperation::Add, offset),
    }
}

fn canonical_integer_string(number: &Number) -> Option<String> {
    number
        .as_i64()
        .map(|value| value.to_string())
        .or_else(|| number.as_u64().map(|value| value.to_string()))
}

#[derive(Clone, Copy)]
enum NumericOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

fn numeric_operation(
    left: Value,
    right: Value,
    operation: NumericOperation,
    offset: usize,
) -> Result<Value, ExpressionError> {
    let left_number = match &left {
        Value::Number(number) => number,
        _ => {
            return Err(ExpressionError::type_error(
                format!(
                    "numeric operation requires numbers, got {}",
                    value_type(&left)
                ),
                offset,
            ));
        }
    };
    let right_number = match &right {
        Value::Number(number) => number,
        _ => {
            return Err(ExpressionError::type_error(
                format!(
                    "numeric operation requires numbers, got {}",
                    value_type(&right)
                ),
                offset,
            ));
        }
    };
    if !matches!(operation, NumericOperation::Divide) {
        if let (Some(left), Some(right)) = (left_number.as_i64(), right_number.as_i64()) {
            let result = match operation {
                NumericOperation::Add => left.checked_add(right),
                NumericOperation::Subtract => left.checked_sub(right),
                NumericOperation::Multiply => left.checked_mul(right),
                NumericOperation::Remainder => {
                    if right == 0 {
                        return Err(ExpressionError::overflow("remainder by zero", offset));
                    }
                    left.checked_rem(right)
                }
                NumericOperation::Divide => unreachable!(),
            }
            .ok_or_else(|| {
                ExpressionError::overflow("checked integer arithmetic overflowed", offset)
            })?;
            return Ok(Value::Number(Number::from(result)));
        }
    }
    let left = left_number.as_f64().ok_or_else(|| {
        ExpressionError::overflow("left number cannot be represented safely", offset)
    })?;
    let right = right_number.as_f64().ok_or_else(|| {
        ExpressionError::overflow("right number cannot be represented safely", offset)
    })?;
    if matches!(
        operation,
        NumericOperation::Divide | NumericOperation::Remainder
    ) && right == 0.0
    {
        return Err(ExpressionError::overflow("division by zero", offset));
    }
    let result = match operation {
        NumericOperation::Add => left + right,
        NumericOperation::Subtract => left - right,
        NumericOperation::Multiply => left * right,
        NumericOperation::Divide => left / right,
        NumericOperation::Remainder => left % right,
    };
    Number::from_f64(result)
        .map(Value::Number)
        .ok_or_else(|| ExpressionError::overflow("numeric result is not finite", offset))
}

fn numeric_unary_minus(number: Number, offset: usize) -> Result<Value, ExpressionError> {
    if let Some(value) = number.as_i64() {
        return value
            .checked_neg()
            .map(|value| Value::Number(Number::from(value)))
            .ok_or_else(|| {
                ExpressionError::overflow("checked integer arithmetic overflowed", offset)
            });
    }
    let value = number
        .as_f64()
        .ok_or_else(|| ExpressionError::overflow("number cannot be represented safely", offset))?;
    Number::from_f64(-value)
        .map(Value::Number)
        .ok_or_else(|| ExpressionError::overflow("numeric result is not finite", offset))
}

fn compare(
    left: Value,
    right: Value,
    operator: BinaryOp,
    offset: usize,
) -> Result<Value, ExpressionError> {
    let ordering = match (&left, &right) {
        (Value::Number(left), Value::Number(right)) => left
            .as_f64()
            .and_then(|left| right.as_f64().map(|right| left.total_cmp(&right))),
        (Value::String(left), Value::String(right)) => Some(left.cmp(right)),
        _ => None,
    }
    .ok_or_else(|| {
        ExpressionError::type_error("comparisons require two numbers or two strings", offset)
    })?;
    let result = match operator {
        BinaryOp::Less => ordering.is_lt(),
        BinaryOp::LessEqual => ordering.is_le(),
        BinaryOp::Greater => ordering.is_gt(),
        BinaryOp::GreaterEqual => ordering.is_ge(),
        _ => unreachable!(),
    };
    Ok(Value::Bool(result))
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

impl FromStr for Program {
    type Err = ExpressionError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        compile(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn evaluate(source: &str, input: Value, index: u64) -> Value {
        compile(source)
            .unwrap()
            .evaluate(&input, index)
            .unwrap()
            .into_option()
            .unwrap()
    }

    #[test]
    fn evaluates_the_frozen_eco_expression_set() {
        let input = json!({"index": 3, "value": 7, "nested": {"name": "Ada"}});
        assert_eq!(
            evaluate(
                "$json.value % 2 === 0 ? \"even\" : \"odd\"",
                input.clone(),
                0
            ),
            json!("odd")
        );
        assert_eq!(evaluate("$json.value * 2", input.clone(), 0), json!(14));
        assert_eq!(evaluate("\"eco-\" + $json.index", input, 3), json!("eco-3"));
        assert_eq!(
            evaluate("\"eco-\" + $itemIndex", json!({}), 3),
            json!("eco-3")
        );
    }

    #[test]
    fn preserves_types_and_handles_literals_and_access() {
        let input = json!({"list": ["a", {"ok": true}], "missing": null});
        assert_eq!(evaluate("$json.list[1].ok", input.clone(), 0), json!(true));
        assert_eq!(
            evaluate("$json.nope ?? {\"fallback\": [1, 2]}", input.clone(), 0),
            json!({"fallback": [1, 2]})
        );
        assert_eq!(evaluate("$json.missing ?? false", input, 0), json!(false));
    }

    #[test]
    fn missing_is_distinct_and_unsafe_javascript_is_rejected() {
        let program = compile("$json.nope").unwrap();
        assert_eq!(program.evaluate(&json!({}), 0).unwrap(), EvalValue::Missing);
        for source in [
            "$json.value.toString()",
            "eval(\"x\")",
            "a = 1",
            "$json.value == 1",
        ] {
            let error = compile(source).unwrap_err();
            assert!(
                error.code == "canopy.expression.unsupported"
                    || error.code == "canopy.expression.syntax",
                "{source}: {error:?}"
            );
        }
    }

    #[test]
    fn checked_arithmetic_and_types_are_stable_errors() {
        let overflow = compile("9223372036854775807 + 1")
            .unwrap()
            .evaluate(&json!({}), 0)
            .unwrap_err();
        assert_eq!(overflow.code, "canopy.expression.overflow");
        let type_error = compile("$json.value + true")
            .unwrap()
            .evaluate(&json!({"value": 1}), 0)
            .unwrap_err();
        assert_eq!(type_error.code, "canopy.expression.type");
        let decimal_conversion = compile("\"eco-\" + 1.5")
            .unwrap()
            .evaluate(&json!({}), 0)
            .unwrap_err();
        assert_eq!(decimal_conversion.code, "canopy.expression.type");
        let reversed_conversion = compile("$itemIndex + \"-eco\"")
            .unwrap()
            .evaluate(&json!({}), 3)
            .unwrap_err();
        assert_eq!(reversed_conversion.code, "canopy.expression.type");
    }
}
