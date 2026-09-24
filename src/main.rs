//! Rino — لغة عربية لبناء الويب.
//! الإصدار 1.5 — مكتبة مكونات جاهزة

mod ast;
mod correction;
mod lexer;
mod parser;
mod token;

mod formatter {
    pub struct Formatter { indent: usize, indent_size: usize }
    impl Formatter {
        pub fn new() -> Self { Self { indent: 0, indent_size: 2 } }
        pub fn format(&mut self, source: &str) -> String {
            let mut output = String::new();
            let mut in_string = false;
            let mut current_line = String::new();
            let mut last_char: Option<char> = None;
            for ch in source.chars() {
                if ch == '"' && last_char != Some('\\') { in_string = !in_string; }
                if in_string { current_line.push(ch); last_char = Some(ch); continue; }
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

/// ============ CSS للمكونات الجاهزة ============
const BUILTIN_CSS: &str = r#"
/* ===== مكتبة Rino الجاهزة ===== */
.rino-btn {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #ffffff;
  padding: 12px 28px;
  border: none;
  border-radius: 10px;
  font-size: 16px;
  font-weight: bold;
  cursor: pointer;
  transition: all 0.3s ease;
  margin: 6px;
  font-family: inherit;
}
.rino-btn:hover {
  transform: translateY(-3px);
  box-shadow: 0 10px 20px rgba(102, 126, 234, 0.4);
}
.rino-card {
  background: #ffffff;
  border-radius: 14px;
  padding: 24px;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.08);
  margin: 16px auto;
  max-width: 420px;
  text-align: center;
  border: 1px solid #eef0f5;
  transition: all 0.3s ease;
}
.rino-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.12);
}
.rino-card h3 {
  color: #2d3748;
  margin: 0 0 12px 0;
  font-size: 22px;
}
.rino-card p {
  color: #718096;
  margin: 0 0 16px 0;
  font-size: 16px;
  line-height: 1.6;
}
.rino-card .rino-price {
  color: #667eea;
  font-size: 24px;
  font-weight: bold;
}
.rino-alert {
  padding: 14px 20px;
  border-radius: 10px;
  margin: 12px auto;
  max-width: 520px;
  text-align: center;
  font-size: 16px;
  font-weight: 500;
  border-right: 5px solid;
}
.rino-success { background: #d4edda; color: #155724; border-color: #28a745; }
.rino-error   { background: #f8d7da; color: #721c24; border-color: #dc3545; }
.rino-warning { background: #fff3cd; color: #856404; border-color: #ffc107; }
.rino-info    { background: #d1ecf1; color: #0c5460; border-color: #17a2b8; }
.rino-progress {
  width: 90%;
  max-width: 500px;
  height: 24px;
  background: #e9ecef;
  border-radius: 12px;
  margin: 12px auto;
  overflow: hidden;
}
.rino-progress-bar {
  height: 100%;
  background: linear-gradient(90deg, #667eea, #764ba2);
  border-radius: 12px;
  transition: width 0.4s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 13px;
  font-weight: bold;
}
.rino-highlight {
  background: #fff8dc;
  border-right: 4px solid #ffa500;
  padding: 14px 20px;
  margin: 12px auto;
  max-width: 600px;
  border-radius: 8px;
  color: #5a4a00;
  font-size: 17px;
  text-align: center;
}
.rino-header {
  background: linear-gradient(135deg, #667eea, #764ba2);
  color: white;
  padding: 22px;
  border-radius: 12px;
  text-align: center;
  font-size: 26px;
  font-weight: bold;
  margin: 16px auto;
  max-width: 700px;
  box-shadow: 0 6px 16px rgba(102, 126, 234, 0.3);
}
.rino-divider {
  height: 2px;
  background: linear-gradient(90deg, transparent, #667eea, transparent);
  margin: 24px auto;
  max-width: 400px;
  border: none;
}
"#;

fn process_imports(source: &str, base_dir: &Path, visited: &mut HashSet<PathBuf>, depth: usize) -> Result<String, String> {
    if depth > 20 { return Err("عدد الاستيرادات المتتالية كبير جدًا".into()); }
    let mut output = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("استيراد ") {
            let rest = trimmed["استيراد".len()..].trim();
            if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
                let rel_path = &rest[1..rest.len() - 1];
                let file_path = base_dir.join(rel_path);
                let canonical = file_path.canonicalize()
                    .map_err(|_| format!("ملف غير موجود: {}", file_path.display()))?;
                if visited.contains(&canonical) { output.push('\n'); continue; }
                visited.insert(canonical.clone());
                let content = fs::read_to_string(&file_path)
                    .map_err(|e| format!("فشل قراءة {}: {}", file_path.display(), e))?;
                let new_base = file_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| base_dir.to_path_buf());
                let processed = process_imports(&content, &new_base, visited, depth + 1)?;
                output.push_str(&processed);
                output.push('\n');
            } else {
                return Err(format!("صيغة استيراد خاطئة: {}", trimmed));
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    Ok(output)
}

#[derive(Debug, Clone)]
enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    List(Vec<Value>),
    Dict(Vec<(String, Value)>),
}

impl Value {
    fn to_display(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Num(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
            Value::Bool(b) => if *b { "صحيح" } else { "خطأ" }.to_string(),
            Value::Null => String::new(),
            Value::List(items) => items.iter().map(|v| v.to_display()).collect::<Vec<_>>().join("، "),
            Value::Dict(pairs) => {
                let p: Vec<String> = pairs.iter().map(|(k, v)| format!("{}: {}", k, v.to_display())).collect();
                format!("{{{}}}", p.join(", "))
            }
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
            Value::Bool(b) => *b, Value::Num(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(), Value::Null => false,
            Value::List(v) => !v.is_empty(),
            Value::Dict(p) => !p.is_empty(),
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
            Value::Dict(pairs) => {
                let p: Vec<String> = pairs.iter().map(|(k, v)| format!("\"{}\": {}", k, v.to_js_literal())).collect();
                format!("{{{}}}", p.join(", "))
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
        Expression::Dict(pairs) => {
            let mut map = Vec::new();
            for (k, v) in pairs { map.push((k.clone(), eval(v, env)?)); }
            Ok(Value::Dict(map))
        }
        Expression::MemberAccess { object, property } => {
            let obj = eval(object, env)?;
            match obj {
                Value::Dict(pairs) => {
                    pairs.iter()
                        .find(|(k, _)| k == property)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| format!("الخاصية \"{}\" غير موجودة", property))
                }
                _ => Err("لا يمكن الوصول لخاصية من نوع غير قاموس".into()),
            }
        }
        Expression::Call { name, args } => {
            if name == "اقرأ_محلي" || name == "اقرأ_مدخل" || name == "اجلب" || name == "اجلب_نص" {
                return Ok(Value::Null);
            }

            let mut vs = Vec::new();
            for a in args { vs.push(eval(a, env)?); }

            match name.as_str() {
                "طول" => match vs.first() {
                    Some(Value::List(l)) => Ok(Value::Num(l.len() as f64)),
                    Some(Value::Str(s)) => Ok(Value::Num(s.chars().count() as f64)),
                    Some(Value::Dict(p)) => Ok(Value::Num(p.len() as f64)),
                    _ => Err("طول تحتاج قائمة أو نصًا أو قاموسًا".into()),
                },
                "كبير" => match vs.first() { Some(Value::Str(s)) => Ok(Value::Str(s.to_uppercase())), _ => Err("كبير تحتاج نصًا".into()) },
                "صغير" => match vs.first() { Some(Value::Str(s)) => Ok(Value::Str(s.to_lowercase())), _ => Err("صغير تحتاج نصًا".into()) },
                "يحتوي" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::Str(s)), Some(Value::Str(sub))) => Ok(Value::Bool(s.contains(sub.as_str()))),
                    _ => Err("يحتوي تحتاج نصين".into()),
                },
                "استبدل" => match (vs.get(0), vs.get(1), vs.get(2)) {
                    (Some(Value::Str(s)), Some(Value::Str(a)), Some(Value::Str(b))) => Ok(Value::Str(s.replace(a.as_str(), b.as_str()))),
                    _ => Err("استبدل تحتاج 3 نصوص".into()),
                },
                "جذر" => match vs.first() { Some(v) => Ok(Value::Num(v.as_num()?.sqrt())), _ => Err("جذر تحتاج عددًا".into()) },
                "قوة" => match (vs.get(0), vs.get(1)) { (Some(a), Some(b)) => Ok(Value::Num(a.as_num()?.powf(b.as_num()?))), _ => Err("قوة تحتاج عددين".into()) },
                "قوس" => match vs.first() { Some(v) => Ok(Value::Num(v.as_num()?.round())), _ => Err("قوس تحتاج عددًا".into()) },
                "أرضي" => match vs.first() { Some(v) => Ok(Value::Num(v.as_num()?.floor())), _ => Err("أرضي تحتاج عددًا".into()) },
                "سقف" => match vs.first() { Some(v) => Ok(Value::Num(v.as_num()?.ceil())), _ => Err("سقف تحتاج عددًا".into()) },
                "مطلق" => match vs.first() { Some(v) => Ok(Value::Num(v.as_num()?.abs())), _ => Err("مطلق تحتاج عددًا".into()) },
                "أصغر" => match (vs.get(0), vs.get(1)) {
                    (Some(a), Some(b)) => { let (x, y) = (a.as_num()?, b.as_num()?); Ok(Value::Num(if x < y { x } else { y })) }
                    _ => Err("أصغر تحتاج عددين".into()),
                },
                "أكبر" => match (vs.get(0), vs.get(1)) {
                    (Some(a), Some(b)) => { let (x, y) = (a.as_num()?, b.as_num()?); Ok(Value::Num(if x > y { x } else { y })) }
                    _ => Err("أكبر تحتاج عددين".into()),
                },
                "عدد" => match vs.first() { Some(v) => Ok(Value::Num(v.as_num()?)), _ => Err("عدد تحتاج قيمة".into()) },

                "مفاتيح" => match vs.first() {
                    Some(Value::Dict(p)) => {
                        let keys: Vec<Value> = p.iter().map(|(k, _)| Value::Str(k.clone())).collect();
                        Ok(Value::List(keys))
                    }
                    _ => Err("مفاتيح تحتاج قاموسًا".into()),
                },
                "قيم" => match vs.first() {
                    Some(Value::Dict(p)) => {
                        let values: Vec<Value> = p.iter().map(|(_, v)| v.clone()).collect();
                        Ok(Value::List(values))
                    }
                    _ => Err("قيم تحتاج قاموسًا".into()),
                },
                "يحتوي_مفتاح" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::Dict(p)), Some(Value::Str(k))) => {
                        Ok(Value::Bool(p.iter().any(|(key, _)| key == k)))
                    }
                    _ => Err("يحتوي_مفتاح تحتاج قاموسًا ونصًا".into()),
                },

                "أضف" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::List(l)), Some(v)) => {
                        let mut new_list = l.clone();
                        new_list.push(v.clone());
                        Ok(Value::List(new_list))
                    }
                    _ => Err("أضف تحتاج قائمة وقيمة".into()),
                },
                "احذف" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::List(l)), Some(v)) => {
                        let mut new_list = l.clone();
                        if let Some(pos) = new_list.iter().position(|x| x.to_display() == v.to_display()) {
                            new_list.remove(pos);
                        }
                        Ok(Value::List(new_list))
                    }
                    _ => Err("احذف تحتاج قائمة وقيمة".into()),
                },
                "اعكس" => match vs.first() {
                    Some(Value::List(l)) => {
                        let mut new_list = l.clone();
                        new_list.reverse();
                        Ok(Value::List(new_list))
                    }
                    _ => Err("اعكس تحتاج قائمة".into()),
                },
                "دمج" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::List(l)), Some(Value::Str(sep))) => {
                        let parts: Vec<String> = l.iter().map(|v| v.to_display()).collect();
                        Ok(Value::Str(parts.join(sep)))
                    }
                    (Some(Value::List(l)), None) => {
                        let parts: Vec<String> = l.iter().map(|v| v.to_display()).collect();
                        Ok(Value::Str(parts.join("")))
                    }
                    _ => Err("دمج تحتاج قائمة وفاصلًا".into()),
                },
                "أول" => match vs.first() {
                    Some(Value::List(l)) => l.first().cloned().ok_or_else(|| "القائمة فارغة".into()),
                    _ => Err("أول تحتاج قائمة".into()),
                },
                "آخر" => match vs.first() {
                    Some(Value::List(l)) => l.last().cloned().ok_or_else(|| "القائمة فارغة".into()),
                    _ => Err("آخر تحتاج قائمة".into()),
                },
                "مفهرس" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::List(l)), Some(i)) => {
                        let idx = i.as_num()? as usize;
                        l.get(idx).cloned().ok_or_else(|| format!("الفهرس {} خارج الحدود", idx))
                    }
                    _ => Err("مفهرس تحتاج قائمة ورقمًا".into()),
                },
                "يحتوي_قائمة" => match (vs.get(0), vs.get(1)) {
                    (Some(Value::List(l)), Some(v)) => {
                        Ok(Value::Bool(l.iter().any(|x| x.to_display() == v.to_display())))
                    }
                    _ => Err("يحتوي_قائمة تحتاج قائمة وقيمة".into()),
                },
                "رتب" => match vs.first() {
                    Some(Value::List(l)) => {
                        let mut new_list = l.clone();
                        new_list.sort_by(|a, b| {
                            match (a, b) {
                                (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                                _ => a.to_display().cmp(&b.to_display()),
                            }
                        });
                        Ok(Value::List(new_list))
                    }
                    _ => Err("رتب تحتاج قائمة".into()),
                },
                "مدى" => match (vs.get(0), vs.get(1)) {
                    (Some(a), Some(b)) => {
                        let start = a.as_num()? as i64;
                        let end = b.as_num()? as i64;
                        let mut items = Vec::new();
                        if start <= end {
                            for i in start..=end { items.push(Value::Num(i as f64)); }
                        } else {
                            for i in (end..=start).rev() { items.push(Value::Num(i as f64)); }
                        }
                        Ok(Value::List(items))
                    }
                    _ => Err("مدى تحتاج رقمين".into()),
                },

                "احفظ" => Ok(Value::Null),
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
                    if b == 0.0 { Err("القسمة على صفر".into()) } else { Ok(Value::Num(l.as_num()? / b)) }
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
                LogOp::And => if !l.as_bool() { Ok(Value::Bool(false)) } else { Ok(Value::Bool(eval(right, env)?.as_bool())) },
                LogOp::Or => if l.as_bool() { Ok(Value::Bool(true)) } else { Ok(Value::Bool(eval(right, env)?.as_bool())) },
            }
        }
        Expression::Not(e) => Ok(Value::Bool(!eval(e, env)?.as_bool())),
    }
}

