//! مدقّق الأنواع — يكتشف الأخطاء قبل التوليد.

use crate::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct TypeError {
    pub line: usize,
    pub message: String,
}

pub struct TypeChecker {
    aliases: HashMap<String, Type>,
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, (Vec<Type>, Type)>,
    pub errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            scopes: vec![HashMap::new()],
            funcs: HashMap::new(),
            errors: Vec::new(),
        }
    }

    // ==========================================
    // المساعدات
    // ==========================================

    fn resolve(&self, t: &Type) -> Type {
        match t {
            Type::Named(n) => match self.aliases.get(n) {
                Some(d) => self.resolve(d),
                None => t.clone(),
            },
            Type::List(inner) => Type::List(Box::new(self.resolve(inner))),
            Type::Optional(inner) => Type::Optional(Box::new(self.resolve(inner))),
            Type::Dict(pairs) => Type::Dict(
                pairs.iter().map(|(k, v)| (k.clone(), self.resolve(v))).collect()
            ),
            _ => t.clone(),
        }
    }

    fn compatible(&self, actual: &Type, expected: &Type) -> bool {
        let a = self.resolve(actual);
        let e = self.resolve(expected);

        // Any يتوافق مع أي شيء
        if matches!(a, Type::Any) || matches!(e, Type::Any) {
            return true;
        }

        match (&a, &e) {
            // Optional<X> مع Optional<Y> → تحقق من X, Y
            (Type::Optional(ia), Type::Optional(ie)) => self.compatible(ia, ie),

            // قيمة عادية مع Optional → تحقق من القيمة مع الداخل
            (Type::Optional(ia), o) => self.compatible(ia, o),
            (o, Type::Optional(ie)) => {
                // لا_شيء متوافق مع أي Optional
                if matches!(o, Type::Nothing) {
                    return true;
                }
                self.compatible(o, ie)
            }

            // قائمة<X> مع قائمة<Y>
            (Type::List(ia), Type::List(ie)) => self.compatible(ia, ie),

            // قاموس مع قاموس
            (Type::Dict(pa), Type::Dict(pe)) => {
                if pa.len() != pe.len() {
                    return false;
                }
                pa.iter().zip(pe.iter()).all(|((ka, va), (ke, ve))| {
                    ka == ke && self.compatible(va, ve)
                })
            }

            // الأنواع البسيطة: يجب أن تكون متطابقة
            _ => std::mem::discriminant(&a) == std::mem::discriminant(&e),
        }
    }

    fn declare(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn error(&mut self, line: usize, msg: String) {
        self.errors.push(TypeError { line, message: msg });
    }

    // ==========================================
    // نقطة الدخول
    // ==========================================

    pub fn check_program(&mut self, prog: &Program) -> Vec<TypeError> {
        // 1) سجّل الأنواع
        for alias in &prog.type_aliases {
            if self.aliases.contains_key(&alias.name) {
                self.error(alias.line, format!("النوع \"{}\" مُعرّف مسبقًا", alias.name));
                continue;
            }
            self.aliases.insert(alias.name.clone(), alias.definition.clone());
        }

        // 2) كشف الدورات المباشرة
        let names: Vec<String> = self.aliases.keys().cloned().collect();
        let mut cyclic: HashSet<String> = HashSet::new();
        for name in &names {
            if self.detect_direct_cycle(name) {
                cyclic.insert(name.clone());
            }
        }
        for name in &cyclic {
            self.error(0, format!("النوع \"{}\" مُعرّف دائريًا", name));
            self.aliases.remove(name);
        }

        // 3) سجّل الدوال
        for stmt in &prog.functions {
            if let Statement::Function { name, param_types, return_type, .. } = stmt {
                let args: Vec<Type> = param_types.iter()
                    .map(|t| t.clone().unwrap_or(Type::Any))
                    .collect();
                let ret = return_type.clone().unwrap_or(Type::Any);
                self.funcs.insert(name.clone(), (args, ret));
            }
        }

        // 4) افحص كل شيء
        for stmt in &prog.state { self.check_stmt(stmt); }
        for stmt in &prog.derived { self.check_stmt(stmt); }
        for stmt in &prog.functions { self.check_stmt(stmt); }
        for stmt in &prog.components { self.check_stmt(stmt); }
        for stmt in &prog.body { self.check_stmt(stmt); }
        for stmt in &prog.tests { self.check_stmt(stmt); }

        std::mem::take(&mut self.errors)
    }

    fn detect_direct_cycle(&self, start: &str) -> bool {
        let mut current = start.to_string();
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(current.clone()) {
                return true;
            }
            match self.aliases.get(&current) {
                Some(Type::Named(next)) => current = next.clone(),
                _ => return false,
            }
        }
    }

    // ==========================================
    // الجمل
    // ==========================================

    fn check_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, value, type_ann, line, .. } => {
                let inferred = self.infer_expr(value);
                let final_ty = if let Some(ann) = type_ann {
                    let ann_resolved = self.resolve(ann);
                    if !self.compatible(&inferred, &ann_resolved) {
                        self.error(*line, format!(
                            "لا يمكن إسناد \"{}\" إلى متغير من نوع \"{}\"",
                            inferred.display_ar(), ann_resolved.display_ar()
                        ));
                    }
                    ann_resolved
                } else {
                    inferred
                };
                self.declare(name, final_ty);
            }
            Statement::Const { name, value, type_ann, line } => {
                let inferred = self.infer_expr(value);
                let final_ty = if let Some(ann) = type_ann {
                    let ann_resolved = self.resolve(ann);
                    if !self.compatible(&inferred, &ann_resolved) {
                        self.error(*line, format!(
                            "لا يمكن إسناد \"{}\" إلى ثابت من نوع \"{}\"",
                            inferred.display_ar(), ann_resolved.display_ar()
                        ));
                    }
                    ann_resolved
                } else {
                    inferred
                };
                self.declare(name, final_ty);
            }
            Statement::Assignment { name, value, line } => {
                let inferred = self.infer_expr(value);
                if let Some(existing) = self.lookup(name) {
                    if !self.compatible(&inferred, &existing) {
                        self.error(*line, format!(
                            "لا يمكن إسناد \"{}\" إلى \"{}\" من نوع \"{}\"",
                            inferred.display_ar(), name, existing.display_ar()
                        ));
                    }
                }
            }
            Statement::Function { params, param_types, return_type, body, line, .. } => {
                self.push_scope();
                for (i, p) in params.iter().enumerate() {
                    let ty = param_types.get(i).and_then(|t| t.clone())
                        .unwrap_or(Type::Any);
                    self.declare(p, ty);
                }
                for s in body { self.check_stmt(s); }
                if let Some(ret) = return_type {
                    let ret = self.resolve(ret);
                    self.check_returns(body, &ret, *line);
                }
                self.pop_scope();
            }
            Statement::ComponentDef { params, param_types, body, .. } => {
                self.push_scope();
                for (i, p) in params.iter().enumerate() {
                    let ty = param_types.get(i).and_then(|t| t.clone())
                        .unwrap_or(Type::Any);
                    self.declare(p, ty);
                }
                for s in body { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::Test { body, .. } => {
                self.push_scope();
                for s in body { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.infer_expr(v);
                }
            }
            Statement::If { condition, then_branch, else_branch, line } => {
                let cond_ty = self.infer_expr(condition);
                if !matches!(cond_ty, Type::Any | Type::Boolean) {
                    self.error(*line, format!(
                        "الشرط يجب أن يكون منطقيًا، وليس \"{}\"",
                        cond_ty.display_ar()
                    ));
                }
                self.push_scope();
                for s in then_branch { self.check_stmt(s); }
                self.pop_scope();
                self.push_scope();
                for s in else_branch { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::ForEach { var, iterable, body, line } => {
                let iter_ty = self.infer_expr(iterable);
                let resolved = self.resolve(&iter_ty);
                let elem_ty = match resolved {
                    Type::List(inner) => *inner,
                    Type::Text => Type::Text,
                    Type::Any => Type::Any,
                    _ => {
                        self.error(*line, format!(
                            "\"لكل\" تحتاج قائمة، وليس \"{}\"",
                            resolved.display_ar()
                        ));
                        Type::Any
                    }
                };
                self.push_scope();
                self.declare(var, elem_ty);
                for s in body { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::RangeFor { var, start, end, step, body, line } => {
                let s_ty = self.infer_expr(start);
                if !matches!(s_ty, Type::Any | Type::Number) {
                    self.error(*line, "بداية الحلقة يجب أن تكون عددًا".into());
                }
                let e_ty = self.infer_expr(end);
                if !matches!(e_ty, Type::Any | Type::Number) {
                    self.error(*line, "نهاية الحلقة يجب أن تكون عددًا".into());
                }
                if let Some(st) = step {
                    let st_ty = self.infer_expr(st);
                    if !matches!(st_ty, Type::Any | Type::Number) {
                        self.error(*line, "الخطوة يجب أن تكون عددًا".into());
                    }
                }
                self.push_scope();
                self.declare(var, Type::Number);
                for s in body { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::While { condition, body, line } => {
                let cond_ty = self.infer_expr(condition);
                if !matches!(cond_ty, Type::Any | Type::Boolean) {
                    self.error(*line, "شرط \"بينما\" يجب أن يكون منطقيًا".into());
                }
                self.push_scope();
                for s in body { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::TryCatch { try_body, catch_var, catch_body, .. } => {
                self.push_scope();
                for s in try_body { self.check_stmt(s); }
                self.pop_scope();
                self.push_scope();
                self.declare(catch_var, Type::Text);
                for s in catch_body { self.check_stmt(s); }
                self.pop_scope();
            }
            Statement::HtmlElement { content, children, events, .. } => {
                if let Some(c) = content {
                    self.infer_expr(c);
                }
                if let Some(ch) = children {
                    for s in ch { self.check_stmt(s); }
                }
                for ev in events {
                    self.push_scope();
                    for s in &ev.body { self.check_stmt(s); }
                    self.pop_scope();
                }
            }
            Statement::Call { args, .. } => {
                for a in args { self.infer_expr(a); }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn check_returns(&mut self, body: &[Statement], expected: &Type, _line: usize) {
        for stmt in body {
            if let Statement::Return { value: Some(v), line } = stmt {
                let actual = self.infer_expr(v);
                if !self.compatible(&actual, expected) {
                    self.error(*line, format!(
                        "نوع الإرجاع \"{}\" لا يوافق \"{}\"",
                        actual.display_ar(), expected.display_ar()
                    ));
                }
            }
        }
    }

    // ==========================================
    // التعبيرات
    // ==========================================

    fn infer_expr(&mut self, expr: &Expression) -> Type {
        match expr {
            Expression::String(_) => Type::Text,
            Expression::Number(_) => Type::Number,
            Expression::Boolean(_) => Type::Boolean,
            Expression::Null => Type::Nothing,
            Expression::Identifier(name) => {
                self.lookup(name).unwrap_or(Type::Any)
            }
            Expression::List(items) => {
                if items.is_empty() {
                    return Type::List(Box::new(Type::Any));
                }
                let first = self.infer_expr(&items[0]);
                let mut unified = first.clone();
                for item in &items[1..] {
                    let t = self.infer_expr(item);
                    if !self.compatible(&unified, &t) {
                        unified = Type::Any;
                    }
                }
                Type::List(Box::new(unified))
            }
            Expression::Dict(pairs) => {
                let resolved: Vec<(String, Type)> = pairs.iter()
                    .map(|(k, v)| (k.clone(), self.infer_expr(v)))
                    .collect();
                Type::Dict(resolved)
            }
            Expression::MemberAccess { object, .. } => {
                self.infer_expr(object);
                Type::Any
            }
            Expression::Index { object, index } => {
                let obj_ty = self.infer_expr(object);
                self.infer_expr(index);
                match self.resolve(&obj_ty) {
                    Type::List(inner) => *inner,
                    Type::Text => Type::Text,
                    _ => Type::Any,
                }
            }
            Expression::Call { name, args } => {
                let arg_types: Vec<Type> = args.iter()
                    .map(|a| self.infer_expr(a))
                    .collect();
                self.check_call(name, &arg_types)
            }
            Expression::Binary { left, op, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                self.infer_binary(&lt, op, &rt)
            }
            Expression::Comparison { left, right, .. } => {
                self.infer_expr(left);
                self.infer_expr(right);
                Type::Boolean
            }
            Expression::Logical { left, right, .. } => {
                self.infer_expr(left);
                self.infer_expr(right);
                Type::Boolean
            }
            Expression::Not(e) => {
                self.infer_expr(e);
                Type::Boolean
            }
            Expression::Neg(e) => {
                let t = self.infer_expr(e);
                if !matches!(t, Type::Number | Type::Any) {
                    self.error(0, format!(
                        "النفي يحتاج عددًا، وليس \"{}\"", t.display_ar()
                    ));
                }
                Type::Number
            }
        }
    }

    fn infer_binary(&mut self, lt: &Type, op: &BinOp, rt: &Type) -> Type {
        match op {
            BinOp::Add => {
                if matches!(lt, Type::Text) || matches!(rt, Type::Text) {
                    Type::Text
                } else if matches!(lt, Type::Number) && matches!(rt, Type::Number) {
                    Type::Number
                } else if matches!(lt, Type::Any) || matches!(rt, Type::Any) {
                    Type::Any
                } else {
                    self.error(0, format!(
                        "لا يمكن جمع \"{}\" + \"{}\"",
                        lt.display_ar(), rt.display_ar()
                    ));
                    Type::Any
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if !matches!(lt, Type::Number | Type::Any) {
                    self.error(0, format!(
                        "العملية تحتاج عددًا، وليس \"{}\"", lt.display_ar()
                    ));
                }
                if !matches!(rt, Type::Number | Type::Any) {
                    self.error(0, format!(
                        "العملية تحتاج عددًا، وليس \"{}\"", rt.display_ar()
                    ));
                }
                Type::Number
            }
        }
    }

    fn check_call(&mut self, name: &str, args: &[Type]) -> Type {
        // دوال مدمجة معروفة
        let builtin = match name {
            "سلسلة" | "نص" => Some(Type::Text),
            "عدد" | "رقم_صحيح" | "رقم_عشري" => Some(Type::Number),
            "منطقي" | "تحقق_صحة" => Some(Type::Boolean),
            "طول" | "جذر" | "قوة" | "قوس" | "تقريب" | "أرضي" | "سقف"
            | "مطلق" | "أصغر" | "أكبر" | "بحث" | "مجموع" | "متوسط"
            | "أدنى" | "أقصى" | "عشوائي" | "زمن" | "سنة" => Some(Type::Number),
            "نوع" | "اقتطع" | "استبدل" | "مكرر" | "ازل_فراغات"
            | "كبير" | "صغير" | "انضم" | "دمج" | "تاريخ" => Some(Type::Text),
            "يحتوي" | "يبدأ_بـ" | "ينتهي_بـ" | "يحتوي_مفتاح"
            | "يحتوي_قائمة" | "تضمن" => Some(Type::Boolean),
            "قسم" | "مفاتيح" => Some(Type::List(Box::new(Type::Text))),
            "قيم" | "فرق" => Some(Type::List(Box::new(Type::Any))),
            "أول" | "آخر" | "مفهرس" => Some(Type::Any),
            "أضف" | "أضف_أمام" | "أدرج" | "احذف" | "احذف_أخير"
            | "اعكس" | "اقلب" | "رتب" | "فرز" | "ترتيب"
            | "مدى" => Some(Type::List(Box::new(Type::Any))),
            _ => None,
        };
        if let Some(t) = builtin {
            return t;
        }

        // دوال المستخدم
        if let Some((params, ret)) = self.funcs.get(name).cloned() {
            for (i, (arg, expected)) in args.iter().zip(params.iter()).enumerate() {
                if !self.compatible(arg, expected) {
                    self.error(0, format!(
                        "الدالة \"{}\" — الوسيط {}: متوقع \"{}\"، وجد \"{}\"",
                        name, i + 1, expected.display_ar(), arg.display_ar()
                    ));
                }
            }
            return ret;
        }
        Type::Any
    }
}