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
    Component,
    Try, Catch, From, To, Step,

    // عناصر واجهة
    H1, P, Button, Image, Link, Input, List, ListItem,
    Div, Section, Header, Footer,
    Bold, Italic, Break,

    // CSS selectors
    Dot, Hash,
    OnHover,

    // دوال مدمجة
    Print,

    // أحداث
    OnClick, OnChange,

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
        "من" => From,
        "إلى" => To,
        "خطوة" => Step,
        "حاول" => Try,
        "أمسك" => Catch,
        "و" => And,
        "أو" => Or,
        "ليس" => Not,
        "صحيح" => True,
        "خطأ" => False,
        "لا_شيء" => Null,
        "إعدادات" => Settings,
        "تصحيح" => Correction,
        "مكون" => Component,
        "عند_المرور" => OnHover,
        "عنوان" => H1,
        "فقرة" => P,
        "زر" => Button,
        "صورة" => Image,
        "رابط" => Link,
        "مدخل" => Input,
        "قائمة" => List,
        "عنصر_قائمة" => ListItem,
        "قسم" => Div,
        "منطقة" => Section,
        "ترويسة" => Header,
        "تذييل" => Footer,
        "عريض" => Bold,
        "مائل" => Italic,
        "فاصل" => Break,
        "اطبع" => Print,
        "عند_الضغط" => OnClick,
        "عند_التغيير" => OnChange,
        _ => return None,
    })
}