fn eval_with_funcs(
    expr: &Expression,
    env: &HashMap<String, Value>,
    funcs: &HashMap<String, (Vec<String>, Vec<Statement>)>,
    depth: usize,
) -> Result<Value, String> {
    if depth > 50 {
        return Err("عمق التقييم كبير جدًا".into());
    }

    match expr {
        Expression::Call { name, args } => {
            if let Some((params, body)) = funcs.get(name) {
                let mut new_env: HashMap<String, Value> = HashMap::new();
                for (i, p) in params.iter().enumerate() {
                    if let Some(arg) = args.get(i) {
                        let v = eval_with_funcs(arg, env, funcs, depth + 1)?;
                        new_env.insert(p.clone(), v);
                    }
                }
                for stmt in body {
                    if let Statement::Return { value: Some(v), .. } = stmt {
                        return eval_with_funcs(v, &new_env, funcs, depth + 1);
                    }
                    if let Statement::Let { name, value, .. } = stmt {
                        let v = eval_with_funcs(value, &new_env, funcs, depth + 1)?;
                        new_env.insert(name.clone(), v);
                    }
                }
                return Ok(Value::Null);
            }
            eval(expr, env)
        }
        Expression::Binary { left, op, right } => {
            let l = eval_with_funcs(left, env, funcs, depth + 1)?;
            let r = eval_with_funcs(right, env, funcs, depth + 1)?;
            match op {
                BinOp::Add => match (&l, &r) {
                    (Value::Num(a), Value::Num(b)) => Ok(Value::Num(a + b)),
                    _ => Ok(Value::Str(l.to_display() + &r.to_display())),
                },
                BinOp::Sub => Ok(Value::Num(l.as_num()? - r.as_num()?)),
                BinOp::Mul => Ok(Value::Num(l.as_num()? * r.as_num()?)),
                BinOp::Div => {
                    let b = r.as_num()?;
                    if b == 0.0 { Err("القسمة على صفر".into()) } else { Ok(Value::Num(l.as_num()? / b)) }
                }
                BinOp::Mod => Ok(Value::Num(l.as_num()? % r.as_num()?)),
            }
        }
        Expression::Comparison { left, op, right } => {
            let l = eval_with_funcs(left, env, funcs, depth + 1)?;
            let r = eval_with_funcs(right, env, funcs, depth + 1)?;
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
            let l = eval_with_funcs(left, env, funcs, depth + 1)?;
            match op {
                LogOp::And => if !l.as_bool() { Ok(Value::Bool(false)) }
                    else { Ok(Value::Bool(eval_with_funcs(right, env, funcs, depth + 1)?.as_bool())) },
                LogOp::Or => if l.as_bool() { Ok(Value::Bool(true)) }
                    else { Ok(Value::Bool(eval_with_funcs(right, env, funcs, depth + 1)?.as_bool())) },
            }
        }
        Expression::Not(e) => {
            Ok(Value::Bool(!eval_with_funcs(e, env, funcs, depth + 1)?.as_bool()))
        }
        Expression::List(items) => {
            let mut vs = Vec::new();
            for i in items { vs.push(eval_with_funcs(i, env, funcs, depth + 1)?); }
            Ok(Value::List(vs))
        }
        _ => eval(expr, env),
    }
}

