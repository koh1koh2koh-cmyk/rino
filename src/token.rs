#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Page, Style, State, Derived, Body, Logic,
    TypeKw,                                   // ← جديد: نوع

    Let, Const, Function, Return,
    If, Else, For, In, And, Or, Not,
    While, Break, Continue,
    True, False, Null,
    Settings, Correction,
    Component,
    Try, Catch, From, To, Step,
    Test, Expect, ExpectEquals,

    H1, H2, H3, H4, H5, H6,
    P, Span, Strong, Em, Mark, Small, Del, Ins,
    Blockquote, Code, Pre, Kbd, Abbr, Cite, Dfn, Address, Time, Data,
    Bdi, Bdo, Wbr, Q, Samp, Var, Sub, Sup,

    Button, Image, Link, Input, TextArea, Select, Option, Form,
    Label, FieldSet, Legend, Checkbox, Radio, FileInput,
    DataList, OptGroup, Output, Progress, Meter,

    Div, Section, Header, Footer, Main, Nav, Article, Aside,
    Figure, FigCaption, Details, Summary, Dialog, Template, NoScript, Slot,

    List, ListItem, OrderedList,

    Table, Row, Cell, HeaderCell, TableBody, TableHead, TableFooter, Caption, ColGroup, Col,

    Video, Audio, IFrame, HR, Break_, Canvas, SVG, Picture, Source, Track, Map, Area,

    Bold, Italic,

    Dot, Hash,
    OnHover, OnFocus, OnActive,
    Print,
    OnClick, OnChange, OnSubmit, OnInput, OnKeyDown, OnKeyUp, OnLoad, OnMouseOver,

    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Comma, Colon, Assign,
    EqEq, NotEq, Greater, Less, Ge, Le,
    Plus, Minus, Star, Slash, Percent,

    Arrow,        // ← جديد: ->
    Question,     // ← جديد: ؟

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
        "صفحة" => Page, "نمط" => Style, "حالة" => State,
        "مشتق" => Derived, "جسم" => Body, "منطق" => Logic,
        "نوع" => TypeKw,                            // ← جديد

        "دع" => Let, "ثابت" => Const, "دالة" => Function, "أرجع" => Return,
        "إذا" => If, "وإلا" => Else, "لكل" => For, "في" => In,
        "بينما" => While, "أوقف" => Break, "استمر" => Continue,
        "من" => From, "إلى" => To, "خطوة" => Step,
        "حاول" => Try, "أمسك" => Catch,
        "اختبر" => Test, "توقع" => Expect, "توقع_يساوي" => ExpectEquals,
        "و" => And, "أو" => Or, "ليس" => Not,
        "صحيح" => True, "خطأ" => False, "لا_شيء" => Null,
        "إعدادات" => Settings, "تصحيح" => Correction,
        "مكون" => Component,
        "عند_المرور" => OnHover, "عند_التركيز" => OnFocus, "عند_التنشيط" => OnActive,
        "عنوان" => H1, "عنوان_فرعي" => H2, "عنوان_صغير" => H3,
        "عنوان_رابع" => H4, "عنوان_خامس" => H5, "عنوان_سادس" => H6,
        "فقرة" => P, "نص_داخلي" => Span, "قوي" => Strong, "مؤكد" => Em,
        "مظلل" => Mark, "فقرة_صغيرة" => Small, "محذوف" => Del, "مضاف" => Ins,
        "اقتباس" => Blockquote, "كود" => Code, "نص_منسق" => Pre, "مفتاح" => Kbd,
        "اختصار" => Abbr, "اقتباس_مصدر" => Cite, "تعريف" => Dfn,
        "عنوان_بريد" => Address, "وقت" => Time, "بيان" => Data,
        "نص_بديل" => Bdi, "نص_اتجاهي" => Bdo, "نقطة_كسر" => Wbr,
        "اقتباس_قصير" => Q, "نموذج_كود" => Samp, "متغير" => Var,
        "منخفض" => Sub, "مرتفع" => Sup,
        "زر" => Button, "صورة" => Image, "رابط" => Link,
        "مدخل" => Input, "مربع_نص" => TextArea, "اختيار" => Select, "خيار" => Option,
        "نموذج" => Form, "تسمية" => Label, "مجموعة" => FieldSet, "عنوان_مجموعة" => Legend,
        "مربع_اختيار" => Checkbox, "زر_اختيار" => Radio, "ملف" => FileInput,
        "قائمة_بيانات" => DataList, "مجموعة_خيارات" => OptGroup, "إخراج" => Output,
        "تقدم" => Progress, "مقياس" => Meter,
        "قسم" => Div, "منطقة" => Section, "ترويسة" => Header, "تذييل" => Footer,
        "رئيسي" => Main, "تنقل" => Nav, "مقال" => Article, "جانبي" => Aside,
        "شكل" => Figure, "وصف_شكل" => FigCaption, "تفاصيل" => Details, "ملخص" => Summary,
        "حوار" => Dialog, "قالب" => Template, "بدون_سكربت" => NoScript, "فتحة" => Slot,
        "قائمة" => List, "عنصر_قائمة" => ListItem, "قائمة_مرقمة" => OrderedList,
        "جدول" => Table, "صف" => Row, "خلية" => Cell, "خلية_رأس" => HeaderCell,
        "جسم_جدول" => TableBody, "رأس_جدول" => TableHead, "تذييل_جدول" => TableFooter,
        "عنوان_جدول" => Caption, "مجموعة_أعمدة" => ColGroup, "عمود" => Col,
        "فيديو" => Video, "موسيقى" => Audio, "إطار" => IFrame,
        "فاصل_أفقي" => HR, "فاصل" => Break_,
        "رسم" => Canvas, "رسومات" => SVG, "صورة_متعددة" => Picture,
        "مصدر" => Source, "مسار" => Track, "خريطة" => Map, "منطقة_صورة" => Area,
        "عريض" => Bold, "مائل" => Italic,
        "عند_الضغط" => OnClick, "عند_التغيير" => OnChange, "عند_الإرسال" => OnSubmit,
        "عند_الإدخال" => OnInput, "عند_مفتاح_أسفل" => OnKeyDown, "عند_مفتاح_أعلى" => OnKeyUp,
        "عند_التحميل" => OnLoad, "عند_مرور_الفأرة" => OnMouseOver,
        "اطبع" => Print,
        _ => return None,
    })
}