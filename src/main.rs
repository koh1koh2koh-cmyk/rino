//! Rino — لغة عربية لبناء الويب.
//! المرحلة 2: HTML + CSS + أحداث

mod ast;
mod correction;
mod lexer;
mod parser;
mod token;

use ast::{BinOp, Expression, Program, Statement};
use lexer::Lexer;
use parser::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

impl Value {
    fn to_display(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Num(n) => {
                if n.fract() == 0.0 { format!("{}", *n as i64) }
                else { format!("{}", n) }
            }
            Value::Bool(b) => if *b { "صحيح" } else { "خطأ" }.to_string(),
            Value::Null => String::new(),
        }
    }
    fn as_number(&self) -> Result<f64, String> {
        match self {
            Value::Num(n) => Ok(*n),
            Value::Str(s) => s.parse::<f64>()
                .map_err(|_| format!("لا يمكن تحويل \"{}\" إلى عدد", s)),
            _ => Err("قيمة غير رقمية".to_string()),
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
            .ok_or_else(|| format!("المتغير \"{}\" غير معرف", name)),
        Expression::Binary { left, op, right } => {
            let l = eval(left, env)?;
            let r = eval(right, env)?;
            match op {
                BinOp::Add => match (&l, &r) {
                    (Value::Num(a), Value::Num(b)) => Ok(Value::Num(a + b)),
                    _ => Ok(Value::Str(l.to_display() + &r.to_display())),
                },
                BinOp::Sub => Ok(Value::Num(l.as_number()? - r.as_number()?)),
                BinOp::Mul => Ok(Value::Num(l.as_number()? * r.as_number()?)),
                BinOp::Div => {
                    let b = r.as_number()?;
                    if b == 0.0 { Err("القسمة على صفر".to_string()) }
                    else { Ok(Value::Num(l.as_number()? / b)) }
                }
            }
        }
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
        _ => name,
    }
}

fn css_value(val: &str) -> String {
    match val {
        "أحمر" => "red".to_string(),
        "أزرق" => "blue".to_string(),
        "أخضر" => "green".to_string(),
        "أبيض" => "white".to_string(),
        "أسود" => "black".to_string(),
        "رمادي" => "gray".to_string(),
        "أصفر" => "yellow".to_string(),
        "وسط" => "center".to_string(),
        "يمين" => "right".to_string(),
        "يسار" => "left".to_string(),
        _ => {
            if val.parse::<f64>().is_ok() {
                format!("{}px", val)
            } else {
                val.to_string()
            }
        }
    }
}

fn generate_html(program: &Program, env: &HashMap<String, Value>) -> Result<String, String> {
    let title = match &program.page_title {
        Some(e) => eval(e, env)?.to_display(),
        None => "Rino Page".to_string(),
    };

    // CSS
    let mut css = String::new();
    for rule in &program.styles {
        css.push_str(&format!("{} {{\n", rule.selector));
        for (prop, val) in &rule.properties {
            css.push_str(&format!("  {}: {};\n", css_property(prop), css_value(val)));
        }
        css.push_str("}\n");
    }

    // HTML + JS
    let mut body = String::new();
    let mut js = String::new();
    let mut id_counter = 1usize;

    for stmt in &program.statements {
        match stmt {
            Statement::HtmlElement { tag, content, events, .. } => {
                let v = eval(content, env)?;
                let needs_id = !events.is_empty();
                let id = format!("rino-{}", id_counter);
                id_counter += 1;

                if needs_id {
                    body.push_str(&format!(
                        "  <{} id=\"{}\">{}</{}>\n",
                        tag, id, v.to_display(), tag
                    ));
                } else {
                    body.push_str(&format!(
                        "  <{}>{}</{}>\n",
                        tag, v.to_display(), tag
                    ));
                }

                for event in events {
                    let mut event_body = String::new();
                    for s in &event.body {
                        if let Statement::Call { name, args, .. } = s {
                            if name == "اطبع" {
                                if let Some(arg) = args.first() {
                                    let av = eval(arg, env)?;
                                    event_body.push_str(&format!(
                                        "console.log(\"{}\");",
                                        av.to_display()
                                    ));
                                }
                            }
                        }
                    }
                    js.push_str(&format!(
                        "document.getElementById('{}').addEventListener('{}', function() {{ {} }});\n",
                        id, event.kind, event_body
                    ));
                }
            }
            Statement::Call { name, args, .. } if name == "اطبع" => {
                if let Some(arg) = args.first() {
                    let v = eval(arg, env)?;
                    js.push_str(&format!("console.log(\"{}\");\n", v.to_display()));
                }
            }
            _ => {}
        }
    }

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
  <meta charset="UTF-8">
  <title>{title}</title>
  <style>
{css}  </style>
</head>
<body>
{body}  <script>
{js}  </script>
</body>
</html>
"#,
        title = title, css = css, body = body, js = js
    ))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input_path = if args.len() > 1 { args[1].clone() } else { "index.rino".to_string() };

    println!("📂 قراءة الملف: {}", input_path);
    let source = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("❌ فشل قراءة الملف: {}", e); std::process::exit(1); }
    };
    println!("{}", "─".repeat(50));

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { eprintln!("❌ خطأ لغوي: {}", e); std::process::exit(1); }
    };

    if !lexer.corrections.is_empty() {
        println!("📋 تقرير التصحيح:");
        for c in &lexer.corrections { println!("  {}", c); }
        println!();
    }

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => { eprintln!("❌ خطأ نحوي: {}", e); std::process::exit(1); }
    };

    let mut env: HashMap<String, Value> = HashMap::new();
    for stmt in &program.statements {
        if let Statement::Let { name, value, .. } = stmt {
            match eval(value, &env) {
                Ok(v) => { env.insert(name.clone(), v); }
                Err(e) => { eprintln!("❌ خطأ في المتغير {}: {}", name, e); std::process::exit(1); }
            }
        }
    }

    let html = match generate_html(&program, &env) {
        Ok(h) => h,
        Err(e) => { eprintln!("❌ خطأ في توليد HTML: {}", e); std::process::exit(1); }
    };

    let output_path = "index.html";
    if let Err(e) = fs::write(output_path, &html) {
        eprintln!("❌ فشل كتابة {}: {}", output_path, e);
        std::process::exit(1);
    }

    println!("✅ تم توليد الموقع بنجاح!");
    println!("📄 الملف: {}", Path::new(output_path).canonicalize().unwrap().display());
    println!("🌐 افتحه في المتصفح لمشاهدته.");
}