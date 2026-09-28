//! Rino — لغة عربية لبناء الويب.
//! الإصدار 5.0 — محرك إشارات (Signals)

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

// ============================================
// ثوابت JS
// ============================================

const SIGNALS_JS: &str = r##"
let _currentSub = null;
let _batchDepth = 0;
const _pendingEffects = new Set();

function signal(initial) {
    let value = initial;
    const subs = new Set();
    function get() {
        if (_currentSub) subs.add(_currentSub);
        return value;
    }
    function set(v) {
        if (Object.is(v, value)) return;
        value = v;
        if (_batchDepth > 0) {
            for (const s of subs) _pendingEffects.add(s);
        } else {
            const snapshot = Array.from(subs);
            for (const s of snapshot) {
                try { s(); } catch (e) { console.error("effect error:", e); }
            }
        }
    }
    function update(fn) { set(fn(value)); }
    function peek() { return value; }
    function subscribe(fn) { subs.add(fn); return () => subs.delete(fn); }
    return { get, set, update, peek, subscribe, _subs: subs, _isSignal: true };
}

function createEffect(fn) {
    let _prev = _currentSub;
    let mounted = false;
    function run() {
        if (mounted) return;
        mounted = true;
        _currentSub = run;
        try { fn(); }
        finally { _currentSub = _prev; mounted = false; }
    }
    run();
    return run;
}

function createMemo(fn) {
    const s = signal(undefined);
    createEffect(() => {
        try { s.set(fn()); } catch (e) { console.error("memo error:", e); }
    });
    return s;
}

function batch(fn) {
    _batchDepth++;
    try { fn(); }
    finally {
        _batchDepth--;
        if (_batchDepth === 0 && _pendingEffects.size > 0) {
            const pending = Array.from(_pendingEffects);
            _pendingEffects.clear();
            for (const e of pending) {
                try { e(); } catch (err) { console.error("batch effect error:", err); }
            }
        }
    }
}

function untrack(fn) {
    const prev = _currentSub;
    _currentSub = null;
    try { return fn(); } finally { _currentSub = prev; }
}

function _html_list(items, renderFn) {
    let out = "";
    if (!items) return out;
    for (const item of items) out += renderFn(item);
    return out;
}

function _list_render(src, renderFn) {
    const arr = (src && src._isSignal) ? src.get() : src;
    return _html_list(arr, renderFn);
}
"##;

