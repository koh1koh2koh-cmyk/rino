#![allow(dead_code)]
//! تعريف أنواع الرموز (Tokens) للغة Rino.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Page, Style, State, Body, Logic,
    Let, Const, Function, Return,
    If, Else, For, In, And, Or, Not,
    True, False, Null,
    Settings, Correction,
    Component,
    Try, Catch, From, To, Step,
    Test, Expect, ExpectEquals,

    H1, P, Button, Image, Link, Input, List, ListItem,
    Div, Section, Header, Footer,
    Bold, Italic, Break,
    Table, Row, Cell, Video, Audio, HR,

    Dot, Hash,
    OnHover,

    Print,

    OnClick, OnChange,

    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Comma, Colon, Assign,
    EqEq, NotEq, Greater, Less,
    Plus, Minus, Star, Slash, Percent,

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
        "اختبر" => Test,
        "توقع" => Expect,
        "توقع_يساوي" => ExpectEquals,
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
        "جدول" => Table,
        "صف" => Row,
        "خلية" => Cell,
        "فيديو" => Video,
        "موسيقى" => Audio,
        "فاصل_أفقي" => HR,
        "اطبع" => Print,
        "عند_الضغط" => OnClick,
        "عند_التغيير" => OnChange,
        _ => return None,
    })
}