fn run_tests(program: &Program) -> i32 {
    if program.tests.is_empty() {
        println!("⚠️  لا توجد اختبارات في هذا الملف");
        return 0;
    }

    let mut funcs: HashMap<String, (Vec<String>, Vec<Statement>)> = HashMap::new();
    for f in &program.functions {
        if let Statement::Function { name, params, body, .. } = f {
            funcs.insert(name.clone(), (params.clone(), body.clone()));
        }
    }

    println!();
    println!("🧪 تشغيل {} اختبار...", program.tests.len());
    println!("─────────────────────────────────────");

    let mut passed = 0;
    let mut failed = 0;

    for test in &program.tests {
        if let Statement::Test { name, body, .. } = test {
            let mut env: HashMap<String, Value> = HashMap::new();
            let mut errors: Vec<String> = Vec::new();
            let mut assertion_num = 0;

            for stmt in body {
                match stmt {
                    Statement::Call { name: cname, args, .. } if cname == "توقع" => {
                        assertion_num += 1;
                        if let Some(arg) = args.first() {
                            match eval_with_funcs(arg, &env, &funcs, 0) {
                                Ok(v) => {
                                    if !v.as_bool() {
                                        errors.push(format!("توقع #{} فشل: القيمة = {}", assertion_num, v.to_display()));
                                    }
                                }
                                Err(e) => errors.push(format!("توقع #{}: خطأ — {}", assertion_num, e)),
                            }
                        }
                    }
                    Statement::Call { name: cname, args, .. } if cname == "توقع_يساوي" => {
                        assertion_num += 1;
                        if args.len() >= 2 {
                            let a = eval_with_funcs(&args[0], &env, &funcs, 0);
                            let b = eval_with_funcs(&args[1], &env, &funcs, 0);
                            match (a, b) {
                                (Ok(va), Ok(vb)) => {
                                    let da = va.to_display();
                                    let db = vb.to_display();
                                    if da != db {
                                        errors.push(format!("توقع #{}: متوقع [{}] لكن وجد [{}]", assertion_num, db, da));
                                    }
                                }
                                (Err(e), _) | (_, Err(e)) => {
                                    errors.push(format!("توقع #{}: خطأ — {}", assertion_num, e));
                                }
                            }
                        }
                    }
                    Statement::Let { name, value, .. } => {
                        if let Ok(v) = eval_with_funcs(value, &env, &funcs, 0) {
                            env.insert(name.clone(), v);
                        }
                    }
                    _ => {}
                }
            }

            if errors.is_empty() {
                println!("✅ {}", name);
                passed += 1;
            } else {
                println!("❌ {}", name);
                for e in &errors {
                    println!("   • {}", e);
                }
                failed += 1;
            }
        }
    }

    println!("─────────────────────────────────────");
    println!("📊 النتيجة: {} نجح | {} فشل | {} الإجمالي", passed, failed, passed + failed);

    if failed > 0 { 1 } else { 0 }
}