const BUILTIN_CSS: &str = r##"
.rino-btn { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: #fff; padding: 12px 28px; border: none; border-radius: 10px; font-size: 16px; font-weight: bold; cursor: pointer; transition: all 0.3s ease; margin: 6px; font-family: inherit; }
.rino-btn:hover { transform: translateY(-3px); box-shadow: 0 10px 20px rgba(102,126,234,0.4); }
.rino-card { background: #fff; border-radius: 14px; padding: 24px; box-shadow: 0 4px 14px rgba(0,0,0,0.08); margin: 16px auto; max-width: 420px; text-align: center; border: 1px solid #eef0f5; transition: all 0.3s ease; }
.rino-card:hover { transform: translateY(-4px); box-shadow: 0 10px 24px rgba(0,0,0,0.12); }
.rino-card h3 { color: #2d3748; margin: 0 0 12px 0; font-size: 22px; }
.rino-card p { color: #718096; margin: 0 0 16px 0; font-size: 16px; line-height: 1.6; }
.rino-card .rino-price { color: #667eea; font-size: 24px; font-weight: bold; }
.rino-alert { padding: 14px 20px; border-radius: 10px; margin: 12px auto; max-width: 520px; text-align: center; font-size: 16px; font-weight: 500; border-right: 5px solid; }
.rino-success { background: #d4edda; color: #155724; border-color: #28a745; }
.rino-error { background: #f8d7da; color: #721c24; border-color: #dc3545; }
.rino-warning { background: #fff3cd; color: #856404; border-color: #ffc107; }
.rino-info { background: #d1ecf1; color: #0c5460; border-color: #17a2b8; }
.rino-progress { width: 90%; max-width: 500px; height: 24px; background: #e9ecef; border-radius: 12px; margin: 12px auto; overflow: hidden; }
.rino-progress-bar { height: 100%; background: linear-gradient(90deg, #667eea, #764ba2); border-radius: 12px; transition: width 0.4s ease; display: flex; align-items: center; justify-content: center; color: #fff; font-size: 13px; font-weight: bold; }
.rino-highlight { background: #fff8dc; border-right: 4px solid #ffa500; padding: 14px 20px; margin: 12px auto; max-width: 600px; border-radius: 8px; color: #5a4a00; font-size: 17px; text-align: center; }
.rino-header { background: linear-gradient(135deg, #667eea, #764ba2); color: #fff; padding: 22px; border-radius: 12px; text-align: center; font-size: 26px; font-weight: bold; margin: 16px auto; max-width: 700px; box-shadow: 0 6px 16px rgba(102,126,234,0.3); }
.rino-divider { height: 2px; background: linear-gradient(90deg, transparent, #667eea, transparent); margin: 24px auto; max-width: 400px; border: none; }
"##;

const HELPERS_JS: &str = r##"
function escape_html(s) {
  return String(s)
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}
function عدد(v) { const n = parseFloat(v); return isNaN(n) ? 0 : n; }
function نص(v) { return String(v); }
function سلسلة(v) { return String(v); }
function منطقي(v) { return Boolean(v); }
function تحقق_صحة(v) { return Boolean(v); }
function رقم_صحيح(v) { return Math.trunc(Number(v) || 0); }
function رقم_عشري(v) { return Number(v) || 0; }
function نوع(v) {
  if (Array.isArray(v)) return "قائمة";
  if (v === null) return "لا_شيء";
  const t = typeof v;
  if (t === "string") return "نص";
  if (t === "number") return "عدد";
  if (t === "boolean") return "منطقي";
  if (t === "object") return "قاموس";
  return t;
}
function اقرأ(id) { const el = document.getElementById(id); return el ? el.value : ""; }
function امسح(id) { const el = document.getElementById(id); if (el) el.innerHTML = ""; }
function أضف_مهمة(id_قائمة, نص) {
  const ul = document.getElementById(id_قائمة);
  if (!ul) return;
  const li = document.createElement("li");
  li.textContent = نص;
  li.style.padding = "10px"; li.style.marginTop = "5px";
  li.style.background = "rgb(240, 240, 240)";
  li.style.borderRadius = "5px"; li.style.cursor = "pointer";
  li.title = "انقر للحذف";
  li.onclick = function() { li.remove(); };
  ul.appendChild(li);
}
function مفاتيح(d) { return Object.keys(d); }
function قيم(d) { return Object.values(d); }
function يحتوي_مفتاح(d, k) { return k in d; }
function احفظ(مفتاح, قيمة) { try { localStorage.setItem(String(مفتاح), JSON.stringify(قيمة)); } catch (e) {} }
function اقرأ_محلي(مفتاح) { try { const v = localStorage.getItem(String(مفتاح)); return v !== null ? JSON.parse(v) : null; } catch (e) { return null; } }
function أضف(قائمة, قيمة) { const l = قائمة.slice(); l.push(قيمة); return l; }
function أضف_أمام(قائمة, قيمة) { const l = قائمة.slice(); l.unshift(قيمة); return l; }
function أدرج(قائمة, فهرس, قيمة) { const l = قائمة.slice(); l.splice(فهرس, 0, قيمة); return l; }
function احذف(قائمة, قيمة) { const l = قائمة.slice(); const idx = l.indexOf(قيمة); if (idx >= 0) l.splice(idx, 1); return l; }
function احذف_أخير(قائمة) { const l = قائمة.slice(); l.pop(); return l; }
function اعكس(قائمة) { return قائمة.slice().reverse(); }
function اقلب(قائمة) { return قائمة.slice().reverse(); }
function دمج(قائمة, فاصل) { return قائمة.join(فاصل === undefined ? "" : فاصل); }
function أول(قائمة) { return قائمة[0]; }
function آخر(قائمة) { return قائمة[قائمة.length - 1]; }
function مفهرس(قائمة, فهرس) { const i = فهرس < 0 ? قائمة.length + فهرس : فهرس; return قائمة[i]; }
function يحتوي_قائمة(قائمة, قيمة) { return قائمة.indexOf(قيمة) >= 0; }
function تضمن(قائمة, قيمة) { return قائمة.indexOf(قيمة) >= 0; }
function رتب(قائمة) { return قائمة.slice().sort((a, b) => (typeof a === "number" && typeof b === "number") ? a - b : String(a).localeCompare(String(b), "ar")); }
function فرز(قائمة) { return رتب(قائمة); }
function ترتيب(قائمة) { return رتب(قائمة); }
function فرق(قائمة) { return Array.from(new Set(قائمة)); }
function مدى(من, إلى) { const arr = []; if (من <= إلى) { for (let i = من; i <= إلى; i++) arr.push(i); } else { for (let i = من; i >= إلى; i--) arr.push(i); } return arr; }
function مجموع(قائمة) { return قائمة.reduce((a, b) => a + b, 0); }
function اختصر(قائمة) { return مجموع(قائمة); }
function متوسط(قائمة) { return قائمة.length ? مجموع(قائمة) / قائمة.length : 0; }
function أدنى(قائمة) { return Math.min.apply(null, قائمة); }
function أقصى(قائمة) { return Math.max.apply(null, قائمة); }
function عشوائي(من, إلى) { return Math.floor(Math.random() * (إلى - من + 1)) + من; }
function اقتطع(نص, من, عدد) { return String(نص).substr(من, عدد); }
function قسم(نص, فاصل) { return String(نص).split(فاصل); }
function انضم(قائمة, فاصل) { return قائمة.join(فاصل); }
function بحث(نص, جزء) { return String(نص).indexOf(جزء); }
function مكرر(نص, عدد) { return String(نص).repeat(عدد); }
function ازل_فراغات(نص) { return String(نص).trim(); }
function يبدأ_بـ(نص, بداية) { return String(نص).startsWith(بداية); }
function ينتهي_بـ(نص, نهاية) { return String(نص).endsWith(نهاية); }
function كبير(نص) { return String(نص).toUpperCase(); }
function صغير(نص) { return String(نص).toLowerCase(); }
function جذر(ع) { return Math.sqrt(ع); }
function قوة(أ, ب) { return Math.pow(أ, ب); }
function قوس(ع) { return Math.round(ع); }
function تقريب(ع) { return Math.round(ع); }
function أرضي(ع) { return Math.floor(ع); }
function سقف(ع) { return Math.ceil(ع); }
function مطلق(ع) { return Math.abs(ع); }
function أصغر(أ, ب) { return Math.min(أ, ب); }
function أكبر(أ, ب) { return Math.max(أ, ب); }
function مثلث(ع) { return Math.sin(ع); }
function جا(ع) { return Math.sin(ع); }
function جيب(ع) { return Math.cos(ع); }
function جتا(ع) { return Math.cos(ع); }
function ظل(ع) { return Math.tan(ع); }
function زمن() { return Math.floor(Date.now() / 1000); }
function سنة() { return new Date().getFullYear(); }
function تاريخ() { return new Date().toLocaleDateString("ar"); }
function طول(v) {
  if (typeof v === "string") return Array.from(v).length;
  if (Array.isArray(v)) return v.length;
  if (v && typeof v === "object") return Object.keys(v).length;
  return 0;
}
function اطبع() { console.log.apply(console, arguments); }
"##;

// ============================================
// أدوات مساعدة عامة
// ============================================

fn strip_comment(line: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' { in_string = !in_string; result.push(c); }
        else if !in_string && c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' { break; }
        else { result.push(c); }
        i += 1;
    }
    result
}

fn preprocess_indentation(source: &str) -> String {
    let has_braces = source.lines().any(|line| {
        let no_comment = strip_comment(line);
        let trimmed = no_comment.trim_end();
        trimmed.ends_with('{') || trimmed.ends_with('}')
    });
    if has_braces { return source.to_string(); }
    let mut output = String::new();
    let mut indent_stack: Vec<usize> = Vec::new();
    for line in source.lines() {
        let content = strip_comment(line);
        let trimmed = content.trim_end();
        if trimmed.trim().is_empty() { output.push('\n'); continue; }
        let indent = content.len() - content.trim_start().len();
        while let Some(&top) = indent_stack.last() {
            if indent <= top {
                indent_stack.pop();
                for _ in 0..top { output.push(' '); }
                output.push_str("}\n");
            } else { break; }
        }
        if trimmed.ends_with(':') {
            let without = trimmed[..trimmed.len() - 1].trim_end();
            output.push_str(without);
            output.push_str(" {\n");
            indent_stack.push(indent);
        } else {
            output.push_str(trimmed);
            output.push('\n');
        }
    }
    while let Some(top) = indent_stack.pop() {
        for _ in 0..top { output.push(' '); }
        output.push_str("}\n");
    }
    output
}

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
                let new_base = file_path.parent().map(|p| p.to_path_buf())
                    .unwrap_or_else(|| base_dir.to_path_buf());
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

fn js_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn html_attr_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
     .replace('<', "&lt;").replace('>', "&gt;")
}

fn is_self_closing_tag(tag: &str) -> bool {
    matches!(tag, "img" | "input" | "br" | "hr" | "col" | "source" | "track" | "area" | "wbr")
}

// ============================================
// القيم
// ============================================

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
            Value::Str(s) => s.trim().parse().map_err(|_| format!("\"{}\" ليس عددًا", s)),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
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
            Value::Str(s) => format!("\"{}\"", js_escape(s)),
            Value::Num(n) => if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) },
            Value::Bool(b) => if *b { "true".into() } else { "false".into() },
            Value::Null => "null".into(),
            Value::List(items) => {
                let i: Vec<String> = items.iter().map(|x| x.to_js_literal()).collect();
                format!("[{}]", i.join(", "))
            }
            Value::Dict(pairs) => {
                let p: Vec<String> = pairs.iter()
                    .map(|(k, v)| format!("\"{}\": {}", js_escape(k), v.to_js_literal()))
                    .collect();
                format!("{{{}}}", p.join(", "))
            }
        }
    }
}

// ============================================
// المفسر
// ============================================

type FuncMap = HashMap<String, (Vec<String>, Vec<Statement>)>;

#[derive(Debug)]
enum Flow { Normal, Return(Value), Break, Continue }

struct Interp<'a> {
    funcs: &'a FuncMap,
    depth: usize,
}

impl<'a> Interp<'a> {
    fn new(funcs: &'a FuncMap) -> Self { Self { funcs, depth: 0 } }

    fn eval(&mut self, expr: &Expression, env: &HashMap<String, Value>) -> Result<Value, String> {
        if self.depth > 300 { return Err("عمق التقييم كبير جدًا".into()); }
        match expr {
            Expression::String(s) => Ok(Value::Str(s.clone())),
            Expression::Number(n) => Ok(Value::Num(*n)),
            Expression::Boolean(b) => Ok(Value::Bool(*b)),
            Expression::Null => Ok(Value::Null),
            Expression::Identifier(name) => env.get(name).cloned()
                .ok_or_else(|| format!("المتغير \"{}\" غير معرّف", name)),
            Expression::List(items) => {
                let mut vs = Vec::new();
                for i in items { vs.push(self.eval(i, env)?); }
                Ok(Value::List(vs))
            }
            Expression::Dict(pairs) => {
                let mut map = Vec::new();
                for (k, v) in pairs { map.push((k.clone(), self.eval(v, env)?)); }
                Ok(Value::Dict(map))
            }
            Expression::MemberAccess { object, property } => {
                let obj = self.eval(object, env)?;
                match obj {
                    Value::Dict(pairs) => pairs.into_iter()
                        .find(|(k, _)| k == property)
                        .map(|(_, v)| v)
                        .ok_or_else(|| format!("الخاصية \"{}\" غير موجودة", property)),
                    _ => Err("لا يمكن الوصول لخاصية من نوع غير قاموس".into()),
                }
            }
            Expression::Index { object, index } => {
                let obj = self.eval(object, env)?;
                let idx = self.eval(index, env)?;
                match obj {
                    Value::List(l) => {
                        let i = idx.as_num()? as i64;
                        let i = if i < 0 { (l.len() as i64 + i).max(0) as usize } else { i as usize };
                        l.get(i).cloned().ok_or_else(|| format!("الفهرس {} خارج الحدود", i))
                    }
                    Value::Str(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        let i = idx.as_num()? as i64;
                        let i = if i < 0 { (chars.len() as i64 + i).max(0) as usize } else { i as usize };
                        chars.get(i).map(|c| Value::Str(c.to_string()))
                            .ok_or_else(|| format!("الفهرس {} خارج الحدود", i))
                    }
                    Value::Dict(d) => {
                        let key = idx.to_display();
                        d.into_iter().find(|(k, _)| k == &key).map(|(_, v)| v)
                            .ok_or_else(|| format!("المفتاح \"{}\" غير موجود", key))
                    }
                    _ => Err("لا يمكن الفهرسة على هذا النوع".into()),
                }
            }
            Expression::Call { name, args } => {
                if let Some((params, body)) = self.funcs.get(name) {
                    let params = params.clone();
                    let body = body.clone();
                    let mut new_env: HashMap<String, Value> = HashMap::new();
                    for (i, p) in params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            let v = self.eval(arg, env)?;
                            new_env.insert(p.clone(), v);
                        }
                    }
                    self.depth += 1;
                    let mut result = Flow::Normal;
                    for stmt in &body {
                        result = self.exec(stmt, &mut new_env)?;
                        if matches!(result, Flow::Return(_)) { break; }
                    }
                    self.depth -= 1;
                    return Ok(match result { Flow::Return(v) => v, _ => Value::Null });
                }
                let mut vs = Vec::new();
                for a in args { vs.push(self.eval(a, env)?); }
                self.call_builtin(name, &vs)
            }
            Expression::Binary { left, op, right } => {
                let l = self.eval(left, env)?;
                let r = self.eval(right, env)?;
                self.binop(l, op, r)
            }
            Expression::Comparison { left, op, right } => {
                let l = self.eval(left, env)?;
                let r = self.eval(right, env)?;
                self.cmpop(l, op, r)
            }
            Expression::Logical { left, op, right } => {
                let l = self.eval(left, env)?;
                match op {
                    LogOp::And => if !l.as_bool() { Ok(Value::Bool(false)) }
                        else { Ok(Value::Bool(self.eval(right, env)?.as_bool())) },
                    LogOp::Or => if l.as_bool() { Ok(Value::Bool(true)) }
                        else { Ok(Value::Bool(self.eval(right, env)?.as_bool())) },
                }
            }
            Expression::Not(e) => Ok(Value::Bool(!self.eval(e, env)?.as_bool())),
            Expression::Neg(e) => Ok(Value::Num(-self.eval(e, env)?.as_num()?)),
        }
    }

    fn binop(&self, l: Value, op: &BinOp, r: Value) -> Result<Value, String> {
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

    fn cmpop(&self, l: Value, op: &CmpOp, r: Value) -> Result<Value, String> {
        let res = match op {
            CmpOp::Eq => match (&l, &r) {
                (Value::Num(a), Value::Num(b)) => a == b,
                _ => l.to_display() == r.to_display(),
            },
            CmpOp::Ne => match (&l, &r) {
                (Value::Num(a), Value::Num(b)) => a != b,
                _ => l.to_display() != r.to_display(),
            },
            CmpOp::Gt => l.as_num()? > r.as_num()?,
            CmpOp::Lt => l.as_num()? < r.as_num()?,
            CmpOp::Ge => l.as_num()? >= r.as_num()?,
            CmpOp::Le => l.as_num()? <= r.as_num()?,
        };
        Ok(Value::Bool(res))
    }

    fn exec(&mut self, stmt: &Statement, env: &mut HashMap<String, Value>) -> Result<Flow, String> {
        match stmt {
            Statement::Let { name, value, .. } |
            Statement::Const { name, value, .. } |
            Statement::Assignment { name, value, .. } => {
                let v = self.eval(value, env)?;
                env.insert(name.clone(), v);
                Ok(Flow::Normal)
            }
            Statement::Return { value, .. } => {
                let v = match value { Some(e) => self.eval(e, env)?, None => Value::Null };
                Ok(Flow::Return(v))
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                let c = self.eval(condition, env)?.as_bool();
                let branch = if c { then_branch } else { else_branch };
                for st in branch {
                    let f = self.exec(st, env)?;
                    if !matches!(f, Flow::Normal) { return Ok(f); }
                }
                Ok(Flow::Normal)
            }
            Statement::ForEach { var, iterable, body, .. } => {
                let v = self.eval(iterable, env)?;
                let items: Vec<Value> = match v {
                    Value::List(l) => l,
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Dict(p) => p.into_iter().map(|(k, _)| Value::Str(k)).collect(),
                    _ => return Err("لكل تحتاج قائمة أو نصًا أو قاموسًا".into()),
                };
                for item in items {
                    env.insert(var.clone(), item);
                    for st in body {
                        match self.exec(st, env)? {
                            Flow::Normal => {}
                            Flow::Break => return Ok(Flow::Normal),
                            Flow::Continue => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                        }
                    }
                }
                Ok(Flow::Normal)
            }
            Statement::RangeFor { var, start, end, step, body, .. } => {
                let s = self.eval(start, env)?.as_num()? as i64;
                let e = self.eval(end, env)?.as_num()? as i64;
                let st = match step {
                    Some(x) => { let v = self.eval(x, env)?.as_num()? as i64; if v == 0 { 1 } else { v } }
                    None => 1,
                };
                let mut i = s;
                let mut guard = 0;
                while (st > 0 && i <= e) || (st < 0 && i >= e) {
                    guard += 1;
                    if guard > 1_000_000 { return Err("حلقة من..إلى استمرت طويلًا".into()); }
                    env.insert(var.clone(), Value::Num(i as f64));
                    for stmt in body {
                        match self.exec(stmt, env)? {
                            Flow::Normal => {}
                            Flow::Break => return Ok(Flow::Normal),
                            Flow::Continue => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                        }
                    }
                    i += st;
                }
                Ok(Flow::Normal)
            }
            Statement::While { condition, body, .. } => {
                let mut guard = 0;
                while self.eval(condition, env)?.as_bool() {
                    guard += 1;
                    if guard > 1_000_000 { return Err("حلقة بينما استمرت طويلًا".into()); }
                    for stmt in body {
                        match self.exec(stmt, env)? {
                            Flow::Normal => {}
                            Flow::Break => return Ok(Flow::Normal),
                            Flow::Continue => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                        }
                    }
                }
                Ok(Flow::Normal)
            }
            Statement::Break { .. } => Ok(Flow::Break),
            Statement::Continue { .. } => Ok(Flow::Continue),
            Statement::TryCatch { try_body, catch_var, catch_body, .. } => {
                let mut err: Option<String> = None;
                let mut result = Flow::Normal;
                for st in try_body {
                    match self.exec(st, env) {
                        Ok(f) => { if !matches!(f, Flow::Normal) { result = f; break; } }
                        Err(e) => { err = Some(e); break; }
                    }
                }
                if let Some(e) = err {
                    env.insert(catch_var.clone(), Value::Str(e));
                    for st in catch_body {
                        let f = self.exec(st, env)?;
                        if !matches!(f, Flow::Normal) { return Ok(f); }
                    }
                }
                Ok(result)
            }
            Statement::Call { name, args, .. } => {
                let _ = self.eval(&Expression::Call { name: name.clone(), args: args.clone() }, env)?;
                Ok(Flow::Normal)
            }
            _ => Ok(Flow::Normal),
        }
    }

    fn call_builtin(&self, name: &str, vs: &[Value]) -> Result<Value, String> {
        let get_num = |i: usize| -> Result<f64, String> {
            vs.get(i).and_then(|v| v.as_num().ok())
                .ok_or_else(|| format!("{} تحتاج عددًا في الموضع {}", name, i + 1))
        };
        match name {
            "طول" => match vs.first() {
                Some(Value::List(l)) => Ok(Value::Num(l.len() as f64)),
                Some(Value::Str(s)) => Ok(Value::Num(s.chars().count() as f64)),
                Some(Value::Dict(p)) => Ok(Value::Num(p.len() as f64)),
                _ => Err("طول تحتاج قائمة أو نصًا أو قاموسًا".into()),
            },
            "نوع" => match vs.first() {
                Some(Value::Str(_)) => Ok(Value::Str("نص".into())),
                Some(Value::Num(_)) => Ok(Value::Str("عدد".into())),
                Some(Value::Bool(_)) => Ok(Value::Str("منطقي".into())),
                Some(Value::Null) => Ok(Value::Str("لا_شيء".into())),
                Some(Value::List(_)) => Ok(Value::Str("قائمة".into())),
                Some(Value::Dict(_)) => Ok(Value::Str("قاموس".into())),
                None => Err("نوع تحتاج قيمة".into()),
            },
            "تحقق_صحة" => Ok(Value::Bool(vs.first().map(|v| v.as_bool()).unwrap_or(false))),
            "عدد" => match vs.first() {
                Some(v) => Ok(Value::Num(v.as_num()?)),
                None => Err("عدد تحتاج قيمة".into()),
            },
            "سلسلة" | "نص" => match vs.first() {
                Some(v) => Ok(Value::Str(v.to_display())),
                None => Err("سلسلة تحتاج قيمة".into()),
            },
            "منطقي" => match vs.first() {
                Some(v) => Ok(Value::Bool(v.as_bool())),
                None => Err("منطقي تحتاج قيمة".into()),
            },
            "رقم_صحيح" => Ok(Value::Num(get_num(0)?.trunc())),
            "رقم_عشري" => Ok(Value::Num(get_num(0)?)),
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
                (Some(Value::Str(s)), Some(Value::Str(a)), Some(Value::Str(b))) =>
                    Ok(Value::Str(s.replace(a.as_str(), b.as_str()))),
                _ => Err("استبدل تحتاج 3 نصوص".into()),
            },
            "اقتطع" => match (vs.get(0), vs.get(1), vs.get(2)) {
                (Some(Value::Str(s)), Some(a), Some(b)) => {
                    let chars: Vec<char> = s.chars().collect();
                    let start = a.as_num()? as usize;
                    let count = b.as_num()? as usize;
                    Ok(Value::Str(chars.iter().skip(start).take(count).collect()))
                }
                _ => Err("اقتطع تحتاج (نص, من, عدد)".into()),
            },
            "قسم" => match (vs.get(0), vs.get(1)) {
                (Some(Value::Str(s)), Some(Value::Str(sep))) =>
                    Ok(Value::List(s.split(sep.as_str()).map(|x| Value::Str(x.to_string())).collect())),
                _ => Err("قسم تحتاج (نص, فاصل)".into()),
            },
            "انضم" => match (vs.get(0), vs.get(1)) {
                (Some(Value::List(l)), Some(Value::Str(sep))) =>
                    Ok(Value::Str(l.iter().map(|v| v.to_display()).collect::<Vec<_>>().join(sep))),
                _ => Err("انضم تحتاج (قائمة, فاصل)".into()),
            },
            "بحث" => match (vs.get(0), vs.get(1)) {
                (Some(Value::Str(s)), Some(Value::Str(sub))) =>
                    Ok(Value::Num(s.find(sub.as_str()).map(|i| i as f64).unwrap_or(-1.0))),
                _ => Err("بحث تحتاج (نص, جزء)".into()),
            },
            "مكرر" => match (vs.get(0), vs.get(1)) {
                (Some(Value::Str(s)), Some(n)) => Ok(Value::Str(s.repeat(n.as_num()? as usize))),
                _ => Err("مكرر تحتاج (نص, عدد)".into()),
            },
            "ازل_فراغات" => match vs.first() {
                Some(Value::Str(s)) => Ok(Value::Str(s.trim().to_string())),
                _ => Err("ازل_فراغات تحتاج نصًا".into()),
            },
            "يبدأ_بـ" => match (vs.get(0), vs.get(1)) {
                (Some(Value::Str(s)), Some(Value::Str(p))) => Ok(Value::Bool(s.starts_with(p.as_str()))),
                _ => Err("يبدأ_بـ تحتاج نصين".into()),
            },
            "ينتهي_بـ" => match (vs.get(0), vs.get(1)) {
                (Some(Value::Str(s)), Some(Value::Str(p))) => Ok(Value::Bool(s.ends_with(p.as_str()))),
                _ => Err("ينتهي_بـ تحتاج نصين".into()),
            },
            "جذر" => Ok(Value::Num(get_num(0)?.sqrt())),
            "قوة" => Ok(Value::Num(get_num(0)?.powf(get_num(1)?))),
            "قوس" | "تقريب" => Ok(Value::Num(get_num(0)?.round())),
            "أرضي" => Ok(Value::Num(get_num(0)?.floor())),
            "سقف" => Ok(Value::Num(get_num(0)?.ceil())),
            "مطلق" => Ok(Value::Num(get_num(0)?.abs())),
            "أصغر" => Ok(Value::Num(get_num(0)?.min(get_num(1)?))),
            "أكبر" => Ok(Value::Num(get_num(0)?.max(get_num(1)?))),
            "مثلث" | "جا" => Ok(Value::Num(get_num(0)?.sin())),
            "جيب" | "جتا" => Ok(Value::Num(get_num(0)?.cos())),
            "ظل" => Ok(Value::Num(get_num(0)?.tan())),
            "مفاتيح" => match vs.first() {
                Some(Value::Dict(p)) => Ok(Value::List(p.iter().map(|(k, _)| Value::Str(k.clone())).collect())),
                _ => Err("مفاتيح تحتاج قاموسًا".into()),
            },
            "قيم" => match vs.first() {
                Some(Value::Dict(p)) => Ok(Value::List(p.iter().map(|(_, v)| v.clone()).collect())),
                _ => Err("قيم تحتاج قاموسًا".into()),
            },
            "يحتوي_مفتاح" => match (vs.get(0), vs.get(1)) {
                (Some(Value::Dict(p)), Some(Value::Str(k))) =>
                    Ok(Value::Bool(p.iter().any(|(key, _)| key == k))),
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
            "أضف_أمام" => match (vs.get(0), vs.get(1)) {
                (Some(Value::List(l)), Some(v)) => {
                    let mut new_list = l.clone();
                    new_list.insert(0, v.clone());
                    Ok(Value::List(new_list))
                }
                _ => Err("أضف_أمام تحتاج قائمة وقيمة".into()),
            },
            "أدرج" => match (vs.get(0), vs.get(1), vs.get(2)) {
                (Some(Value::List(l)), Some(i), Some(v)) => {
                    let mut new_list = l.clone();
                    let idx = (i.as_num()? as usize).min(new_list.len());
                    new_list.insert(idx, v.clone());
                    Ok(Value::List(new_list))
                }
                _ => Err("أدرج تحتاج (قائمة, فهرس, قيمة)".into()),
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
            "احذف_أخير" => match vs.first() {
                Some(Value::List(l)) => {
                    let mut new_list = l.clone();
                    new_list.pop();
                    Ok(Value::List(new_list))
                }
                _ => Err("احذف_أخير تحتاج قائمة".into()),
            },
            "اعكس" | "اقلب" => match vs.first() {
                Some(Value::List(l)) => {
                    let mut new_list = l.clone();
                    new_list.reverse();
                    Ok(Value::List(new_list))
                }
                _ => Err("اعكس تحتاج قائمة".into()),
            },
            "دمج" => match (vs.get(0), vs.get(1)) {
                (Some(Value::List(l)), Some(Value::Str(sep))) =>
                    Ok(Value::Str(l.iter().map(|v| v.to_display()).collect::<Vec<_>>().join(sep))),
                (Some(Value::List(l)), None) =>
                    Ok(Value::Str(l.iter().map(|v| v.to_display()).collect::<Vec<_>>().join(""))),
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
                    let idx = i.as_num()? as i64;
                    let idx = if idx < 0 { (l.len() as i64 + idx).max(0) as usize } else { idx as usize };
                    l.get(idx).cloned().ok_or_else(|| format!("الفهرس {} خارج الحدود", idx))
                }
                _ => Err("مفهرس تحتاج قائمة ورقمًا".into()),
            },
            "يحتوي_قائمة" | "تضمن" => match (vs.get(0), vs.get(1)) {
                (Some(Value::List(l)), Some(v)) =>
                    Ok(Value::Bool(l.iter().any(|x| x.to_display() == v.to_display()))),
                _ => Err("يحتوي_قائمة تحتاج قائمة وقيمة".into()),
            },
            "رتب" | "فرز" | "ترتيب" => match vs.first() {
                Some(Value::List(l)) => {
                    let mut new_list = l.clone();
                    new_list.sort_by(|a, b| match (a, b) {
                        (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                        _ => a.to_display().cmp(&b.to_display()),
                    });
                    Ok(Value::List(new_list))
                }
                _ => Err("رتب تحتاج قائمة".into()),
            },
            "فرق" => match vs.first() {
                Some(Value::List(l)) => {
                    let mut seen: HashSet<String> = HashSet::new();
                    let mut out = Vec::new();
                    for v in l {
                        let k = v.to_display();
                        if !seen.contains(&k) { seen.insert(k); out.push(v.clone()); }
                    }
                    Ok(Value::List(out))
                }
                _ => Err("فرق تحتاج قائمة".into()),
            },
            "بحث_ثنائي" => match (vs.get(0), vs.get(1)) {
                (Some(Value::List(l)), Some(target)) => {
                    let found = l.iter().position(|x| x.to_display() == target.to_display());
                    Ok(Value::Num(found.map(|i| i as f64).unwrap_or(-1.0)))
                }
                _ => Err("بحث_ثنائي تحتاج (قائمة, قيمة)".into()),
            },
            "مدى" => match (vs.get(0), vs.get(1)) {
                (Some(a), Some(b)) => {
                    let start = a.as_num()? as i64;
                    let end = b.as_num()? as i64;
                    let mut items = Vec::new();
                    if start <= end { for i in start..=end { items.push(Value::Num(i as f64)); } }
                    else { for i in (end..=start).rev() { items.push(Value::Num(i as f64)); } }
                    Ok(Value::List(items))
                }
                _ => Err("مدى تحتاج رقمين".into()),
            },
            "مجموع" | "اختصر" => match vs.first() {
                Some(Value::List(l)) => {
                    let mut total = 0.0;
                    for v in l { total += v.as_num()?; }
                    Ok(Value::Num(total))
                }
                _ => Err("مجموع تحتاج قائمة".into()),
            },
            "متوسط" => match vs.first() {
                Some(Value::List(l)) => {
                    if l.is_empty() { return Ok(Value::Num(0.0)); }
                    let mut total = 0.0;
                    for v in l { total += v.as_num()?; }
                    Ok(Value::Num(total / l.len() as f64))
                }
                _ => Err("متوسط تحتاج قائمة".into()),
            },
            "أدنى" => match vs.first() {
                Some(Value::List(l)) => {
                    if l.is_empty() { return Err("القائمة فارغة".into()); }
                    let mut m = l[0].as_num()?;
                    for v in l { let n = v.as_num()?; if n < m { m = n; } }
                    Ok(Value::Num(m))
                }
                _ => Err("أدنى تحتاج قائمة".into()),
            },
            "أقصى" => match vs.first() {
                Some(Value::List(l)) => {
                    if l.is_empty() { return Err("القائمة فارغة".into()); }
                    let mut m = l[0].as_num()?;
                    for v in l { let n = v.as_num()?; if n > m { m = n; } }
                    Ok(Value::Num(m))
                }
                _ => Err("أقصى تحتاج قائمة".into()),
            },
            "عشوائي" => match (vs.get(0), vs.get(1)) {
                (Some(a), Some(b)) => {
                    let min = a.as_num()? as i64;
                    let max = b.as_num()? as i64;
                    let range = (max - min + 1).max(1) as u64;
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64).unwrap_or(1);
                    Ok(Value::Num((min + (seed % range) as i64) as f64))
                }
                _ => Err("عشوائي تحتاج (من, إلى)".into()),
            },
            "زمن" | "سنة" => {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0);
                if name == "سنة" {
                    Ok(Value::Num((1970 + (now / 31536000)) as f64))
                } else {
                    Ok(Value::Num(now as f64))
                }
            }
            "تاريخ" => {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0);
                Ok(Value::Str(format!("{}", now)))
            }
            "اقرأ_محلي" | "اقرأ_مدخل" | "اجلب" | "اجلب_نص" | "احفظ" => Ok(Value::Null),
            _ => Err(format!("دالة غير معروفة: {}", name)),
        }
    }
}

// ============================================
// تشغيل الاختبارات
// ============================================

fn run_tests(program: &Program) -> i32 {
    if program.tests.is_empty() {
        println!("لا توجد اختبارات في هذا الملف");
        return 0;
    }
    let mut funcs: FuncMap = HashMap::new();
    for f in &program.functions {
        if let Statement::Function { name, params, body, .. } = f {
            funcs.insert(name.clone(), (params.clone(), body.clone()));
        }
    }
    println!("\nتشغيل {} اختبار...", program.tests.len());
    println!("-------------------------------------");
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
                            let mut interp = Interp::new(&funcs);
                            match interp.eval(arg, &env) {
                                Ok(v) => if !v.as_bool() {
                                    errors.push(format!("توقع #{} فشل: القيمة = {}",
                                        assertion_num, v.to_display()));
                                },
                                Err(e) => errors.push(format!("توقع #{}: خطأ — {}", assertion_num, e)),
                            }
                        }
                    }
                    Statement::Call { name: cname, args, .. } if cname == "توقع_يساوي" => {
                        assertion_num += 1;
                        if args.len() >= 2 {
                            let mut interp = Interp::new(&funcs);
                            let a = interp.eval(&args[0], &env);
                            let b = interp.eval(&args[1], &env);
                            match (a, b) {
                                (Ok(va), Ok(vb)) => {
                                    let da = va.to_display();
                                    let db = vb.to_display();
                                    if da != db {
                                        errors.push(format!(
                                            "توقع #{}: متوقع [{}] لكن وجد [{}]",
                                            assertion_num, db, da));
                                    }
                                }
                                (Err(e), _) | (_, Err(e)) =>
                                    errors.push(format!("توقع #{}: خطأ — {}", assertion_num, e)),
                            }
                        }
                    }
                    _ => {
                        let mut interp = Interp::new(&funcs);
                        if let Err(e) = interp.exec(stmt, &mut env) {
                            errors.push(format!("خطأ في التنفيذ: {}", e));
                        }
                    }
                }
            }
            if errors.is_empty() {
                println!("PASS: {}", name);
                passed += 1;
            } else {
                println!("FAIL: {}", name);
                for e in &errors { println!("   - {}", e); }
                failed += 1;
            }
        }
    }
    println!("-------------------------------------");
    println!("النتيجة: {} نجح | {} فشل | {} الإجمالي", passed, failed, passed + failed);
    if failed > 0 { 1 } else { 0 }
}

// ============================================
// CSS
// ============================================

fn css_property(name: &str) -> &str {
    match name {
        "لون" | "لون_النص" => "color",
        "حجم" | "حجم_خط" | "حجم_الخط" => "font-size",
        "وزن" | "وزن_الخط" => "font-weight",
        "نوع_الخط" => "font-style",
        "تحويل" => "text-transform",
        "مسافة_بين_الأسطر" => "line-height",
        "محاذاة" => "text-align",
        "زخرفة" | "زخرفة_النص" => "text-decoration",
        "مسافة_حروف" => "letter-spacing",
        "مسافة_كلمات" => "word-spacing",
        "ظل_النص" => "text-shadow",
        "خلفية" => "background",
        "لون_خلفية" => "background-color",
        "استدارة" => "border-radius",
        "حد" => "border",
        "لون_حد" | "لون_الحد" => "border-color",
        "عرض_حد" => "border-width",
        "نمط_حد" => "border-style",
        "عرض" => "width",
        "ارتفاع" => "height",
        "أقصى_عرض" => "max-width",
        "أدنى_عرض" => "min-width",
        "أقصى_ارتفاع" => "max-height",
        "أدنى_ارتفاع" => "min-height",
        "حشوة" | "حشوة_داخلية" => "padding",
        "هامش" | "هامش_خارجي" => "margin",
        "هامش_علوي" => "margin-top",
        "هامش_سفلي" => "margin-bottom",
        "هامش_يمين" => "margin-right",
        "هامش_يسار" => "margin-left",
        "حشوة_علوية" => "padding-top",
        "حشوة_سفلية" => "padding-bottom",
        "حشوة_يمين" => "padding-right",
        "حشوة_يسار" => "padding-left",
        "عرض_نوع" | "نوع_العرض" => "display",
        "اتجاه" => "direction",
        "موضع" => "position",
        "أعلى" => "top",
        "أسفل" => "bottom",
        "يمين_الموضع" | "يمين" => "right",
        "يسار_الموضع" | "يسار" => "left",
        "ترتيب" | "ترتيب_الطبقة" => "z-index",
        "طفو" => "float",
        "محتوى_زائد" | "فيض" => "overflow",
        "محاذاة_رأسية" => "vertical-align",
        "محاذاة_أفقية" => "justify-content",
        "محاذاة_رأسية_مرنة" => "align-items",
        "اتجاه_مرن" => "flex-direction",
        "التفاف" | "لف" => "flex-wrap",
        "فجوة" | "فراغ" => "gap",
        "نمو" => "flex-grow",
        "انكماش" => "flex-shrink",
        "أساس" => "flex-basis",
        "أعمدة" => "grid-template-columns",
        "صفوف" => "grid-template-rows",
        "ظل" => "box-shadow",
        "شفافية" => "opacity",
        "انتقال" => "transition",
        "تحويل_شكل" | "تحويل_الشكل" => "transform",
        "مؤشر" | "مؤشر_الفأرة" => "cursor",
        "خط_عائلة" | "عائلة_الخط" => "font-family",
        "نمط_قائمة" | "نمط_القائمة" => "list-style",
        _ => name,
    }
}

fn css_value(val: &str) -> String {
    let out = match val {
        "أحمر" => "red", "أزرق" => "blue", "أخضر" => "green",
        "أبيض" => "white", "أسود" => "black", "رمادي" => "gray",
        "أصفر" => "yellow", "برتقالي" => "orange", "بنفسجي" => "purple",
        "وردي" => "pink", "بني" => "brown", "ذهبي" => "gold",
        "فضي" => "silver", "سماوي" => "skyblue", "ليموني" => "lime",
        "أرجواني" => "magenta", "نيلي" | "كحلي" => "navy",
        "كريمي" => "beige", "شفاف" => "transparent",
        "تركوازي" | "فيروزي" => "turquoise", "مرجاني" => "coral",
        "زيتي" => "olive", "فحمي" => "dimgray",
        "أزرق_فاتح" => "lightblue", "أخضر_فاتح" => "lightgreen",
        "أحمر_فاتح" => "lightcoral", "أصفر_فاتح" => "lightyellow",
        "بنفسجي_فاتح" => "plum", "أرجواني_فاتح" => "violet",
        "قرمزي" => "crimson", "قرمزي_فاتح" => "salmon",
        "بني_فاتح" => "tan", "ذهبي_فاتح" => "wheat",
        "أزرق_غامق" => "darkblue", "أخضر_غامق" => "darkgreen",
        "أحمر_غامق" => "darkred", "برتقالي_غامق" => "darkorange",
        "بنفسجي_غامق" => "indigo", "بحر_أخضر" => "teal",
        "وسط" => "center", "يمين" => "right", "يسار" => "left",
        "أعلى" => "top", "أسفل" => "bottom",
        "ضبط" => "justify", "بداية" => "flex-start", "نهاية" => "flex-end",
        "بين" => "space-between", "حول" => "space-around", "تساو" => "space-evenly",
        "عريض" => "bold", "ضعيف" => "lighter", "مائل" => "italic", "عادي" => "normal",
        "خط_تحت" => "underline", "خط_فوق" => "overline",
        "خط_وسط" => "line-through", "بدون_خط" => "none",
        "كتلة" => "block", "سطري" => "inline", "سطري_كتلة" => "inline-block",
        "مرن" => "flex", "شبكة" => "grid", "مخفي" => "none",
        "ثابت" => "fixed", "نسبي" => "relative",
        "مطلق_موضع" => "absolute", "لاصق" => "sticky",
        "تلقائي" => "auto",
        "مخفي_زائد" => "hidden", "تمرير" => "scroll",
        "مؤشر_يد" => "pointer", "نص_مؤشر" => "text",
        "سريع" => "0.2s", "بطيء" => "0.8s",
        "صف_مرن" => "row", "عمود_مرن" => "column",
        _ => {
            if val.parse::<f64>().is_ok() { return format!("{}px", val); }
            val
        }
    };
    out.to_string()
}

// ============================================
// مولد الكود
// ============================================

struct Codegen {
    counter: usize,
    events_js: String,
    effects_js: String,
    components: HashMap<String, (Vec<String>, Vec<Statement>)>,
    state_vars: HashSet<String>,
    derived_vars: HashSet<String>,
    depth: usize,
}

impl Codegen {
    fn new(state_vars: HashSet<String>, derived_vars: HashSet<String>) -> Self {
        Self {
            counter: 0,
            events_js: String::new(),
            effects_js: String::new(),
            components: HashMap::new(),
            state_vars,
            derived_vars,
            depth: 0,
        }
    }

    fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("r{}", self.counter)
    }

    fn eval_const(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::String(s) => Some(s.clone()),
            Expression::Number(n) => Some(if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }),
            Expression::Boolean(b) => Some(if *b { "صحيح" } else { "خطأ" }.into()),
            Expression::Null => Some(String::new()),
            _ => None,
        }
    }

    fn contains_only_reactive_safe(stmts: &[Statement]) -> bool {
        for st in stmts {
            match st {
                Statement::HtmlElement { children, .. } => {
                    if let Some(ref ch) = children {
                        if !Self::contains_only_reactive_safe(ch) { return false; }
                    }
                }
                Statement::If { then_branch, else_branch, .. } => {
                    if !Self::contains_only_reactive_safe(then_branch) { return false; }
                    if !Self::contains_only_reactive_safe(else_branch) { return false; }
                }
                Statement::ForEach { body, .. }
                | Statement::RangeFor { body, .. }
                | Statement::While { body, .. } => {
                    if !Self::contains_only_reactive_safe(body) { return false; }
                }
                Statement::Let { .. }
                | Statement::Const { .. }
                | Statement::Assignment { .. } => {}
                Statement::Call { name, .. } => {
                    if !Self::is_simple_builtin(name) { return false; }
                }
                _ => return false,
            }
        }
        true
    }

    fn is_simple_builtin(name: &str) -> bool {
        matches!(name,
            "زر_جميل" | "بطاقة"
            | "تنبيه_نجاح" | "تنبيه_خطأ" | "تنبيه_تحذير" | "تنبيه_معلومة"
            | "فقرة_مهمة" | "رأس_جميل" | "فاصل_جميل")
    }

    fn is_reactive(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(n) =>
                self.state_vars.contains(n) || self.derived_vars.contains(n),
            Expression::Binary { left, right, .. }
            | Expression::Comparison { left, right, .. }
            | Expression::Logical { left, right, .. } => {
                self.is_reactive(left) || self.is_reactive(right)
            }
            Expression::Not(e) | Expression::Neg(e) => self.is_reactive(e),
            Expression::List(items) => items.iter().any(|i| self.is_reactive(i)),
            Expression::Dict(pairs) => pairs.iter().any(|(_, v)| self.is_reactive(v)),
            Expression::MemberAccess { object, .. } => self.is_reactive(object),
            Expression::Index { object, index } => {
                self.is_reactive(object) || self.is_reactive(index)
            }
            Expression::Call { name, args } => {
                if name == "اقرأ_محلي" { return true; }
                args.iter().any(|a| self.is_reactive(a))
            }
            _ => false,
        }
    }

    fn expr_to_js(&self, expr: &Expression, locals: &HashSet<String>) -> String {
        match expr {
            Expression::String(s) => format!("\"{}\"", js_escape(s)),
            Expression::Number(n) => if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            },
            Expression::Boolean(b) => if *b { "true".into() } else { "false".into() },
            Expression::Null => "null".into(),
            Expression::Identifier(n) => {
                if locals.contains(n) {
                    n.clone()
                } else if self.state_vars.contains(n) || self.derived_vars.contains(n) {
                    format!("{}.get()", n)
                } else {
                    n.clone()
                }
            }
            Expression::List(items) => {
                let list: Vec<String> = items.iter().map(|i| self.expr_to_js(i, locals)).collect();
                format!("[{}]", list.join(", "))
            }
            Expression::Dict(pairs) => {
                let p: Vec<String> = pairs.iter()
                    .map(|(k, v)| format!("\"{}\": {}", js_escape(k), self.expr_to_js(v, locals)))
                    .collect();
                format!("{{{}}}", p.join(", "))
            }
            Expression::MemberAccess { object, property } =>
                format!("{}.{}", self.expr_to_js(object, locals), property),
            Expression::Index { object, index } =>
                format!("{}[{}]", self.expr_to_js(object, locals), self.expr_to_js(index, locals)),
            Expression::Call { name, args } => {
                if name == "اقرأ_مدخل" && args.is_empty() { return "this.value".to_string(); }
                let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x, locals)).collect();
                format!("{}({})", name, a.join(", "))
            }
            Expression::Binary { left, op, right } => {
                let op_str = match op {
                    BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
                    BinOp::Div => "/", BinOp::Mod => "%",
                };
                format!("({} {} {})",
                    self.expr_to_js(left, locals), op_str, self.expr_to_js(right, locals))
            }
            Expression::Comparison { left, op, right } => {
                let op_str = match op {
                    CmpOp::Eq => "===", CmpOp::Ne => "!==",
                    CmpOp::Gt => ">", CmpOp::Lt => "<",
                    CmpOp::Ge => ">=", CmpOp::Le => "<=",
                };
                format!("({} {} {})",
                    self.expr_to_js(left, locals), op_str, self.expr_to_js(right, locals))
            }
            Expression::Logical { left, op, right } => {
                let op_str = match op { LogOp::And => "&&", LogOp::Or => "||" };
                format!("({} {} {})",
                    self.expr_to_js(left, locals), op_str, self.expr_to_js(right, locals))
            }
            Expression::Not(e) => format!("(!{})", self.expr_to_js(e, locals)),
            Expression::Neg(e) => format!("(-{})", self.expr_to_js(e, locals)),
        }
    }

    fn gen_builtin_js(
        &self,
        name: &str,
        args: &[Expression],
        locals: &HashSet<String>,
    ) -> Option<String> {
        let arg = |i: usize| -> String {
            args.get(i)
                .map(|a| self.expr_to_js(a, locals))
                .unwrap_or_else(|| "\"\"".into())
        };
        let js = match name {
            "زر_جميل" => {
                let a = arg(0);
                format!("('<button class=\"rino-btn\">' + escape_html(String({a})) + '</button>')")
            }
            "بطاقة" => {
                let t = arg(0);
                let d = arg(1);
                let p = if args.len() > 2 {
                    format!(" + '<div class=\"rino-price\">' + escape_html(String({})) + '</div>'", arg(2))
                } else { String::new() };
                format!("('<div class=\"rino-card\"><h3>' + escape_html(String({t})) + '</h3><p>' + escape_html(String({d})) + '</p>' {} + '</div>')", p)
            }
            "تنبيه_نجاح" => format!("('<div class=\"rino-alert rino-success\">' + escape_html(String({})) + '</div>')", arg(0)),
            "تنبيه_خطأ" => format!("('<div class=\"rino-alert rino-error\">' + escape_html(String({})) + '</div>')", arg(0)),
            "تنبيه_تحذير" => format!("('<div class=\"rino-alert rino-warning\">' + escape_html(String({})) + '</div>')", arg(0)),
            "تنبيه_معلومة" => format!("('<div class=\"rino-alert rino-info\">' + escape_html(String({})) + '</div>')", arg(0)),
            "فقرة_مهمة" => format!("('<div class=\"rino-highlight\">' + escape_html(String({})) + '</div>')", arg(0)),
            "رأس_جميل" => format!("('<div class=\"rino-header\">' + escape_html(String({})) + '</div>')", arg(0)),
            "فاصل_جميل" => "('<hr class=\"rino-divider\">')".to_string(),
            _ => return None,
        };
        Some(js)
    }

    fn stmt_to_html_js(
        &self,
        stmt: &Statement,
        indent: &str,
        out_var: &str,
        locals: &HashSet<String>,
    ) -> Result<String, String> {
        let mut s = String::new();
        match stmt {
            Statement::HtmlElement { tag, content, attrs, children, events: _, .. } => {
                s.push_str(indent);
                s.push_str(out_var);
                s.push_str(" += '<");
                s.push_str(tag);

                for (k, v) in attrs {
                    if let Some(val) = self.eval_const(v) {
                        s.push_str(" ");
                        s.push_str(k);
                        s.push_str("=\"");
                        s.push_str(&html_attr_escape(&val));
                        s.push_str("\"");
                    } else {
                        let expr = self.expr_to_js(v, locals);
                        s.push_str("';\n");
                        s.push_str(indent);
                        s.push_str(out_var);
                        s.push_str(" += ' ");
                        s.push_str(k);
                        s.push_str("=\"' + escape_html(String(");
                        s.push_str(&expr);
                        s.push_str(")) + '");
                    }
                }
                if tag == "video" || tag == "audio" {
                    s.push_str(" controls");
                }
                s.push_str(">';\n");

                if let Some(c) = content {
                    if let Some(val) = self.eval_const(c) {
                        s.push_str(indent);
                        s.push_str(out_var);
                        s.push_str(" += '");
                        s.push_str(&js_escape(&val));
                        s.push_str("';\n");
                    } else {
                        let expr = self.expr_to_js(c, locals);
                        s.push_str(indent);
                        s.push_str(out_var);
                        s.push_str(" += escape_html(String(");
                        s.push_str(&expr);
                        s.push_str("));\n");
                    }
                }

                if let Some(ref ch) = children {
                    for c in ch {
                        s.push_str(&self.stmt_to_html_js(c, indent, out_var, locals)?);
                    }
                }

                if !is_self_closing_tag(tag) {
                    s.push_str(indent);
                    s.push_str(out_var);
                    s.push_str(" += '</");
                    s.push_str(tag);
                    s.push_str(">';\n");
                }
            }

            Statement::If { condition, then_branch, else_branch, .. } => {
                let cond_js = self.expr_to_js(condition, locals);
                s.push_str(indent);
                s.push_str("if (");
                s.push_str(&cond_js);
                s.push_str(") {\n");
                for st in then_branch {
                    s.push_str(&self.stmt_to_html_js(st, &format!("{}  ", indent), out_var, locals)?);
                }
                s.push_str(indent);
                s.push_str("}\n");
                if !else_branch.is_empty() {
                    s.push_str(indent);
                    s.push_str("else {\n");
                    for st in else_branch {
                        s.push_str(&self.stmt_to_html_js(st, &format!("{}  ", indent), out_var, locals)?);
                    }
                    s.push_str(indent);
                    s.push_str("}\n");
                }
            }

            Statement::ForEach { var, iterable, body, .. } => {
                let iter_js = self.expr_to_js(iterable, locals);
                let mut inner = locals.clone();
                inner.insert(var.clone());
                s.push_str(indent);
                s.push_str("for (const ");
                s.push_str(var);
                s.push_str(" of ");
                s.push_str(&iter_js);
                s.push_str(") {\n");
                for st in body {
                    s.push_str(&self.stmt_to_html_js(st, &format!("{}  ", indent), out_var, &inner)?);
                }
                s.push_str(indent);
                s.push_str("}\n");
            }

            Statement::RangeFor { var, start, end, step, body, .. } => {
                let start_js = self.expr_to_js(start, locals);
                let end_js = self.expr_to_js(end, locals);
                let step_js = step
                    .as_ref()
                    .map(|e| self.expr_to_js(e, locals))
                    .unwrap_or_else(|| "1".into());
                let mut inner = locals.clone();
                inner.insert(var.clone());
                s.push_str(indent);
                s.push_str("for (let ");
                s.push_str(var);
                s.push_str(" = ");
                s.push_str(&start_js);
                s.push_str("; ");
                s.push_str(var);
                s.push_str(" <= ");
                s.push_str(&end_js);
                s.push_str("; ");
                s.push_str(var);
                s.push_str(" += ");
                s.push_str(&step_js);
                s.push_str(") {\n");
                for st in body {
                    s.push_str(&self.stmt_to_html_js(st, &format!("{}  ", indent), out_var, &inner)?);
                }
                s.push_str(indent);
                s.push_str("}\n");
            }

            Statement::While { condition, body, .. } => {
                let cond_js = self.expr_to_js(condition, locals);
                s.push_str(indent);
                s.push_str("while (");
                s.push_str(&cond_js);
                s.push_str(") {\n");
                for st in body {
                    s.push_str(&self.stmt_to_html_js(st, &format!("{}  ", indent), out_var, locals)?);
                }
                s.push_str(indent);
                s.push_str("}\n");
            }

            Statement::Let { name, value, .. }
            | Statement::Const { name, value, .. }
            | Statement::Assignment { name, value, .. } => {
                let expr = self.expr_to_js(value, locals);
                s.push_str(indent);
                s.push_str("let ");
                s.push_str(name);
                s.push_str(" = ");
                s.push_str(&expr);
                s.push_str(";\n");
            }

            Statement::Call { name, args, .. } => {
                if let Some(expr) = self.gen_builtin_js(name, args, locals) {
                    s.push_str(indent);
                    s.push_str(out_var);
                    s.push_str(" += ");
                    s.push_str(&expr);
                    s.push_str(";\n");
                }
            }

            _ => {}
        }
        Ok(s)
    }

    fn stmt_to_js(&mut self, stmt: &Statement, indent: &str,
                  locals: &HashSet<String>) -> Result<String, String> {
        match stmt {
            Statement::Let { name, value, .. } =>
                Ok(format!("{}let {} = {};\n", indent, name, self.expr_to_js(value, locals))),
            Statement::Const { name, value, .. } =>
                Ok(format!("{}const {} = {};\n", indent, name, self.expr_to_js(value, locals))),
            Statement::Assignment { name, value, .. } => {
                let expr = self.expr_to_js(value, locals);
                if self.state_vars.contains(name) && !locals.contains(name) {
                    Ok(format!("{}batch(() => {}.set({}));\n", indent, name, expr))
                } else if self.derived_vars.contains(name) {
                    Err(format!("لا يمكن إسناد قيمة إلى مشتق \"{}\"", name))
                } else {
                    Ok(format!("{}{} = {};\n", indent, name, expr))
                }
            }
            Statement::Return { value, .. } => match value {
                Some(v) => Ok(format!("{}return {};\n", indent, self.expr_to_js(v, locals))),
                None => Ok(format!("{}return;\n", indent)),
            },
            Statement::If { condition, then_branch, else_branch, .. } => {
                let mut s = format!("{}if ({}) {{\n", indent, self.expr_to_js(condition, locals));
                for st in then_branch {
                    s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), locals)?);
                }
                s.push_str(&format!("{}}}", indent));
                if !else_branch.is_empty() {
                    s.push_str(" else {\n");
                    for st in else_branch {
                        s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), locals)?);
                    }
                    s.push_str(&format!("{}}}", indent));
                }
                s.push('\n');
                Ok(s)
            }
            Statement::ForEach { var, iterable, body, .. } => {
                let mut s = format!("{}for (let {} of {}) {{\n",
                    indent, var, self.expr_to_js(iterable, locals));
                let mut inner = locals.clone();
                inner.insert(var.clone());
                for st in body {
                    s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), &inner)?);
                }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            Statement::RangeFor { var, start, end, step, body, .. } => {
                let start_js = self.expr_to_js(start, locals);
                let end_js = self.expr_to_js(end, locals);
                let step_js = step.as_ref().map(|e| self.expr_to_js(e, locals))
                    .unwrap_or_else(|| "1".to_string());
                let mut inner = locals.clone();
                inner.insert(var.clone());
                let mut s = format!("{}for (let {} = {}; {} <= {}; {} += {}) {{\n",
                    indent, var, start_js, var, end_js, var, step_js);
                for st in body {
                    s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), &inner)?);
                }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            Statement::While { condition, body, .. } => {
                let mut s = format!("{}while ({}) {{\n", indent, self.expr_to_js(condition, locals));
                for st in body {
                    s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), locals)?);
                }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            Statement::Break { .. } => Ok(format!("{}break;\n", indent)),
            Statement::Continue { .. } => Ok(format!("{}continue;\n", indent)),
            Statement::TryCatch { try_body, catch_var, catch_body, .. } => {
                let mut s = format!("{}try {{\n", indent);
                for st in try_body {
                    s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), locals)?);
                }
                s.push_str(&format!("{}}} catch ({}) {{\n", indent, catch_var));
                for st in catch_body {
                    s.push_str(&self.stmt_to_js(st, &format!("{}  ", indent), locals)?);
                }
                s.push_str(&format!("{}}}\n", indent));
                Ok(s)
            }
            Statement::Call { name, args, .. } => {
                if name == "_skip_" { return Ok(String::new()); }
                if name == "اجلب" || name == "اجلب_نص" {
                    return Ok(self.gen_fetch_call(name, args, indent, locals));
                }
                let a: Vec<String> = args.iter().map(|x| self.expr_to_js(x, locals)).collect();
                if name == "اطبع" {
                    Ok(format!("{}console.log({});\n", indent, a.join(", ")))
                } else {
                    Ok(format!("{}{}({});\n", indent, name, a.join(", ")))
                }
            }
            _ => Ok(String::new()),
        }
    }

    fn gen_fetch_call(&self, name: &str, args: &[Expression], indent: &str,
                      locals: &HashSet<String>) -> String {
        if args.len() < 2 { return String::new(); }
        let url_js = self.expr_to_js(&args[0], locals);
        let var_name = match &args[1] {
            Expression::String(s) => s.clone(),
            Expression::Identifier(n) => n.clone(),
            _ => return String::new(),
        };
        let target = if self.state_vars.contains(&var_name) {
            format!("{}.set", var_name)
        } else {
            return String::new();
        };
        let field_name = if args.len() >= 3 {
            match &args[2] { Expression::String(s) => Some(s.clone()), _ => None }
        } else { None };

        let mut out = String::new();
        out.push_str(indent);
        out.push_str("fetch(");
        out.push_str(&url_js);
        out.push_str(")\n");
        if name == "اجلب" {
            out.push_str(indent);
            out.push_str("  .then(function(r) { return r.json(); })\n");
            out.push_str(indent);
            if let Some(field) = field_name {
                out.push_str("  .then(function(d) { ");
                out.push_str(&target);
                out.push_str("((typeof d.");
                out.push_str(&field);
                out.push_str(" === 'string') ? d.");
                out.push_str(&field);
                out.push_str(" : JSON.stringify(d.");
                out.push_str(&field);
                out.push_str(")); })\n");
            } else {
                out.push_str("  .then(function(d) { let _v = d; if (typeof d === 'object' && d !== null) { const keys = Object.keys(d); for (const k of keys) { if (typeof d[k] === 'string') { _v = d[k]; break; } } _v = JSON.stringify(_v); } ");
                out.push_str(&target);
                out.push_str("(String(_v)); })\n");
            }
            out.push_str(indent);
            out.push_str("  .catch(function(e) { ");
            out.push_str(&target);
            out.push_str("('خطأ: ' + e.message); });\n");
        } else {
            out.push_str(indent);
            out.push_str("  .then(function(r) { return r.text(); })\n");
            out.push_str(indent);
            out.push_str("  .then(function(d) { ");
            out.push_str(&target);
            out.push_str("(d); })\n");
            out.push_str(indent);
            out.push_str("  .catch(function(e) { ");
            out.push_str(&target);
            out.push_str("('خطأ: ' + e.message); });\n");
        }
        out
    }

    fn gen_builtin(
        &self,
        name: &str,
        args: &[Expression],
        env: &HashMap<String, Value>,
        interp: &mut Interp,
    ) -> Result<Option<String>, String> {
        let get_str = |i: usize, interp: &mut Interp| -> Result<String, String> {
            match args.get(i) {
                Some(a) => Ok(interp.eval(a, env)?.to_display()),
                None => Ok(String::new()),
            }
        };
        let get_num = |i: usize, interp: &mut Interp| -> Result<f64, String> {
            match args.get(i) {
                Some(a) => interp.eval(a, env).and_then(|v| v.as_num()),
                None => Ok(0.0),
            }
        };
        let html = match name {
            "زر_جميل" => Some(format!("<button class=\"rino-btn\">{}</button>", get_str(0, interp)?)),
            "بطاقة" => {
                let title = get_str(0, interp)?;
                let desc = get_str(1, interp)?;
                let price = if args.len() > 2 {
                    format!("<div class=\"rino-price\">{}</div>", get_str(2, interp)?)
                } else { String::new() };
                Some(format!("<div class=\"rino-card\"><h3>{}</h3><p>{}</p>{}</div>",
                    title, desc, price))
            }
            "تنبيه_نجاح" => Some(format!("<div class=\"rino-alert rino-success\">{}</div>", get_str(0, interp)?)),
            "تنبيه_خطأ" => Some(format!("<div class=\"rino-alert rino-error\">{}</div>", get_str(0, interp)?)),
            "تنبيه_تحذير" => Some(format!("<div class=\"rino-alert rino-warning\">{}</div>", get_str(0, interp)?)),
            "تنبيه_معلومة" => Some(format!("<div class=\"rino-alert rino-info\">{}</div>", get_str(0, interp)?)),
            "شريط_تقدم" => {
                let pct = get_num(0, interp)?.clamp(0.0, 100.0);
                Some(format!("<div class=\"rino-progress\"><div class=\"rino-progress-bar\" style=\"width: {}%\">{:.0}%</div></div>", pct, pct))
            }
            "فقرة_مهمة" => Some(format!("<div class=\"rino-highlight\">{}</div>", get_str(0, interp)?)),
            "رأس_جميل" => Some(format!("<div class=\"rino-header\">{}</div>", get_str(0, interp)?)),
            "فاصل_جميل" => Some("<hr class=\"rino-divider\">".to_string()),
            _ => None,
        };
        Ok(html)
    }

    fn gen_html(
        &mut self,
        stmt: &Statement,
        env: &mut HashMap<String, Value>,
        interp: &mut Interp,
    ) -> Result<String, String> {
        match stmt {
            Statement::HtmlElement { tag, content, attrs, children, events, .. } => {
                let mut attr_str = String::new();
                let mut existing_id: Option<String> = None;
                for (k, v) in attrs {
                    let val = interp.eval(v, env)?.to_display();
                    if k == "id" { existing_id = Some(val.clone()); }
                    attr_str.push_str(" ");
                    attr_str.push_str(k);
                    attr_str.push_str("=\"");
                    attr_str.push_str(&html_attr_escape(&val));
                    attr_str.push('"');
                }

                if tag == "video" || tag == "audio" {
                    attr_str.push_str(" controls");
                }

                // ربط ثنائي الاتجاه
                if matches!(tag.as_str(), "input" | "textarea") {
                    if let Some(ref id_val) = existing_id {
                        if self.state_vars.contains(id_val) {
                            let mut listener = String::new();
                            listener.push_str("(function() { const _el = document.getElementById('");
                            listener.push_str(id_val);
                            listener.push_str("'); if (_el) _el.addEventListener('input', function() { ");
                            listener.push_str(id_val);
                            listener.push_str(".set(this.value); }); })();\n");
                            self.events_js.push_str(&listener);
                        }
                    }
                }

                let needs_reactive = content.as_ref()
                    .map(|c| self.is_reactive(c)).unwrap_or(false);
                let needs_id = needs_reactive || !events.is_empty();
                let id = if needs_id && existing_id.is_none() {
                    Some(self.next_id())
                } else {
                    existing_id.clone()
                };

                if existing_id.is_none() {
                    if let Some(ref i) = id {
                        attr_str.push_str(" id=\"");
                        attr_str.push_str(i);
                        attr_str.push('"');
                    }
                }

                let (open, close, self_closing) = match tag.as_str() {
                    "img"|"input"|"br"|"hr"|"col"|"source"|"track"|"area"|"wbr" =>
                        (format!("<{}{}>", tag, attr_str), String::new(), true),
                    _ => (format!("<{}{}>", tag, attr_str), format!("</{}>", tag), false),
                };

                let inner = if self_closing { String::new() }
                else if let Some(ref ch) = children {
                    let mut s = String::new();
                    for c in ch {
                        s.push_str(&self.gen_html(c, env, interp)?);
                    }
                    s
                } else if let Some(c) = content {
                    if needs_reactive {
                        if let Some(ref i) = id {
                            let js = self.expr_to_js(c, &HashSet::new());
                            let mut eff = String::new();
                            eff.push_str("createEffect(() => {\n");
                            eff.push_str("  const _el = document.getElementById('");
                            eff.push_str(i);
                            eff.push_str("');\n");
                            eff.push_str("  if (_el) _el.textContent = ");
                            eff.push_str(&js);
                            eff.push_str(";\n");
                            eff.push_str("});\n");
                            self.effects_js.push_str(&eff);
                        }
                    }
                    interp.eval(c, env)?.to_display()
                } else {
                    String::new()
                };

                for ev in events {
                    if let Some(ref i) = id {
                        let mut body_js = String::new();
                        for s in &ev.body {
                            body_js.push_str(&self.stmt_to_js(s, "  ", &HashSet::new())?);
                        }
                        let mut ev_js = String::new();
                        ev_js.push_str("document.getElementById('");
                        ev_js.push_str(i);
                        ev_js.push_str("').addEventListener('");
                        ev_js.push_str(&ev.kind);
                        ev_js.push_str("', function() {\n");
                        ev_js.push_str(&body_js);
                        ev_js.push_str("});\n");
                        self.events_js.push_str(&ev_js);
                    }
                }
                Ok(format!("{}{}{}", open, inner, close))
            }

            Statement::If { condition, then_branch, else_branch, .. } => {
                let cond_reactive = self.is_reactive(condition);
                let body_safe = Self::contains_only_reactive_safe(then_branch)
                    && Self::contains_only_reactive_safe(else_branch);

                if cond_reactive && body_safe {
                    let id = self.next_id();

                    // المحتوى الابتدائي
                    let initial_true = interp.eval(condition, env)?.as_bool();
                    let initial_branch = if initial_true { then_branch } else { else_branch };
                    let mut initial_html = String::new();
                    for st in initial_branch {
                        initial_html.push_str(&self.gen_html(st, env, interp)?);
                    }

                    // رندر JS
                    let mut then_render = String::new();
                    for st in then_branch {
                        then_render.push_str(&self.stmt_to_html_js(
                            st, "    ", "_out", &HashSet::new())?);
                    }
                    let mut else_render = String::new();
                    for st in else_branch {
                        else_render.push_str(&self.stmt_to_html_js(
                            st, "    ", "_out", &HashSet::new())?);
                    }

                    let cond_js = self.expr_to_js(condition, &HashSet::new());

                    let mut eff = String::new();
                    eff.push_str("createEffect(() => {\n");
                    eff.push_str("  let _out = '';\n");
                    eff.push_str("  if (");
                    eff.push_str(&cond_js);
                    eff.push_str(") {\n");
                    eff.push_str(&then_render);
                    eff.push_str("  } else {\n");
                    eff.push_str(&else_render);
                    eff.push_str("  }\n");
                    eff.push_str("  const _e = document.getElementById('");
                    eff.push_str(&id);
                    eff.push_str("');\n");
                    eff.push_str("  if (_e) _e.innerHTML = _out;\n");
                    eff.push_str("});\n");
                    self.effects_js.push_str(&eff);

                    Ok(format!("<div id=\"{}\">{}</div>", id, initial_html))
                } else {
                    let cond_val = interp.eval(condition, env)?.as_bool();
                    let branch = if cond_val { then_branch } else { else_branch };
                    let mut s = String::new();
                    for st in branch {
                        s.push_str(&self.gen_html(st, env, interp).unwrap_or_default());
                    }
                    Ok(s)
                }
            }

            Statement::ForEach { var, iterable, body, .. } => {
                let iter_reactive = self.is_reactive(iterable);
                let body_safe = Self::contains_only_reactive_safe(body);

                if iter_reactive && body_safe {
                    let id = self.next_id();

                    // محتوى ابتدائي
                    let iterable_val = interp.eval(iterable, env)?;
                    let initial_items: Vec<Value> = match iterable_val {
                        Value::List(l) => l,
                        Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                        Value::Dict(p) => p.into_iter().map(|(k, _)| Value::Str(k)).collect(),
                        _ => vec![],
                    };
                    let mut initial_html = String::new();
                    let cached_env = env.clone();
                    for item in initial_items {
                        env.insert(var.clone(), item);
                        for st in body {
                            initial_html.push_str(&self.gen_html(st, env, interp).unwrap_or_default());
                        }
                    }
                    *env = cached_env;

                    // رندر JS
                    let mut locals: HashSet<String> = HashSet::new();
                    locals.insert(var.clone());
                    let mut render = String::new();
                    for st in body {
                        render.push_str(&self.stmt_to_html_js(st, "    ", "_out", &locals)?);
                    }

                    let src_js = self.expr_to_js(iterable, &HashSet::new());

                    let mut eff = String::new();
                    eff.push_str("createEffect(() => {\n");
                    eff.push_str("  let _out = '';\n");
                    eff.push_str("  for (const ");
                    eff.push_str(var);
                    eff.push_str(" of ");
                    eff.push_str(&src_js);
                    eff.push_str(") {\n");
                    eff.push_str(&render);
                    eff.push_str("  }\n");
                    eff.push_str("  const _e = document.getElementById('");
                    eff.push_str(&id);
                    eff.push_str("');\n");
                    eff.push_str("  if (_e) _e.innerHTML = _out;\n");
                    eff.push_str("});\n");
                    self.effects_js.push_str(&eff);

                    Ok(format!("<div id=\"{}\">{}</div>", id, initial_html))
                } else {
                    let iterable_val = interp.eval(iterable, env)?;
                    let items: Vec<Value> = match iterable_val {
                        Value::List(l) => l,
                        Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                        Value::Dict(p) => p.into_iter().map(|(k, _)| Value::Str(k)).collect(),
                        _ => return Ok(String::new()),
                    };
                    let mut s = String::new();
                    let old_env = env.clone();
                    for item in items {
                        env.insert(var.clone(), item);
                        for st in body {
                            s.push_str(&self.gen_html(st, env, interp).unwrap_or_default());
                        }
                    }
                    *env = old_env;
                    Ok(s)
                }
            }

            Statement::RangeFor { var, start, end, step, body, .. } => {
                let start_val = interp.eval(start, env)?.as_num()? as i64;
                let end_val = interp.eval(end, env)?.as_num()? as i64;
                let step_val = if let Some(s) = step {
                    let sv = interp.eval(s, env)?.as_num()? as i64;
                    if sv == 0 { 1 } else { sv }
                } else { 1 };
                let mut s = String::new();
                let old_env = env.clone();
                let mut i = start_val;
                let mut guard = 0;
                while (step_val > 0 && i <= end_val) || (step_val < 0 && i >= end_val) {
                    guard += 1;
                    if guard > 100_000 { break; }
                    env.insert(var.clone(), Value::Num(i as f64));
                    for st in body {
                        s.push_str(&self.gen_html(st, env, interp).unwrap_or_default());
                    }
                    i += step_val;
                }
                *env = old_env;
                Ok(s)
            }

            Statement::While { condition, body, .. } => {
                let mut s = String::new();
                let old_env = env.clone();
                let mut guard = 0;
                while interp.eval(condition, env)?.as_bool() {
                    guard += 1;
                    if guard > 100_000 { break; }
                    for st in body {
                        s.push_str(&self.gen_html(st, env, interp).unwrap_or_default());
                    }
                }
                *env = old_env;
                Ok(s)
            }

            Statement::TryCatch { try_body, catch_var, catch_body, .. } => {
                let mut s = String::new();
                let mut error_occurred = false;
                let cached_env = env.clone();
                for st in try_body {
                    match self.gen_html(st, env, interp) {
                        Ok(html) => s.push_str(&html),
                        Err(_) => { error_occurred = true; break; }
                    }
                }
                if error_occurred {
                    s.clear();
                    *env = cached_env.clone();
                    env.insert(catch_var.clone(), Value::Str("خطأ".into()));
                    for st in catch_body {
                        s.push_str(&self.gen_html(st, env, interp).unwrap_or_default());
                    }
                    *env = cached_env;
                }
                Ok(s)
            }

            Statement::Call { name, args, .. } => {
                if name == "_skip_" { return Ok(String::new()); }

                if let Some(html) = self.gen_builtin(name, args, env, interp)? {
                    return Ok(html);
                }

                if let Some((params, body)) = self.components.get(name).cloned() {
                    if self.depth > 30 { return Err("استدعاء متكرر لا نهائي".into()); }
                    let mut new_env: HashMap<String, Value> = HashMap::new();
                    for (i, p) in params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            let v = interp.eval(arg, env)?;
                            new_env.insert(p.clone(), v);
                        }
                    }
                    self.depth += 1;
                    let mut html = String::new();
                    for stmt in &body {
                        html.push_str(&self.gen_html(stmt, &mut new_env, interp)?);
                    }
                    self.depth -= 1;
                    return Ok(html);
                }

                if name == "احفظ" {
                    let a: Vec<String> = args.iter()
                        .map(|x| self.expr_to_js(x, &HashSet::new())).collect();
                    self.events_js.push_str("احفظ(");
                    self.events_js.push_str(&a.join(", "));
                    self.events_js.push_str(");\n");
                    return Ok(String::new());
                }
                if name == "اجلب" || name == "اجلب_نص" {
                    let code = self.gen_fetch_call(name, args, "  ", &HashSet::new());
                    self.events_js.push_str(&code);
                    return Ok(String::new());
                }

                let expr_call = Expression::Call { name: name.clone(), args: args.clone() };
                match interp.eval(&expr_call, env) {
                    Ok(v) => Ok(v.to_display()),
                    Err(_) => Ok(String::new()),
                }
            }

            Statement::Let { name, value, .. }
            | Statement::Const { name, value, .. }
            | Statement::Assignment { name, value, .. } => {
                let v = interp.eval(value, env)?;
                env.insert(name.clone(), v);
                Ok(String::new())
            }

            _ => Ok(String::new()),
        }
    }
}

