#![allow(dead_code)]
//! شجرة الكود المجردة (AST) للغة Rino.

#[derive(Debug, Clone)]
pub struct Program {
    pub page_title: Option<Expression>,
    pub styles: Vec<StyleRule>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,           // "h1", "p", "button"
    pub properties: Vec<(String, String)>, // [("color", "red"), ...]
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        name: String,
        value: Expression,
        line: usize,
    },
    HtmlElement {
        tag: String,
        content: Expression,
        events: Vec<Event>,
        line: usize,
    },
    Call {
        name: String,
        args: Vec<Expression>,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Event {
    pub kind: String,      // "click", "change"
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}