fn css_property(name: &str) -> &str {
    match name {
        "لون" => "color", "خلفية" => "background", "حجم" => "font-size",
        "حشوة" => "padding", "هامش" => "margin", "استدارة" => "border-radius",
        "محاذاة" => "text-align", "عرض" => "width", "ارتفاع" => "height",
        "حد" => "border", "وزن" => "font-weight", "ظل" => "box-shadow",
        "اتجاه" => "direction", "تحويل" => "text-transform",
        "مسافة_بين_الأسطر" => "line-height",
        "انتقال" => "transition",
        "أقصى_عرض" => "max-width", "أدنى_عرض" => "min-width",
        "أقصى_ارتفاع" => "max-height", "أدنى_ارتفاع" => "min-height",
        "موضع" => "position", "أعلى" => "top", "أسفل" => "bottom",
        "يمين_الموضع" => "right", "يسار_الموضع" => "left",
        "ظل_النص" => "text-shadow", "شفافية" => "opacity",
        _ => name,
    }
}

fn css_value(val: &str) -> String {
    match val {
        "أحمر" => "red".into(), "أزرق" => "blue".into(), "أخضر" => "green".into(),
        "أبيض" => "white".into(), "أسود" => "black".into(), "رمادي" => "gray".into(),
        "أصفر" => "yellow".into(), "برتقالي" => "orange".into(), "بنفسجي" => "purple".into(),
        "وردي" => "pink".into(), "بني" => "brown".into(), "ذهبي" => "gold".into(),
        "فضي" => "silver".into(), "سماوي" => "skyblue".into(), "ليموني" => "lime".into(),
        "أرجواني" => "magenta".into(), "نيلي" => "navy".into(), "كريمي" => "beige".into(),
        "شفاف" => "transparent".into(), "تركوازي" => "turquoise".into(), "مرجاني" => "coral".into(),
        "وسط" => "center".into(), "يمين" => "right".into(), "يسار" => "left".into(),
        "عريض" => "bold".into(), "ضعيف" => "lighter".into(), "مائل" => "italic".into(),
        "سريع" => "0.2s".into(), "بطيء" => "0.8s".into(),
        _ => if val.parse::<f64>().is_ok() { format!("{}px", val) } else { val.to_string() },
    }
}