// ============================================
// مراجعة الكود
// ============================================

fn review_code(source: &str, input_path: &str) -> i32 {
    println!("\n=====================================");
    println!("  Rino Review - مراجعة الكود");
    println!("=====================================\n");
    println!("الملف: {}\n", input_path);

    let mut errors: Vec<(usize, String, String)> = Vec::new();
    let mut warnings: Vec<(usize, String)> = Vec::new();
    let mut info: Vec<String> = Vec::new();

    let total_lines = source.lines().count();
    let mut empty_lines = 0;
    let mut comment_lines = 0;
    let mut code_lines = 0;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { empty_lines += 1; }
        else if trimmed.starts_with("//") { comment_lines += 1; }
        else { code_lines += 1; }
    }
    info.push(format!("اجمالي الاسطر: {}", total_lines));
    info.push(format!("اسطر الكود: {}", code_lines));
    info.push(format!("اسطر التعليقات: {}", comment_lines));
    info.push(format!("اسطر فارغة: {}", empty_lines));

    let mut lexer = Lexer::new(source);
    match lexer.tokenize() {
        Ok(_) => info.push("التحليل اللغوي: ناجح".to_string()),
        Err(e) => errors.push((0, "لغوي".to_string(), e)),
    }
    for c in &lexer.corrections { warnings.push((0, c.clone())); }

    let mut lexer2 = Lexer::new(source);
    if let Ok(tokens) = lexer2.tokenize() {
        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(program) => {
                info.push("التحليل النحوي: ناجح".to_string());
                if program.page_title.is_none() {
                    warnings.push((0, "لم يتم تحديد عنوان الصفحة".to_string()));
                }
                info.push(format!("قواعد النمط: {}", program.styles.len()));
                info.push(format!("متغيرات الحالة: {}", program.state.len()));
                info.push(format!("المشتقات: {}", program.derived.len()));
                info.push(format!("الدوال: {}", program.functions.len()));
                info.push(format!("المكونات: {}", program.components.len()));
                info.push(format!("عناصر الصفحة: {}", program.body.len()));
            }
            Err(e) => errors.push((0, "نحوي".to_string(), e)),
        }
    }

    println!("الإحصاءات:");
    for i in &info { println!("   {}", i); }
    println!();

    if !errors.is_empty() {
        println!("الاخطاء ({}):", errors.len());
        for (line, kind, msg) in &errors {
            if *line > 0 { println!("   [سطر {}] [{}] {}", line, kind, msg); }
            else { println!("   [{}] {}", kind, msg); }
        }
        println!();
    }
    if !warnings.is_empty() {
        println!("التحذيرات ({}):", warnings.len());
        for (line, msg) in &warnings {
            if *line > 0 { println!("   [سطر {}] {}", line, msg); }
            else { println!("   {}", msg); }
        }
        println!();
    }

    if errors.is_empty() { 0 } else { 1 }
}

