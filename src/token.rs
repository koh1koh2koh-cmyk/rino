#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // كتل أساسية
    Page, Style, State, Derived, Body, Logic,

    // تعريفات
    Let, Const, Function, Return,

    // تحكم
    If, Else, For, In, And, Or, Not,
    While, Break, Continue,

    // قيم حرفية
    True, False, Null,

    // إعدادات
    Settings, Correction,

    // مكوّنات
    Component,

    // معالجة الأخطاء
    Try, Catch, From, To, Step,

    // اختبارات
    Test, Expect, ExpectEquals,

    // عناوين
    H1, H2, H3, H4, H5, H6,

    // نصوص
    P, Span, Strong, Em, Mark, Small, Del, Ins,
    Blockquote, Code, Pre, Kbd, Abbr, Cite, Dfn, Address, Time, Data,
    Bdi, Bdo, Wbr, Q, Samp, Var, Sub, Sup,

    // تفاعل
    Button, Image, Link, Input, TextArea, Select, Option, Form,
    Label, FieldSet, Legend, Checkbox, Radio, FileInput,
    DataList, OptGroup, Output, Progress, Meter,

    // تخطيط
    Div, Section, Header, Footer, Main, Nav, Article, Aside,
    Figure, FigCaption, Details, Summary, Dialog, Template, NoScript, Slot,

    // قوائم
    List, ListItem, OrderedList,

    // جداول
    Table, Row, Cell, HeaderCell, TableBody, TableHead, TableFooter, Caption, ColGroup, Col,

    // وسائط
    Video, Audio, IFrame, HR, Break_, Canvas, SVG, Picture, Source, Track, Map, Area,

    // تنسيقات إضافية
    Bold, Italic,

    // محددات
    Dot, Hash,

    // أنماط تفاعلية
    OnHover, OnFocus, OnActive,

    // طباعة
    Print,

    // أحداث
    OnClick, OnChange, OnSubmit, OnInput, OnKeyDown, OnKeyUp, OnLoad, OnMouseOver,

    // رموز
    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,
    Comma, Colon, Assign,
    EqEq, NotEq, Greater, Less, Ge, Le,
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
        // كتل أساسية
        "صفحة" => Page,
        "نمط" => Style,
        "حالة" => State,
        "مشتق" => Derived,
        "جسم" => Body,
        "منطق" => Logic,

        // تعريفات
        "دع" => Let,
        "ثابت" => Const,
        "دالة" => Function,
        "أرجع" => Return,

        // تحكم
        "إذا" => If,
        "وإلا" => Else,
        "لكل" => For,
        "في" => In,
        "بينما" => While,
        "أوقف" => Break,
        "استمر" => Continue,
        "من" => From,
        "إلى" => To,
        "خطوة" => Step,

        // معالجة أخطاء
        "حاول" => Try,
        "أمسك" => Catch,

        // اختبارات
        "اختبر" => Test,
        "توقع" => Expect,
        "توقع_يساوي" => ExpectEquals,

        // منطقي
        "و" => And,
        "أو" => Or,
        "ليس" => Not,
        "صحيح" => True,
        "خطأ" => False,
        "لا_شيء" => Null,

        // إعدادات
        "إعدادات" => Settings,
        "تصحيح" => Correction,
        "مكون" => Component,

        // أنماط تفاعلية
        "عند_المرور" => OnHover,
        "عند_التركيز" => OnFocus,
        "عند_التنشيط" => OnActive,

        // عناوين
        "عنوان" => H1,
        "عنوان_فرعي" => H2,
        "عنوان_صغير" => H3,
        "عنوان_رابع" => H4,
        "عنوان_خامس" => H5,
        "عنوان_سادس" => H6,

        // نصوص
        "فقرة" => P,
        "نص_داخلي" => Span,
        "قوي" => Strong,
        "مؤكد" => Em,
        "مظلل" => Mark,
        "فقرة_صغيرة" => Small,
        "محذوف" => Del,
        "مضاف" => Ins,
        "اقتباس" => Blockquote,
        "كود" => Code,
        "نص_منسق" => Pre,
        "مفتاح" => Kbd,
        "اختصار" => Abbr,
        "اقتباس_مصدر" => Cite,
        "تعريف" => Dfn,
        "عنوان_بريد" => Address,
        "وقت" => Time,
        "بيان" => Data,
        "نص_بديل" => Bdi,
        "نص_اتجاهي" => Bdo,
        "نقطة_كسر" => Wbr,
        "اقتباس_قصير" => Q,
        "نموذج_كود" => Samp,
        "متغير" => Var,
        "منخفض" => Sub,
        "مرتفع" => Sup,

        // تفاعل
        "زر" => Button,
        "صورة" => Image,
        "رابط" => Link,
        "مدخل" => Input,
        "مربع_نص" => TextArea,
        "اختيار" => Select,
        "خيار" => Option,
        "نموذج" => Form,
        "تسمية" => Label,
        "مجموعة" => FieldSet,
        "عنوان_مجموعة" => Legend,
        "مربع_اختيار" => Checkbox,
        "زر_اختيار" => Radio,
        "ملف" => FileInput,
        "قائمة_بيانات" => DataList,
        "مجموعة_خيارات" => OptGroup,
        "إخراج" => Output,
        "تقدم" => Progress,
        "مقياس" => Meter,

        // تخطيط
        "قسم" => Div,
        "منطقة" => Section,
        "ترويسة" => Header,
        "تذييل" => Footer,
        "رئيسي" => Main,
        "تنقل" => Nav,
        "مقال" => Article,
        "جانبي" => Aside,
        "شكل" => Figure,
        "وصف_شكل" => FigCaption,
        "تفاصيل" => Details,
        "ملخص" => Summary,
        "حوار" => Dialog,
        "قالب" => Template,
        "بدون_سكربت" => NoScript,
        "فتحة" => Slot,

        // قوائم
        "قائمة" => List,
        "عنصر_قائمة" => ListItem,
        "قائمة_مرقمة" => OrderedList,

        // جداول
        "جدول" => Table,
        "صف" => Row,
        "خلية" => Cell,
        "خلية_رأس" => HeaderCell,
        "جسم_جدول" => TableBody,
        "رأس_جدول" => TableHead,
        "تذييل_جدول" => TableFooter,
        "عنوان_جدول" => Caption,
        "مجموعة_أعمدة" => ColGroup,
        "عمود" => Col,

        // وسائط
        "فيديو" => Video,
        "موسيقى" => Audio,
        "إطار" => IFrame,
        "فاصل_أفقي" => HR,
        "فاصل" => Break_,
        "رسم" => Canvas,
        "رسومات" => SVG,
        "صورة_متعددة" => Picture,
        "مصدر" => Source,
        "مسار" => Track,
        "خريطة" => Map,
        "منطقة_صورة" => Area,

        // تنسيقات إضافية
        "عريض" => Bold,
        "مائل" => Italic,

        // أحداث
        "عند_الضغط" => OnClick,
        "عند_التغيير" => OnChange,
        "عند_الإرسال" => OnSubmit,
        "عند_الإدخال" => OnInput,
        "عند_مفتاح_أسفل" => OnKeyDown,
        "عند_مفتاح_أعلى" => OnKeyUp,
        "عند_التحميل" => OnLoad,
        "عند_مرور_الفأرة" => OnMouseOver,

        // طباعة
        "اطبع" => Print,

        _ => return None,
    })
}