struct Codegen {
    counter: usize,
    events_js: String,
    updates_js: String,
    components: HashMap<String, (Vec<String>, Vec<Statement>)>,
    depth: usize,
}

impl Codegen {
    fn new() -> Self {
        Self { counter: 0, events_js: String::new(), updates_js: String::new(), components: HashMap::new(), depth: 0 }
    }
    fn next_id(&mut self) -> String { self.counter += 1; format!("r{}", self.counter) }

    /// ============ مكتبة المكونات الجاهزة ============
    fn gen_builtin(&mut self, name: &str, args: &[Expression], env: &HashMap<String, Value>) -> Option<String> {
        let get_str = |i: usize| -> String {
            args.get(i).and_then(|a| eval(a, env).ok()).map(|v| v.to_display()).unwrap_or_default()
        };
        let get_num = |i: usize| -> f64 {
            args.get(i).and_then(|a| eval(a, env).ok()).and_then(|v| v.as_num().ok()).unwrap_or(0.0)
        };

        match name {
            "زر_جميل" => {
                Some(format!("<button class=\"rino-btn\">{}</button>", get_str(0)))
            }
            "بطاقة" => {
                let title = get_str(0);
                let desc = get_str(1);
                let price = if args.len() > 2 { format!("<div class=\"rino-price\">{}</div>", get_str(2)) } else { String::new() };
                Some(format!(
                    "<div class=\"rino-card\"><h3>{}</h3><p>{}</p>{}</div>",
                    title, desc, price
                ))
            }
            "تنبيه_نجاح" => Some(format!("<div class=\"rino-alert rino-success\">{}</div>", get_str(0))),
            "تنبيه_خطأ" => Some(format!("<div class=\"rino-alert rino-error\">{}</div>", get_str(0))),
            "تنبيه_تحذير" => Some(format!("<div class=\"rino-alert rino-warning\">{}</div>", get_str(0))),
            "تنبيه_معلومة" => Some(format!("<div class=\"rino-alert rino-info\">{}</div>", get_str(0))),
            "شريط_تقدم" => {
                let pct = get_num(0).clamp(0.0, 100.0);
                Some(format!(
                    "<div class=\"rino-progress\"><div class=\"rino-progress-bar\" style=\"width: {}%\">{:.0}%</div></div>",
                    pct, pct
                ))
            }
            "فقرة_مهمة" => Some(format!("<div class=\"rino-highlight\">{}</div>", get_str(0))),
            "رأس_جميل" => Some(format!("<div class=\"rino-header\">{}</div>", get_str(0))),
            "فاصل_جميل" => Some("<hr class=\"rino-divider\">".to_string()),
            _ => None,
        }
    }

