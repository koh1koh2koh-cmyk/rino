//! Rino — لغة عربية لبناء الويب.
//! الإصدار 0.6 — نظام الاستيراد

mod ast;
mod correction;
mod lexer;
mod parser;
mod token;

mod formatter {
    pub struct Formatter {
        indent: usize,
        indent_size: usize,
    }
    impl Formatter {
        pub fn new() -> Self {
            Self { indent: 0, indent_size: 2 }
        }
        pub fn format(&mut self, source: &str) -> String {
            let mut output = String::new();
            let mut in_string = false;
            let mut current_line = String::new();
            let mut last_char: Option<char> = None;
            for ch in source.chars() {
                if ch == '"' && last_char != Some('\\') { in_string = !in_string; }
                if in_string {
                    current_line.push(ch);
                    last_char = Some(ch);
                    continue;
                }
                match ch {
                    '{' => {
                        current_line.push('{');
                        self.flush_line(&mut output, &mut current_line);
                        self.indent += 1;
                    }
                    '}' => {
                        if !current_line.trim().is_empty() {
                            self.flush_line(&mut output, &mut current_line);
                        }
                        self.indent = self.indent.saturating_sub(1);
                        current_line.push('}');
                    }
                    '\n' => {
                        if !current_line.trim().is_empty() {
                            self.flush_line(&mut output, &mut current_line);
                        }
                    }
                    _ => { current_line.push(ch); }
                }
                last_char = Some(ch);
            }
            if !current_line.trim().is_empty() {
                self.flush_line(&mut output, &mut current_line);
            }
            if !output.ends_with('\n') { output.push('\n'); }
            output
        }
        fn flush_line(&mut self, output: &mut String, line: &mut String) {
            let trimmed = line.trim();
            if trimmed.is_empty() { line.clear(); return; }
            for _ in 0..self.indent {
                for _ in 0..self.indent_size { output.push(' '); }
            }
            output.push_str(trimmed);
            output.push('\n');
            line.clear();
        }
    }
}

use ast::*;
use lexer::Lexer;
use parser::Parser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

// ========== نظام الاستيراد ==========

/// يقرأ ملف Rino، ويعالج كل `استيراد "..."` فيه بشكل تكراري.
fn process_imports(
    source: &str,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<String, String> {
    if depth > 20 {
        return Err("عدد الاستيرادات المتتالية كبير جدًا".into());
    }

    let mut output = String::new();

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("استيراد ") {
            let rest = trimmed["استيراد".len()..].trim();

            if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
                let rel_path = &rest[1..rest.len() - 1];
                let file_path = base_dir.join(rel_path);

                let canonical = file_path
                    .canonicalize()
                    .map_err(|_| format!("ملف غير موجود: {}", file_path.display()))?;

                if visited.contains(&canonical) {
                    // تجنّب التكرار
                    output.push('\n');
                    continue;
                }
                visited.insert(canonical.clone());

                let content = fs::read_to_string(&file_path)
                    .map_err(|e| format!("فشل قراءة {}: {}", file_path.display(), e))?;

                let new_base = file_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| base_dir.to_path_buf());

                let processed = process_imports(&content, &new_base, visited, depth + 1)?;
                output.push_str(&processed);
                output.push('\n');
            } else {
                return Err(format!(
                    "صيغة استيراد خاطئة: {} (الصحيح: استيراد \"مسار/ملف.rino\")",
                    trimmed
                ));
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    Ok(output)
}

// ========== تقييم وقت الترجمة ==========

#[derive(Debug, Clone)]
enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    List(Vec<Value>),
}

