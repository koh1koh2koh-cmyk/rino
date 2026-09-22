#![allow(dead_code)]
//! تعريف أنواع الرموز (Tokens) للغة Rino.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // كلمات مفتاحية
    Page, Style, State, Body, Logic,
    Let, Const, Function, Return,
    If, Else, For, In, And, Or, Not,
    True, False, Null,
    Settings, Correction,

    // عناصر واجهة
    H1, P, Button, Image, Link, Input, List, ListItem,

    // دوال مدمجة
    Print,      // اطبع

    // أحداث
    OnClick,    // عند_الضغط
    OnChange,   // عند_التغيير

    // رموز
    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Comma, Colon, Assign,
    EqEq, NotEq, Greater, Less,
    Plus, Minus, Star, Slash, Percent,

    // قيم
    String(String),
    Number(f64),
    Identifier(String),

    EOF,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub fn keyword_to_token(word: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match word {
        "صفحة" => Page,
        "نمط" => Style,
        "حالة" => State,
        "جسم" => Body,
        "منطق" => Logic,
        "دع" => Let,
        "ثابت" => Const,
        "دالة" => Function,
        "أرجع" => Return,
        "إذا" => If,
        "وإلا" => Else,
        "لكل" => For,
        "في" => In,
        "و" => And,
        "أو" => Or,
        "ليس" => Not,
        "صحيح" => True,
        "خطأ" => False,
        "لا_شيء" => Null,
        "إعدادات" => Settings,
        "تصحيح" => Correction,
        "عنوان" => H1,
        "فقرة" => P,
        "زر" => Button,
        "صورة" => Image,
        "رابط" => Link,
        "مدخل" => Input,
        "قائمة" => List,
        "عنصر_قائمة" => ListItem,
        "اطبع" => Print,
        "عند_الضغط" => OnClick,
        "عند_التغيير" => OnChange,
        _ => return None,
    })
}