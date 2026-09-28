#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct Program {
    pub page_title: Option<Expression>,
    pub type_aliases: Vec<TypeAlias>,   // ← جديد
    pub styles: Vec<StyleRule>,
    pub state: Vec<Statement>,
    pub derived: Vec<Statement>,
    pub functions: Vec<Statement>,
    pub components: Vec<Statement>,
    pub tests: Vec<Statement>,
    pub body: Vec<Statement>,
    pub top_level: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub definition: Type,
    pub line: usize,
}

// ==========================================
// نظام الأنواع
// ==========================================
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Text,                           // نص
    Number,                         // عدد
    Boolean,                        // منطقي
    Nothing,                        // لا_شيء
    Any,                            // غير محدّد (للتوافق)
    List(Box<Type>),                // قائمة<T>
    Dict(Vec<(String, Type)>),      // { اسم: نص }
    Named(String),                  // نوع مستخدم
    Optional(Box<Type>),            // نص؟
    Function(Vec<Type>, Box<Type>), // (عدد) -> عدد
}

impl Type {
    pub fn display_ar(&self) -> String {
        match self {
            Type::Text => "نص".into(),
            Type::Number => "عدد".into(),
            Type::Boolean => "منطقي".into(),
            Type::Nothing => "لا_شيء".into(),
            Type::Any => "أي".into(),
            Type::List(inner) => format!("قائمة<{}>", inner.display_ar()),
            Type::Dict(pairs) => {
                let p: Vec<String> = pairs.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.display_ar()))
                    .collect();
                format!("{{{}}}", p.join(", "))
            }
            Type::Named(n) => n.clone(),
            Type::Optional(inner) => format!("{}؟", inner.display_ar()),
            Type::Function(args, ret) => {
                let a: Vec<String> = args.iter().map(|t| t.display_ar()).collect();
                format!("({}) -> {}", a.join(", "), ret.display_ar())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub selector_kind: SelectorKind,
    pub properties: Vec<(String, String)>,
    pub hover_properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorKind { Tag, Class, Id }

#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        name: String,
        value: Expression,
        is_state: bool,
        type_ann: Option<Type>,   // ← جديد
        line: usize,
    },
    Const {
        name: String,
        value: Expression,
        type_ann: Option<Type>,   // ← جديد
        line: usize,
    },
    Assignment { name: String, value: Expression, line: usize },
    Function {
        name: String,
        params: Vec<String>,
        param_types: Vec<Option<Type>>,   // ← جديد
        return_type: Option<Type>,        // ← جديد
        body: Vec<Statement>,
        line: usize,
    },
    ComponentDef {
        name: String,
        params: Vec<String>,
        param_types: Vec<Option<Type>>,   // ← جديد
        body: Vec<Statement>,
        line: usize,
    },
    Test { name: String, body: Vec<Statement>, line: usize },
    Return { value: Option<Expression>, line: usize },
    If { condition: Expression, then_branch: Vec<Statement>, else_branch: Vec<Statement>, line: usize },
    ForEach { var: String, iterable: Expression, body: Vec<Statement>, line: usize },
    RangeFor { var: String, start: Expression, end: Expression, step: Option<Expression>, body: Vec<Statement>, line: usize },
    While { condition: Expression, body: Vec<Statement>, line: usize },
    Break { line: usize },
    Continue { line: usize },
    TryCatch { try_body: Vec<Statement>, catch_var: String, catch_body: Vec<Statement>, line: usize },
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
pub struct Event { pub kind: String, pub body: Vec<Statement> }

#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Identifier(String),
    List(Vec<Expression>),
    Dict(Vec<(String, Expression)>),
    MemberAccess { object: Box<Expression>, property: String },
    Index { object: Box<Expression>, index: Box<Expression> },
    Call { name: String, args: Vec<Expression> },
    Binary { left: Box<Expression>, op: BinOp, right: Box<Expression> },
    Comparison { left: Box<Expression>, op: CmpOp, right: Box<Expression> },
    Logical { left: Box<Expression>, op: LogOp, right: Box<Expression> },
    Not(Box<Expression>),
    Neg(Box<Expression>),
}

#[derive(Debug, Clone)]
pub enum BinOp { Add, Sub, Mul, Div, Mod }

#[derive(Debug, Clone)]
pub enum CmpOp { Eq, Ne, Gt, Lt, Ge, Le }

#[derive(Debug, Clone)]
pub enum LogOp { And, Or }