impl Value {
    fn to_display(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Num(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
            Value::Bool(b) => if *b { "صحيح" } else { "خطأ" }.to_string(),
            Value::Null => String::new(),
            Value::List(items) => items.iter().map(|v| v.to_display()).collect::<Vec<_>>().join("، "),
        }
    }
    fn as_num(&self) -> Result<f64, String> {
        match self {
            Value::Num(n) => Ok(*n),
            Value::Str(s) => s.parse().map_err(|_| format!("\"{}\" ليس عددًا", s)),
            _ => Err("قيمة غير رقمية".into()),
        }
    }
    fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Null => false,
            Value::List(v) => !v.is_empty(),
        }
    }
    fn to_js_literal(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Value::Num(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
            Value::Bool(b) => if *b { "true".into() } else { "false".into() },
            Value::Null => "null".into(),
            Value::List(items) => {
                let i: Vec<String> = items.iter().map(|x| x.to_js_literal()).collect();
                format!("[{}]", i.join(", "))
            }
        }
    }
}

fn eval(expr: &Expression, env: &HashMap<String, Value>) -> Result<Value, String> {
    match expr {
        Expression::String(s) => Ok(Value::Str(s.clone())),
        Expression::Number(n) => Ok(Value::Num(*n)),
        Expression::Boolean(b) => Ok(Value::Bool(*b)),
        Expression::Null => Ok(Value::Null),
        Expression::Identifier(name) => env.get(name).cloned()
            .ok_or_else(|| format!("المتغير \"{}\" غير معرّف", name)),
        Expression::List(items) => {
            let mut vs = Vec::new();
            for i in items { vs.push(eval(i, env)?); }
            Ok(Value::List(vs))
        }
        Expression::Call { name, args } => {
            let mut vs = Vec::new();
            for a in args { vs.push(eval(a, env)?); }
            match name.as_str() {
                "طول" => match vs.first() {
                    Some(Value::List(l)) => Ok(Value::Num(l.len() as f64)),
                    Some(Value::Str(s)) => Ok(Value::Num(s.chars().count() as f64)),
                    _ => Err("طول تحتاج قائمة أو نصًا".into()),
                },
                "كبير" => match vs.first() {
                    Some(Value::Str(s)) => Ok(Value::Str(s.to_uppercase())),
                    _ => Err("كبير تحتاج نصًا".into()),
                },
                "صغير" => match vs.first() {
                    Some(Value::Str(s)) => Ok(Value::Str(s.to_lowercase())),
                    _ => Err("صغير تحتاج نصًا".into()),
                },
                "يحتوي" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::Str(s)), Some(Value::Str(sub))) => Ok(Value::Bool(s.contains(sub.as_str()))),
                    _ => Err("يحتوي تحتاج نصين".into()),
                },
                "استبدل" => match (vs.get(0), vs.get(1), vs.get(2)) {
                    (Some(Value::Str(s)), Some(Value::Str(a)), Some(Value::Str(b))) => {
                        Ok(Value::Str(s.replace(a.as_str(), b.as_str())))
                    }
                    _ => Err("استبدل تحتاج 3 نصوص".into()),
                },
                "جذر" => match vs.first() {
                    Some(v) => Ok(Value::Num(v.as_num()?.sqrt())),
                    _ => Err("جذر تحتاج عددًا".into()),
                },
                "قوة" => match (vs.get(0), vs.get(1)) {
                    (Some(a), Some(b)) => Ok(Value::Num(a.as_num()?.powf(b.as_num()?))),
                    _ => Err("قوة تحتاج عددين".into()),
                },
                "قوس" => match vs.first() {
                    Some(v) => Ok(Value::Num(v.as_num()?.round())),
                    _ => Err("قوس تحتاج عددًا".into()),
                },
                "أرضي" => match vs.first() {
                    Some(v) => Ok(Value::Num(v.as_num()?.floor())),
                    _ => Err("أرضي تحتاج عددًا".into()),
                },
                "سقف" => match vs.first() {
                    Some(v) => Ok(Value::Num(v.as_num()?.ceil())),
                    _ => Err("سقف تحتاج عددًا".into()),
                },
                "مطلق" => match vs.first() {
                    Some(v) => Ok(Value::Num(v.as_num()?.abs())),
                    _ => Err("مطلق تحتاج عددًا".into()),
                },
                "أصغر" => match (vs.get(0), vs.get(1)) {
                    (Some(a), Some(b)) => {
                        let (x, y) = (a.as_num()?, b.as_num()?);
                        Ok(Value::Num(if x < y { x } else { y }))
                    }
                    _ => Err("أصغر تحتاج عددين".into()),
                },
                "أكبر" => match (vs.get(0), vs.get(1)) {
                    (Some(a), Some(b)) => {
                        let (x, y) = (a.as_num()?, b.as_num()?);
                        Ok(Value::Num(if x > y { x } else { y }))
                    }
                    _ => Err("أكبر تحتاج عددين".into()),
                },
                "عدد" => match vs.first() {
                    Some(v) => Ok(Value::Num(v.as_num()?)),
                    _ => Err("عدد تحتاج قيمة".into()),
                },
                _ => Err(format!("دالة غير معروفة: {}", name)),
            }
        }
        Expression::Binary { left, op, right } => {
            let l = eval(left, env)?;
            let r = eval(right, env)?;
            match op {
                BinOp::Add => match (&l, &r) {
                    (Value::Num(a), Value::Num(b)) => Ok(Value::Num(a + b)),
                    _ => Ok(Value::Str(l.to_display() + &r.to_display())),
                },
                BinOp::Sub => Ok(Value::Num(l.as_num()? - r.as_num()?)),
                BinOp::Mul => Ok(Value::Num(l.as_num()? * r.as_num()?)),
                BinOp::Div => {
                    let b = r.as_num()?;
                    if b == 0.0 { Err("القسمة على صفر".into()) }
                    else { Ok(Value::Num(l.as_num()? / b)) }
                }
                BinOp::Mod => Ok(Value::Num(l.as_num()? % r.as_num()?)),
            }
        }
        Expression::Comparison { left, op, right } => {
            let l = eval(left, env)?;
            let r = eval(right, env)?;
            let res = match op {
                CmpOp::Eq => l.to_display() == r.to_display(),
                CmpOp::Ne => l.to_display() != r.to_display(),
                CmpOp::Gt => l.as_num()? > r.as_num()?,
                CmpOp::Lt => l.as_num()? < r.as_num()?,
                CmpOp::Ge => l.as_num()? >= r.as_num()?,
                CmpOp::Le => l.as_num()? <= r.as_num()?,
            };
            Ok(Value::Bool(res))
        }
        Expression::Logical { left, op, right } => {
            let l = eval(left, env)?;
            match op {
                LogOp::And => if !l.as_bool() { Ok(Value::Bool(false)) }
                    else { Ok(Value::Bool(eval(right, env)?.as_bool())) },
                LogOp::Or => if l.as_bool() { Ok(Value::Bool(true)) }
                    else { Ok(Value::Bool(eval(right, env)?.as_bool())) },
            }
        }
        Expression::Not(e) => Ok(Value::Bool(!eval(e, env)?.as_bool())),
    }
}

