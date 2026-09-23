#![allow(dead_code)]
//! شجرة الكود المجردة (AST) للغة Rino.

#[derive(Debug, Clone)]
pub struct Program {
    pub page_title: Option<Expression>,
    pub styles: Vec<StyleRule>,
    pub state: Vec<Statement>,
    pub functions: Vec<Statement>,
    pub components: Vec<Statement>,
    pub body: Vec<Statement>,
    pub top_level: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub selector_kind: SelectorKind,
    pub properties: Vec<(String, String)>,
    pub hover_properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorKind {
    Tag,    // h1, p, button
    Class,  // .بطاقة
    Id,     // #رئيسي
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let { name: String, value: Expression, is_state: bool, line: usize },
    Const { name: String, value: Expression, line: usize },
    Assignment { name: String, value: Expression, line: usize },
    Function { name: String, params: Vec<String>, body: Vec<Statement>, line: usize },
    ComponentDef { name: String, params: Vec<String>, body: Vec<Statement>, line: usize },
    Return { value: Option<Expression>, line: usize },
    If { condition: Expression, then_branch: Vec<Statement>, else_branch: Vec<Statement>, line: usize },
    ForEach { var: String, iterable: Expression, body: Vec<Statement>, line: usize },
    HtmlElement {
        tag: String,
        content: Option<Expression>,
        attrs: Vec<(String, Expression)>,
        children: Option<Vec<Statement>>,
        events: Vec<Event>,
        line: usize,
    },
    Call { name: String, args: Vec<Expression>, line: usize },
}

#[derive(Debug, Clone)]
pub struct Event {
    pub kind: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Identifier(String),
    List(Vec<Expression>),
    Call { name: String, args: Vec<Expression> },
    Binary { left: Box<Expression>, op: BinOp, right: Box<Expression> },
    Comparison { left: Box<Expression>, op: CmpOp, right: Box<Expression> },
    Logical { left: Box<Expression>, op: LogOp, right: Box<Expression> },
    Not(Box<Expression>),
}

#[derive(Debug, Clone)]
pub enum BinOp { Add, Sub, Mul, Div, Mod }

#[derive(Debug, Clone)]
pub enum CmpOp { Eq, Ne, Gt, Lt, Ge, Le }

#[derive(Debug, Clone)]
pub enum LogOp { And, Or }