// ============================================
// main
// ============================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 2 && args[1] == "review" {
        let file = &args[2];
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => { eprintln!("فشل قراءة الملف: {}", e); std::process::exit(1); }
        };
        let code = review_code(&source, file);
        std::process::exit(code);
    }

    if args.len() > 2 && args[1] == "format" {
        let file_path = PathBuf::from(&args[2]);
        let source = match fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(e) => { eprintln!("فشل قراءة الملف: {}", e); std::process::exit(1); }
        };
        let mut fmt = formatter::Formatter::new();
        let formatted = fmt.format(&source);
        if let Err(e) = fs::write(&file_path, &formatted) {
            eprintln!("فشل الحفظ: {}", e);
            std::process::exit(1);
        }
        println!("تم تنسيق: {}", file_path.display());
        return;
    }

    if args.len() > 1 && args[1] == "test" {
        let test_file = if args.len() > 2 { &args[2] } else { "index.rino" };
        let test_path = PathBuf::from(test_file);
        if !test_path.exists() {
            eprintln!("الملف غير موجود: {}", test_file);
            std::process::exit(1);
        }
        let src = fs::read_to_string(&test_path).expect("فشل قراءة الملف");
        let mut v = HashSet::new();
        if let Ok(c) = test_path.canonicalize() { v.insert(c); }
        let dir = test_path.parent().unwrap_or(Path::new("."));
        let src = process_imports(&src, dir, &mut v, 0).expect("فشل معالجة الاستيرادات");
        let src = preprocess_indentation(&src);

        let mut lx = Lexer::new(&src);
        let tk = lx.tokenize().expect("خطأ لغوي");
        if !lx.corrections.is_empty() {
            println!("تصحيحات:");
            for c in &lx.corrections { println!("  {}", c); }
        }
        let mut pr = Parser::new(tk);
        let prog = pr.parse().expect("خطأ نحوي");

        let exit_code = run_tests(&prog);
        std::process::exit(exit_code);
    }

    let input_path: PathBuf = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("index.rino")
    };
    if !input_path.exists() {
        eprintln!("الملف غير موجود: {}", input_path.display());
        eprintln!("\nالاستخدام:");
        eprintln!("   rino <ملف.rino>         — بناء الملف");
        eprintln!("   rino format <ملف.rino>  — تنسيق الملف");
        eprintln!("   rino test <ملف.rino>    — تشغيل الاختبارات");
        eprintln!("   rino review <ملف.rino>  — مراجعة الكود");
        std::process::exit(1);
    }

    let input_dir = input_path.parent()
        .map(|p| if p.as_os_str().is_empty() { PathBuf::from(".") } else { p.to_path_buf() })
        .unwrap_or_else(|| PathBuf::from("."));
    let base_name = input_path.file_stem()
        .and_then(|s| s.to_str()).unwrap_or("index").to_string();

    let output_html_path = input_dir.join(format!("{}.html", base_name));
    let output_css_path  = input_dir.join(format!("{}.css", base_name));
    let output_js_path   = input_dir.join(format!("{}.js", base_name));

    let css_filename = format!("{}.css", base_name);
    let js_filename  = format!("{}.js", base_name);

    println!("قراءة: {}", input_path.display());

    let source = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    };

    let mut visited = HashSet::new();
    if let Ok(canonical) = input_path.canonicalize() { visited.insert(canonical); }
    let source = match process_imports(&source, &input_dir, &mut visited, 0) {
        Ok(s) => s,
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    };
    let source = preprocess_indentation(&source);

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { eprintln!("خطأ لغوي: {}", e); std::process::exit(1); }
    };
    if !lexer.corrections.is_empty() {
        println!("تصحيحات:");
        for c in &lexer.corrections { println!("  {}", c); }
    }

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => { eprintln!("خطأ نحوي: {}", e); std::process::exit(1); }
    };

    // خريطة الدوال
    let mut funcs: FuncMap = HashMap::new();
    for f in &program.functions {
        if let Statement::Function { name, params, body, .. } = f {
            funcs.insert(name.clone(), (params.clone(), body.clone()));
        }
    }

    // متغيرات الحالة والمشتقات
    let mut state_vars: HashSet<String> = HashSet::new();
    for s in &program.state {
        if let Statement::Let { name, .. } = s { state_vars.insert(name.clone()); }
    }
    let mut derived_vars: HashSet<String> = HashSet::new();
    for s in &program.derived {
        if let Statement::Let { name, .. } = s { derived_vars.insert(name.clone()); }
    }

    // بيئة التوليد
    let mut env: HashMap<String, Value> = HashMap::new();
    {
        let mut interp = Interp::new(&funcs);
        for s in &program.state { let _ = interp.exec(s, &mut env); }
        for s in &program.body {
            if matches!(s,
                Statement::Let { .. } |
                Statement::Const { .. } |
                Statement::Assignment { .. }) {
                let _ = interp.exec(s, &mut env);
            }
        }
    }

    // توليد دوال JS
    let mut cg = Codegen::new(state_vars.clone(), derived_vars.clone());
    for c in &program.components {
        if let Statement::ComponentDef { name, params, body, .. } = c {
            cg.components.insert(name.clone(), (params.clone(), body.clone()));
        }
    }

    let mut funcs_js = String::new();
    for f in &program.functions {
        if let Statement::Function { name, params, body, .. } = f {
            let locals: HashSet<String> = params.iter().cloned().collect();
            let mut inner = String::new();
            for s in body {
                inner.push_str(&cg.stmt_to_js(s, "  ", &locals).unwrap_or_default());
            }
            funcs_js.push_str(&format!(
                "function {}({}) {{\n{}}}\n",
                name, params.join(", "), inner));
        }
    }

    // توليد body_html (بعد بناء cg مكتملاً)
    let mut body_html = String::new();
    {
        let mut interp = Interp::new(&funcs);
        for s in &program.body {
            match cg.gen_html(s, &mut env, &mut interp) {
                Ok(h) => body_html.push_str(&h),
                Err(e) => {
                    eprintln!("خطأ في توليد HTML: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    // CSS
    let mut css = String::from(BUILTIN_CSS);
    css.push('\n');
    for r in &program.styles {
        let prefix = match r.selector_kind {
            SelectorKind::Tag => "",
            SelectorKind::Class => ".",
            SelectorKind::Id => "#",
        };
        css.push_str(prefix);
        css.push_str(&r.selector);
        css.push_str(" {\n");
        for (p, v) in &r.properties {
            css.push_str("  ");
            css.push_str(css_property(p));
            css.push_str(": ");
            css.push_str(&css_value(v));
            css.push_str(";\n");
        }
        css.push_str("}\n");
        if !r.hover_properties.is_empty() {
            css.push_str(prefix);
            css.push_str(&r.selector);
            css.push_str(":hover {\n");
            for (p, v) in &r.hover_properties {
                css.push_str("  ");
                css.push_str(css_property(p));
                css.push_str(": ");
                css.push_str(&css_value(v));
                css.push_str(";\n");
            }
            css.push_str("}\n");
        }
    }

    // توليد الإشارات الأولية
    let mut state_init = String::new();
    for s in &program.state {
        if let Statement::Let { name, value, .. } = s {
            let mut interp = Interp::new(&funcs);
            let v = interp.eval(value, &env).unwrap_or(Value::Null);
            state_init.push_str("const ");
            state_init.push_str(name);
            state_init.push_str(" = signal(");
            state_init.push_str(&v.to_js_literal());
            state_init.push_str(");\n");
        }
    }

    // توليد المشتقات (memos)
    let mut derived_js = String::new();
    for s in &program.derived {
        if let Statement::Let { name, value, .. } = s {
            let js = cg.expr_to_js(value, &HashSet::new());
            derived_js.push_str("const ");
            derived_js.push_str(name);
            derived_js.push_str(" = createMemo(() => ");
            derived_js.push_str(&js);
            derived_js.push_str(");\n");
        }
    }

    let title = match &program.page_title {
        Some(e) => {
            let mut interp = Interp::new(&funcs);
            interp.eval(e, &env).map(|v| v.to_display())
                .unwrap_or_else(|_| "Rino Page".into())
        }
        None => "Rino Page".into(),
    };

    // بناء السكربت النهائي
    let mut script = String::new();
    script.push_str(SIGNALS_JS);
    script.push('\n');
    script.push_str(HELPERS_JS);
    script.push('\n');
    script.push_str(&state_init);
    script.push('\n');
    script.push_str(&derived_js);
    script.push('\n');
    script.push_str(&cg.events_js);
    script.push('\n');
    script.push_str(&cg.effects_js);

    // HTML
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"ar\" dir=\"rtl\">\n");
    html.push_str("<head>\n");
    html.push_str("  <meta charset=\"UTF-8\">\n");
    html.push_str("  <title>");
    html.push_str(&title);
    html.push_str("</title>\n");
    html.push_str("  <link rel=\"stylesheet\" href=\"");
    html.push_str(&css_filename);
    html.push_str("\">\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str(&body_html);
    html.push_str("  <script src=\"");
    html.push_str(&js_filename);
    html.push_str("\"></script>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    let js_content = format!("{}{}", funcs_js, script);

    if let Err(e) = fs::write(&output_html_path, &html) {
        eprintln!("فشل كتابة {}: {}", output_html_path.display(), e);
        std::process::exit(1);
    }
    if let Err(e) = fs::write(&output_css_path, &css) {
        eprintln!("فشل كتابة {}: {}", output_css_path.display(), e);
        std::process::exit(1);
    }
    if let Err(e) = fs::write(&output_js_path, &js_content) {
        eprintln!("فشل كتابة {}: {}", output_js_path.display(), e);
        std::process::exit(1);
    }

    println!("\nتم التوليد:");
    println!("   {}", output_html_path.display());
    println!("   {}", output_css_path.display());
    println!("   {}", output_js_path.display());
}