fn css_property(name: &str) -> &str {
    match name {
        "لون" => "color",
        "خلفية" => "background",
        "حجم" => "font-size",
        "حشوة" => "padding",
        "هامش" => "margin",
        "استدارة" => "border-radius",
        "محاذاة" => "text-align",
        "عرض" => "width",
        "ارتفاع" => "height",
        "حد" => "border",
        "وزن" => "font-weight",
        "ظل" => "box-shadow",
        "اتجاه" => "direction",
        "تحويل" => "text-transform",
        "مسافة_بين_الأسطر" => "line-height",
        _ => name,
    }
}

fn css_value(val: &str) -> String {
    match val {
        "أحمر" => "red".into(),
        "أزرق" => "blue".into(),
        "أخضر" => "green".into(),
        "أبيض" => "white".into(),
        "أسود" => "black".into(),
        "رمادي" => "gray".into(),
        "أصفر" => "yellow".into(),
        "برتقالي" => "orange".into(),
        "بنفسجي" => "purple".into(),
        "وردي" => "pink".into(),
        "بني" => "brown".into(),
        "ذهبي" => "gold".into(),
        "فضي" => "silver".into(),
        "سماوي" => "skyblue".into(),
        "ليموني" => "lime".into(),
        "أرجواني" => "magenta".into(),
        "نيلي" => "navy".into(),
        "كريمي" => "beige".into(),
        "شفاف" => "transparent".into(),
        "تركوازي" => "turquoise".into(),
        "مرجاني" => "coral".into(),
        "وسط" => "center".into(),
        "يمين" => "right".into(),
        "يسار" => "left".into(),
        "عريض" => "bold".into(),
        "ضعيف" => "lighter".into(),
        "مائل" => "italic".into(),
        _ => if val.parse::<f64>().is_ok() { format!("{}px", val) } else { val.to_string() },
    }
}