    fn is_reactive(&self, expr: &Expression, state_vars: &[String]) -> bool {
        match expr {
            Expression::Identifier(n) => state_vars.contains(n),
            Expression::Binary { left, right, .. } |
            Expression::Comparison { left, right, .. } |
            Expression::Logical { left, right, .. } => {
                self.is_reactive(left, state_vars) || self.is_reactive(right, state_vars)
            }
            Expression::Not(e) => self.is_reactive(e, state_vars),
            Expression::List(items) => items.iter().any(|i| self.is_reactive(i, state_vars)),
            Expression::Dict(pairs) => pairs.iter().any(|(_, v)| self.is_reactive(v, state_vars)),
            Expression::MemberAccess { object, .. } => self.is_reactive(object, state_vars),
            Expression::Call { name, args } => {
                if name == "اقرأ_محلي" { return true; }
                args.iter().any(|a| self.is_reactive(a, state_vars))
            }
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
            Expression::Dict(pairs) => {
                let p: Vec<String> = pairs.iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                    .collect();
                format!("{{{}}}", p.join(", "))
            }
            Expression::MemberAccess { object, property } => {
                format!("{}.{}", self.expr_to_js(object), property)
            }
            Expression::Call { name, args } => {
                if name == "اقرأ_مدخل" && args.is_empty() { return "this.value".to_string(); }
                let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x)).collect();
                format!("{}({})", name, a.join(", "))
            }
            Expression::Binary { left, op, right } => {
                let op_str = match op { BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%" };
                format!("({} {} {})", self.expr_to_js(left), op_str, self.expr_to_js(right))
            }
            Expression::Comparison { left, op, right } => {
                let op_str = match op {
                    CmpOp::Eq => "===", CmpOp::Ne => "!==",
                    CmpOp::Gt => ">", CmpOp::Lt => "<", CmpOp::Ge => ">=", CmpOp::Le => "<=",
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
            Statement::TryCatch { try_body, catch_var, catch_body, .. } => {
                let mut s = format!("{}try {{\n", indent);
                for st in try_body { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                s.push_str(&format!("{}}} catch ({}) {{\n", indent, catch_var));
                for st in catch_body { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            Statement::Call { name, args, .. } => {
                if name == "_skip_" { return Ok(String::new()); }
                if name == "اجلب" || name == "اجلب_نص" {
                    return Ok(self.gen_fetch_call(name, args, indent));
                }
                let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x)).collect();
                if name == "اطبع" { Ok(format!("{}console.log({});\n", indent, a.join(", "))) }
                else { Ok(format!("{}{}({});\n", indent, name, a.join(", "))) }
            }
            Statement::ForEach { var, iterable, body, .. } => {
                let mut s = format!("{}for (let {} of {}) {{\n", indent, var, self.expr_to_js(iterable));
                for st in body { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            Statement::RangeFor { var, start, end, step, body, .. } => {
                let start_js = self.expr_to_js(start);
                let end_js = self.expr_to_js(end);
                let step_js = step.as_ref().map(|e| self.expr_to_js(e)).unwrap_or_else(|| "1".to_string());
                let mut s = format!("{}for (let {} = {}; {} <= {}; {} += {}) {{\n", indent, var, start_js, var, end_js, var, step_js);
                for st in body { s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent))?); }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            _ => Ok(String::new()),
        }
    }

    fn gen_fetch_call(&self, name: &str, args: &[Expression], indent: &str) -> String {
        if args.len() < 2 { return String::new(); }
        let url_js = self.expr_to_js(&args[0]);
        let var_name = match &args[1] {
            Expression::String(s) => s.clone(),
            _ => return String::new(),
        };
        let field_name = if args.len() >= 3 {
            match &args[2] {
                Expression::String(s) => Some(s.clone()),
                _ => None,
            }
        } else { None };

        if name == "اجلب" {
            if let Some(field) = field_name {
                format!(
                    "{}fetch({url})\n{indent}  .then(function(r) {{ return r.json(); }})\n{indent}  .then(function(d) {{ حالة.{var} = (typeof d.{field} === 'string') ? d.{field} : JSON.stringify(d.{field}); }})\n{indent}  .catch(function(e) {{ حالة.{var} = 'خطأ: ' + e.message; }});\n",
                    indent, indent = indent, url = url_js, var = var_name, field = field
                )
            } else {
                format!(
                    "{}fetch({url})\n{indent}  .then(function(r) {{ return r.json(); }})\n{indent}  .then(function(d) {{ if (typeof d === 'object' && d !== null) {{ const keys = Object.keys(d); for (const k of keys) {{ if (typeof d[k] === 'string') {{ حالة.{var} = d[k]; return; }} }} }} حالة.{var} = (typeof d === 'string') ? d : JSON.stringify(d); }})\n{indent}  .catch(function(e) {{ حالة.{var} = 'خطأ: ' + e.message; }});\n",
                    indent, indent = indent, url = url_js, var = var_name
                )
            }
        } else {
            format!(
                "{}fetch({url})\n{indent}  .then(function(r) {{ return r.text(); }})\n{indent}  .then(function(d) {{ حالة.{var} = d; }})\n{indent}  .catch(function(e) {{ حالة.{var} = 'خطأ: ' + e.message; }});\n",
                indent, indent = indent, url = url_js, var = var_name
            )
        }
    }

    fn gen_html(&mut self, stmt: &Statement, env: &mut HashMap<String, Value>, state_vars: &[String]) -> Result<String, String> {
        match stmt {
            Statement::HtmlElement { tag, content, attrs, children, events, .. } => {
                let mut attr_str = String::new();
                let mut existing_id: Option<String> = None;
                for (k, v) in attrs {
                    let val = eval(v, env)?.to_display();
                    if k == "id" { existing_id = Some(val.clone()); }
                    attr_str.push_str(&format!(" {}=\"{}\"", k, val));
                }

                if tag == "video" || tag == "audio" {
                    attr_str.push_str(" controls");
                }

                let needs_reactive = content.as_ref().map(|c| self.is_reactive(c, state_vars)).unwrap_or(false);
                let needs_id = needs_reactive || !events.is_empty();
                let id = if needs_id && existing_id.is_none() { Some(self.next_id()) } else { existing_id.clone() };

                if existing_id.is_none() {
                    if let Some(ref i) = id { attr_str.push_str(&format!(" id=\"{}\"", i)); }
                }

                let (open, close, self_closing) = match tag.as_str() {
                    "img" | "input" | "br" | "hr" => (format!("<{}{}>", tag, attr_str), String::new(), true),
                    _ => (format!("<{}{}>", tag, attr_str), format!("</{}>", tag), false),
                };

                let inner = if self_closing { String::new() }
                else if let Some(ref ch) = children {
                    let mut s = String::new();
                    for c in ch { s.push_str(&self.gen_html(c, env, state_vars)?); }
                    s
                } else if let Some(c) = content {
                    if needs_reactive {
                        if let Some(ref i) = id {
                            self.updates_js.push_str(&format!(
                                "  document.getElementById('{}').textContent = {};\n", i, self.expr_to_js(c)
                            ));
                        }
                        String::new()
                    } else { eval(c, env)?.to_display() }
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
                    for st in then_branch { then_html.push_str(&self.gen_html(st, env, state_vars)?); }
                    let mut else_html = String::new();
                    for st in else_branch { else_html.push_str(&self.gen_html(st, env, state_vars)?); }
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
                    for st in branch { s.push_str(&self.gen_html(st, env, state_vars).unwrap_or_default()); }
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
                        for st in body { s.push_str(&self.gen_html(st, env, state_vars).unwrap_or_default()); }
                    }
                    *env = old_env;
                    Ok(s)
                } else { Ok(String::new()) }
            }

            Statement::RangeFor { var, start, end, step, body, .. } => {
                let start_val = eval(start, env)?.as_num()? as i64;
                let end_val = eval(end, env)?.as_num()? as i64;
                let step_val = if let Some(s) = step {
                    let sv = eval(s, env)?.as_num()? as i64;
                    if sv == 0 { 1 } else { sv.abs() }
                } else { 1 };

                let mut s = String::new();
                let old_env = env.clone();
                let mut i = start_val;
                while (step_val > 0 && i <= end_val) || (step_val < 0 && i >= end_val) {
                    env.insert(var.clone(), Value::Num(i as f64));
                    for st in body {
                        s.push_str(&self.gen_html(st, env, state_vars).unwrap_or_default());
                    }
                    i += step_val;
                }
                *env = old_env;
                Ok(s)
            }

            Statement::TryCatch { try_body, catch_var, catch_body, .. } => {
                let mut s = String::new();
                let mut error_occurred = false;

                for st in try_body {
                    match self.gen_html(st, env, state_vars) {
                        Ok(html) => s.push_str(&html),
                        Err(_) => {
                            error_occurred = true;
                            break;
                        }
                    }
                }

                if error_occurred {
                    s.clear();
                    let old_env = env.clone();
                    env.insert(catch_var.clone(), Value::Str("خطأ".into()));
                    for st in catch_body {
                        s.push_str(&self.gen_html(st, env, state_vars).unwrap_or_default());
                    }
                    *env = old_env;
                }

                Ok(s)
            }

            Statement::Call { name, args, .. } => {
                if name == "_skip_" { return Ok(String::new()); }

                // ===== 1) مكتبة المكونات الجاهزة =====
                if let Some(html) = self.gen_builtin(name, args, env) {
                    return Ok(html);
                }

                // ===== 2) مكونات المستخدم =====
                let component = self.components.get(name).cloned();
                if let Some((params, body)) = component {
                    if self.depth > 20 { return Err("استدعاء متكرر لا نهائي".into()); }
                    let mut new_env: HashMap<String, Value> = HashMap::new();
                    for (i, p) in params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            let v = eval(arg, env)?;
                            new_env.insert(p.clone(), v);
                        }
                    }
                    self.depth += 1;
                    let mut html = String::new();
                    for stmt in &body { html.push_str(&self.gen_html(stmt, &mut new_env, state_vars)?); }
                    self.depth -= 1;
                    Ok(html)
                } else if name == "احفظ" {
                    let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x)).collect();
                    self.events_js.push_str(&format!("احفظ({});\n", a.join(", ")));
                    Ok(String::new())
                } else if name == "اجلب" || name == "اجلب_نص" {
                    let code = self.gen_fetch_call(name, args, "  ");
                    self.events_js.push_str(&code);
                    Ok(String::new())
                } else {
                    Ok(String::new())
                }
            }

            Statement::Let { name, value, .. } => {
                let v = eval(value, env)?;
                env.insert(name.clone(), v);
                Ok(String::new())
            }
            Statement::Const { name, value, .. } => {
                let v = eval(value, env)?;
                env.insert(name.clone(), v);
                Ok(String::new())
            }
            Statement::Assignment { name, value, .. } => {
                let v = eval(value, env)?;
                env.insert(name.clone(), v);
                Ok(String::new())
            }

            _ => Ok(String::new()),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

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

    if args.len() > 1 && args[1] == "test" {
        let test_file = if args.len() > 2 { &args[2] } else { "index.rino" };
        let test_path = PathBuf::from(test_file);
        if !test_path.exists() {
            eprintln!("❌ الملف غير موجود: {}", test_file);
            std::process::exit(1);
        }
        let src = fs::read_to_string(&test_path).expect("فشل قراءة الملف");
        let mut v = HashSet::new();
        if let Ok(c) = test_path.canonicalize() { v.insert(c); }
        let dir = test_path.parent().unwrap_or(Path::new("."));
        let src = process_imports(&src, dir, &mut v, 0).expect("فشل معالجة الاستيرادات");

        let mut lx = Lexer::new(&src);
        let tk = lx.tokenize().expect("خطأ لغوي");
        let mut pr = Parser::new(tk);
        let prog = pr.parse().expect("خطأ نحوي");

        let exit_code = run_tests(&prog);
        std::process::exit(exit_code);
    }

    let input_path: PathBuf = if args.len() > 1 { PathBuf::from(&args[1]) } else { PathBuf::from("index.rino") };
    if !input_path.exists() {
        eprintln!("❌ الملف غير موجود: {}", input_path.display());
        eprintln!();
        eprintln!("الاستخدام:");
        eprintln!("   rino <ملف.rino>         — بناء الملف");
        eprintln!("   rino format <ملف.rino>  — تنسيق الملف");
        eprintln!("   rino test <ملف.rino>    — تشغيل الاختبارات");
        std::process::exit(1);
    }

    let input_dir = input_path.parent().map(|p| if p.as_os_str().is_empty() { PathBuf::from(".") } else { p.to_path_buf() }).unwrap_or_else(|| PathBuf::from("."));
    let base_name = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index").to_string();

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

    let mut visited = HashSet::new();
    if let Ok(canonical) = input_path.canonicalize() { visited.insert(canonical); }
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

    for s in &program.body {
        if let Statement::Let { name, value, .. } = s {
            if let Ok(v) = eval(value, &env) {
                env.insert(name.clone(), v);
            }
        }
        if let Statement::Const { name, value, .. } = s {
            if let Ok(v) = eval(value, &env) {
                env.insert(name.clone(), v);
            }
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
    for c in &program.components {
        if let Statement::ComponentDef { name, params, body, .. } = c {
            cg.components.insert(name.clone(), (params.clone(), body.clone()));
        }
    }

    let mut body_html = String::new();
    for s in &program.body {
        body_html.push_str(&cg.gen_html(s, &mut env, &state_vars).unwrap_or_default());
    }

    // CSS — يبدأ بـ BUILTIN_CSS ثم CSS المستخدم
    let mut css = String::from(BUILTIN_CSS);
    css.push('\n');
    for r in &program.styles {
        let selector_prefix = match r.selector_kind {
            SelectorKind::Tag => "",
            SelectorKind::Class => ".",
            SelectorKind::Id => "#",
        };
        css.push_str(&format!("{}{} {{\n", selector_prefix, r.selector));
        for (p, v) in &r.properties {
            css.push_str(&format!("  {}: {};\n", css_property(p), css_value(v)));
        }
        css.push_str("}\n");
        if !r.hover_properties.is_empty() {
            css.push_str(&format!("{}{}:hover {{\n", selector_prefix, r.selector));
            for (p, v) in &r.hover_properties {
                css.push_str(&format!("  {}: {};\n", css_property(p), css_value(v)));
            }
            css.push_str("}\n");
        }
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
function مفاتيح(d) { return Object.keys(d); }
function قيم(d) { return Object.values(d); }
function يحتوي_مفتاح(d, k) { return k in d; }
function احفظ(مفتاح, قيمة) {
  try {
    localStorage.setItem(String(مفتاح), JSON.stringify(قيمة));
  } catch (e) {
    console.error("فشل الحفظ:", e);
  }
}
function اقرأ_محلي(مفتاح) {
  try {
    const v = localStorage.getItem(String(مفتاح));
    return v !== null ? JSON.parse(v) : null;
  } catch (e) {
    return null;
  }
}
function أضف(قائمة, قيمة) {
  const l = قائمة.slice();
  l.push(قيمة);
  return l;
}
function احذف(قائمة, قيمة) {
  const l = قائمة.slice();
  const idx = l.indexOf(قيمة);
  if (idx >= 0) l.splice(idx, 1);
  return l;
}
function اعكس(قائمة) { return قائمة.slice().reverse(); }
function دمج(قائمة, فاصل) { return قائمة.join(فاصل === undefined ? "" : فاصل); }
function أول(قائمة) { return قائمة[0]; }
function آخر(قائمة) { return قائمة[قائمة.length - 1]; }
function مفهرس(قائمة, فهرس) { return قائمة[فهرس]; }
function يحتوي_قائمة(قائمة, قيمة) { return قائمة.indexOf(قيمة) >= 0; }
function رتب(قائمة) { return قائمة.slice().sort((a, b) => (typeof a === "number" && typeof b === "number") ? a - b : String(a).localeCompare(String(b), "ar")); }
function مدى(من, إلى) {
  const arr = [];
  if (من <= إلى) { for (let i = من; i <= إلى; i++) arr.push(i); }
  else { for (let i = من; i >= إلى; i--) arr.push(i); }
  return arr;
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
            state_init, events = cg.events_js, updates = cg.updates_js, helpers = helpers
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
        title = title, css_file = css_filename, js_file = js_filename, body = body_html
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