struct Codegen {
    counter: usize,
    events_js: String,
    updates_js: String,
}

impl Codegen {
    fn new() -> Self {
        Self { counter: 0, events_js: String::new(), updates_js: String::new() }
    }

    fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("r{}", self.counter)
    }

    fn is_reactive(&self, expr: &Expression, state_vars: &[String]) -> bool {
        match expr {
            Expression::Identifier(n) => state_vars.contains(n),
            Expression::Binary { left, right, .. }
            | Expression::Comparison { left, right, .. }
            | Expression::Logical { left, right, .. } => {
                self.is_reactive(left, state_vars) || self.is_reactive(right, state_vars)
            }
            Expression::Not(e) => self.is_reactive(e, state_vars),
            Expression::List(items) => items.iter().any(|i| self.is_reactive(i, state_vars)),
            Expression::Call { args, .. } => args.iter().any(|a| self.is_reactive(a, state_vars)),
            _ => false,
        }
    }

    fn expr_to_js(&self, expr: &Expression) -> String {
        match expr {
            Expression::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Expression::Number(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
            Expression::Boolean(b) => if *b { "true".into() } else { "false".into() },
            Expression::Null => "null".into(),
            Expression::Identifier(n) => format!("حالة.{}", n),
            Expression::List(items) => {
                let list: Vec<String> = items.iter().map(|i| self.expr_to_js(i)).collect();
                format!("[{}]", list.join(", "))
            }
            Expression::Call { name, args } => {
                if name == "اقرأ_مدخل" && args.is_empty() {
                    return "this.value".to_string();
                }
                let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x)).collect();
                format!("{}({})", name, a.join(", "))
            }
            Expression::Binary { left, op, right } => {
                let op_str = match op {
                    BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
                    BinOp::Div => "/", BinOp::Mod => "%",
                };
                format!("({} {} {})", self.expr_to_js(left), op_str, self.expr_to_js(right))
            }
            Expression::Comparison { left, op, right } => {
                let op_str = match op {
                    CmpOp::Eq => "===", CmpOp::Ne => "!==",
                    CmpOp::Gt => ">", CmpOp::Lt => "<",
                    CmpOp::Ge => ">=", CmpOp::Le => "<=",
                };
                format!("({} {} {})", self.expr_to_js(left), op_str, self.expr_to_js(right))
            }
            Expression::Logical { left, op, right } => {
                let op_str = match op { LogOp::And => "&&", LogOp::Or => "||" };
                format!("({} {} {})", self.expr_to_js(left), op_str, self.expr_to_js(right))
            }
            Expression::Not(e) => format!("(!{})", self.expr_to_js(e)),
        }
    }

    fn stmt_to_js(&mut self, stmt: &Statement, indent: &str) -> Result<String, String> {
        match stmt {
            Statement::Let { name, value, .. } => Ok(format!("{}let {} = {};\n", indent, name, self.expr_to_js(value))),
            Statement::Const { name, value, .. } => Ok(format!("{}const {} = {};\n", indent, name, self.expr_to_js(value))),
            Statement::Assignment { name, value, .. } => Ok(format!("{}حالة.{} = {};\n", indent, name, self.expr_to_js(value))),
            Statement::Return { value, .. } => match value {
                Some(v) => Ok(format!("{}return {};\n", indent, self.expr_to_js(v))),
                None => Ok(format!("{}return;\n", indent)),
            },
            Statement::If { condition, then_branch, else_branch, .. } => {
                let mut s = format!("{}if ({}) {{\n", indent, self.expr_to_js(condition));
                for st in then_branch { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                s.push_str(&format!("{}}}", indent));
                if !else_branch.is_empty() {
                    s.push_str(" else {\n");
                    for st in else_branch { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                    s.push_str(&format!("{}}}", indent));
                }
                s.push('\n');
                Ok(s)
            }
            Statement::Call { name, args, .. } => {
                if name == "_skip_" { return Ok(String::new()); }
                let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x)).collect();
                if name == "اطبع" {
                    Ok(format!("{}console.log({});\n", indent, a.join(", ")))
                } else {
                    Ok(format!("{}{}({});\n", indent, name, a.join(", ")))
                }
            }
            Statement::ForEach { var, iterable, body, .. } => {
                let mut s = format!("{}for (let {} of {}) {{\n", indent, var, self.expr_to_js(iterable));
                for st in body { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            _ => Ok(String::new()),
        }
    }

    fn gen_html(
        &mut self,
        stmt: &Statement,
        env: &mut HashMap<String, Value>,
        state_vars: &[String],
    ) -> Result<String, String> {
        match stmt {
            Statement::HtmlElement { tag, content, attrs, children, events, .. } => {
                let mut attr_str = String::new();
                let mut existing_id: Option<String> = None;
                for (k, v) in attrs {
                    let val = eval(v, env)?.to_display();
                    if k == "id" {
                        existing_id = Some(val.clone());
                    }
                    attr_str.push_str(&format!(" {}=\"{}\"", k, val));
                }

                let needs_reactive = content.as_ref()
                    .map(|c| self.is_reactive(c, state_vars))
                    .unwrap_or(false);
                let needs_id = needs_reactive || !events.is_empty();
                let id = if needs_id && existing_id.is_none() {
                    Some(self.next_id())
                } else {
                    existing_id.clone()
                };

                if existing_id.is_none() {
                    if let Some(ref i) = id {
                        attr_str.push_str(&format!(" id=\"{}\"", i));
                    }
                }

                let (open, close, self_closing) = match tag.as_str() {
                    "img" | "input" | "br" => (format!("<{}{}>", tag, attr_str), String::new(), true),
                    _ => (format!("<{}{}>", tag, attr_str), format!("</{}>", tag), false),
                };

                let inner = if self_closing {
                    String::new()
                } else if let Some(ref ch) = children {
                    let mut s = String::new();
                    for c in ch { s.push_str(&self.gen_html(c, env, state_vars)?); }
                    s
                } else if let Some(c) = content {
                    if needs_reactive {
                        if let Some(ref i) = id {
                            self.updates_js.push_str(&format!(
                                "  document.getElementById('{}').textContent = {};\n",
                                i, self.expr_to_js(c)
                            ));
                        }
                        String::new()
                    } else {
                        eval(c, env)?.to_display()
                    }
                } else { String::new() };

                for ev in events {
                    if let Some(ref i) = id {
                        let mut body_js = String::new();
                        for s in &ev.body { body_js.push_str(&self.stmt_to_js(s, "  ")?); }
                        self.events_js.push_str(&format!(
                            "document.getElementById('{}').addEventListener('{}', function() {{\n{}}});\n",
                            i, ev.kind, body_js
                        ));
                    }
                }
                Ok(format!("{}{}{}", open, inner, close))
            }

            Statement::If { condition, then_branch, else_branch, .. } => {
                let needs_reactive = self.is_reactive(condition, state_vars);
                if needs_reactive {
                    let mut then_html = String::new();
                    for st in then_branch {
                        then_html.push_str(&self.gen_html(st, env, state_vars)?);
                    }
                    let mut else_html = String::new();
                    for st in else_branch {
                        else_html.push_str(&self.gen_html(st, env, state_vars)?);
                    }
                    let id = self.next_id();
                    let cond_js = self.expr_to_js(condition);
                    let then_escaped = then_html.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
                    let else_escaped = else_html.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
                    self.updates_js.push_str(&format!(
                        "  {{ const _c = {}; const _e = document.getElementById('{}'); if (_e) _e.innerHTML = _c ? `{}` : `{}`; }}\n",
                        cond_js, id, then_escaped, else_escaped
                    ));
                    Ok(format!("<div id=\"{}\"></div>", id))
                } else {
                    let cond_val = eval(condition, env)?.as_bool();
                    let branch = if cond_val { then_branch } else { else_branch };
                    let mut s = String::new();
                    for st in branch {
                        s.push_str(&self.gen_html(st, env, state_vars).unwrap_or_default());
                    }
                    Ok(s)
                }
            }

            Statement::ForEach { var, iterable, body, .. } => {
                let iterable_val = eval(iterable, env)?;
                if let Value::List(items) = iterable_val {
                    let mut s = String::new();
                    let old_env = env.clone();
                    for item in items {
                        env.insert(var.clone(), item);
                        for st in body {
                            s.push_str(&self.gen_html(st, env, state_vars).unwrap_or_default());
                        }
                    }
                    *env = old_env;
                    Ok(s)
                } else { Ok(String::new()) }
            }

            Statement::Call { name, .. } if name == "_skip_" => Ok(String::new()),
            _ => Ok(String::new()),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // ========== أمر التنسيق ==========
    if args.len() > 2 && args[1] == "format" {
        let file_path = PathBuf::from(&args[2]);
        let source = match fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(e) => { eprintln!("❌ فشل قراءة الملف: {}", e); std::process::exit(1); }
        };
        let mut fmt = formatter::Formatter::new();
        let formatted = fmt.format(&source);
        if let Err(e) = fs::write(&file_path, &formatted) {
            eprintln!("❌ فشل الحفظ: {}", e);
            std::process::exit(1);
        }
        println!("✅ تم تنسيق: {}", file_path.display());
        return;
    }

    // ========== تحليل المسار ==========
    let input_path: PathBuf = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("index.rino")
    };

    if !input_path.exists() {
        eprintln!("❌ الملف غير موجود: {}", input_path.display());
        eprintln!();
        eprintln!("الاستخدام:");
        eprintln!("   rino <ملف.rino>         — بناء الملف");
        eprintln!("   rino format <ملف.rino>  — تنسيق الملف");
        std::process::exit(1);
    }

    let input_dir = input_path.parent()
        .map(|p| if p.as_os_str().is_empty() { PathBuf::from(".") } else { p.to_path_buf() })
        .unwrap_or_else(|| PathBuf::from("."));

    let base_name = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("index")
        .to_string();

    let output_html_path = input_dir.join(format!("{}.html", base_name));
    let output_css_path  = input_dir.join(format!("{}.css", base_name));
    let output_js_path   = input_dir.join(format!("{}.js", base_name));

    let html_filename = format!("{}.html", base_name);
    let css_filename  = format!("{}.css", base_name);
    let js_filename   = format!("{}.js", base_name);

    println!("📂 {}", input_path.display());

    let source = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    };

    // ========== معالجة الاستيرادات ==========
    let mut visited = HashSet::new();
    if let Ok(canonical) = input_path.canonicalize() {
        visited.insert(canonical);
    }
    let source = match process_imports(&source, &input_dir, &mut visited, 0) {
        Ok(s) => s,
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { eprintln!("❌ خطأ لغوي: {}", e); std::process::exit(1); }
    };
    if !lexer.corrections.is_empty() {
        println!("📋 تصحيحات:");
        for c in &lexer.corrections { println!("  {}", c); }
    }

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => { eprintln!("❌ خطأ نحوي: {}", e); std::process::exit(1); }
    };

    let mut state_vars: Vec<String> = Vec::new();
    let mut env: HashMap<String, Value> = HashMap::new();

    for s in &program.state {
        if let Statement::Let { name, value, .. } = s {
            state_vars.push(name.clone());
            env.insert(name.clone(), eval(value, &env).unwrap_or(Value::Null));
        }
    }

    let mut funcs_js = String::new();
    for f in &program.functions {
        if let Statement::Function { name, params, body, .. } = f {
            let mut cg = Codegen::new();
            let mut inner = String::new();
            for s in body { inner.push_str(&cg.stmt_to_js(s, "  ").unwrap_or_default()); }
            funcs_js.push_str(&format!("function {}({}) {{\n{}}}\n", name, params.join(", "), inner));
        }
    }

    let mut cg = Codegen::new();
    let mut body_html = String::new();
    for s in &program.body {
        body_html.push_str(&cg.gen_html(s, &mut env, &state_vars).unwrap_or_default());
    }

    let mut css = String::new();
    for r in &program.styles {
        css.push_str(&format!("{} {{\n", r.selector));
        for (p, v) in &r.properties {
            css.push_str(&format!("  {}: {};\n", css_property(p), css_value(v)));
        }
        css.push_str("}\n");
    }

    let mut state_init = String::new();
    for s in &program.state {
        if let Statement::Let { name, value, .. } = s {
            let v = eval(value, &env).unwrap_or(Value::Null);
            state_init.push_str(&format!("  {}: {},\n", name, v.to_js_literal()));
        }
    }

    let title = match &program.page_title {
        Some(e) => eval(e, &env).map(|v| v.to_display()).unwrap_or_else(|_| "Rino Page".into()),
        None => "Rino Page".into(),
    };

    let helpers = r#"
function عدد(v) { const n = parseFloat(v); return isNaN(n) ? 0 : n; }
function نص(v) { return String(v); }
function اقرأ(id) {
  const el = document.getElementById(id);
  return el ? el.value : "";
}
function امسح(id) {
  const el = document.getElementById(id);
  if (el) el.innerHTML = "";
}
function أضف_مهمة(id_قائمة, نص) {
  const ul = document.getElementById(id_قائمة);
  if (!ul) return;
  const li = document.createElement("li");
  li.textContent = نص;
  li.style.padding = "10px";
  li.style.marginTop = "5px";
  li.style.background = "rgb(240, 240, 240)";
  li.style.borderRadius = "5px";
  li.style.cursor = "pointer";
  li.title = "انقر للحذف";
  li.onclick = function() { li.remove(); };
  ul.appendChild(li);
}
"#;

    let script = if !state_vars.is_empty() {
        format!(r#"{helpers}
const حالة = new Proxy({{{}}}, {{
  set(target, key, value) {{
    target[key] = value;
    updateAll();
    return true;
  }}
}});

{}

function updateAll() {{
{}}}

updateAll();
"#,
            state_init,
            events = cg.events_js,
            updates = cg.updates_js,
            helpers = helpers
        )
    } else {
        format!("{}{}", helpers, cg.events_js)
    };

    let html = format!(r#"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
  <meta charset="UTF-8">
  <title>{title}</title>
  <link rel="stylesheet" href="{css_file}">
</head>
<body>
{body}  <script src="{js_file}"></script>
</body>
</html>
"#,
        title = title,
        css_file = css_filename,
        js_file = js_filename,
        body = body_html
    );

    let css_content = css;
    let js_content = format!("{}{}", funcs_js, script);

    if let Err(e) = fs::write(&output_html_path, &html) {
        eprintln!("❌ فشل كتابة {}: {}", output_html_path.display(), e);
        std::process::exit(1);
    }
    if let Err(e) = fs::write(&output_css_path, &css_content) {
        eprintln!("❌ فشل كتابة {}: {}", output_css_path.display(), e);
        std::process::exit(1);
    }
    if let Err(e) = fs::write(&output_js_path, &js_content) {
        eprintln!("❌ فشل كتابة {}: {}", output_js_path.display(), e);
        std::process::exit(1);
    }

    println!("✅ تم التوليد:");
    println!("   📄 {}", output_html_path.display());
    println!("   🎨 {}", output_css_path.display());
    println!("   ⚙️  {}", output_js_path.display());
}