//! المحلل اللغوي (Lexer) للغة Rino.

use crate::correction::suggest_keyword;
use crate::token::{keyword_to_token, Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    pub corrections: Vec<String>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            corrections: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek_next(&self) -> Option<char> { self.chars.get(self.pos + 1).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() || is_diacritic(c) => { self.advance(); }
                Some('/') if self.peek_next() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' { break; }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.advance();
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == '\\' {
                self.advance();
                if let Some(esc) = self.peek() {
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        other => { s.push('\\'); s.push(other); }
                    }
                    self.advance();
                }
                continue;
            }
            if c == '"' { self.advance(); return Ok(s); }
            if c == '\n' { return Err(format!("نص غير مغلق في السطر {}", self.line)); }
            s.push(c);
            self.advance();
        }
        Err(format!("نص غير مغلق في السطر {}", self.line))
    }

    fn read_number(&mut self) -> Result<f64, String> {
        let mut s = String::new();
        let mut dots = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else if c == '.' {
                if dots > 0 {
                    return Err(format!("عدد غير صالح '{}' في السطر {}", s, self.line));
                }
                dots += 1;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s.parse().map_err(|_| format!("عدد غير صالح '{}' في السطر {}", s, self.line))
    }

    fn read_identifier(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_char(c) {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();
            let line = self.line;
            let column = self.column;

            let c = match self.peek() {
                Some(c) => c,
                None => {
                    tokens.push(Token { kind: TokenKind::EOF, line, column });
                    break;
                }
            };

            // النقطة
            if c == '.' {
                if let Some(next) = self.peek_next() {
                    if next.is_ascii_digit() {
                        let n = self.read_number()?;
                        tokens.push(Token { kind: TokenKind::Number(n), line, column });
                        continue;
                    }
                }
                self.advance();
                tokens.push(Token { kind: TokenKind::Dot, line, column });
                continue;
            }

            // الهاش
            if c == '#' {
                self.advance();
                tokens.push(Token { kind: TokenKind::Hash, line, column });
                continue;
            }

            // ؟ (علامة النوع الاختياري)
            if c == '؟' {
                self.advance();
                tokens.push(Token { kind: TokenKind::Question, line, column });
                continue;
            }

            // الرموز البسيطة
            let simple = match c {
                '(' => Some(TokenKind::LParen),
                ')' => Some(TokenKind::RParen),
                '{' => Some(TokenKind::LBrace),
                '}' => Some(TokenKind::RBrace),
                '[' => Some(TokenKind::LBracket),
                ']' => Some(TokenKind::RBracket),
                ',' => Some(TokenKind::Comma),
                ':' => Some(TokenKind::Colon),
                '+' => Some(TokenKind::Plus),
                '-' => Some(TokenKind::Minus),
                '*' => Some(TokenKind::Star),
                '/' => Some(TokenKind::Slash),
                '%' => Some(TokenKind::Percent),
                '>' => Some(TokenKind::Greater),
                '<' => Some(TokenKind::Less),
                _ => None,
            };

            if let Some(kind) = simple {
                self.advance();
                // ->
                if kind == TokenKind::Minus && self.peek() == Some('>') {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Arrow, line, column });
                    continue;
                }
                // >=
                if kind == TokenKind::Greater && self.peek() == Some('=') {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Ge, line, column });
                    continue;
                }
                // <=
                if kind == TokenKind::Less && self.peek() == Some('=') {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Le, line, column });
                    continue;
                }
                tokens.push(Token { kind, line, column });
                continue;
            }

            // = أو ==
            if c == '=' {
                self.advance();
                let kind = if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Assign
                };
                tokens.push(Token { kind, line, column });
                continue;
            }

            // ! أو !=
            if c == '!' {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::NotEq, line, column });
                    continue;
                }
                return Err(format!("رمز غير متوقع '!' في السطر {}", line));
            }

            // نص
            if c == '"' {
                let s = self.read_string()?;
                tokens.push(Token { kind: TokenKind::String(s), line, column });
                continue;
            }

            // عدد
            if c.is_ascii_digit() {
                let n = self.read_number()?;
                tokens.push(Token { kind: TokenKind::Number(n), line, column });
                continue;
            }

            // معرّف أو كلمة مفتاحية
            if is_ident_start(c) {
                let word = self.read_identifier();

                // 1) كلمة مفتاحية صحيحة
                if let Some(kind) = keyword_to_token(&word) {
                    tokens.push(Token { kind, line, column });
                    continue;
                }

                // 2) محاولة تصحيح إملائي (فقط للكلمات الطويلة ≥ 4 أحرف)
                if word.chars().count() >= 4 {
                    if let Some(fixed) = suggest_keyword(&word) {
                        self.corrections.push(format!(
                            "⚠ السطر {}: \"{}\" → \"{}\"",
                            line, word, fixed
                        ));
                        let kind = keyword_to_token(fixed).unwrap();
                        tokens.push(Token { kind, line, column });
                        continue;
                    }
                }

                // 3) معرّف عادي
                tokens.push(Token {
                    kind: TokenKind::Identifier(word),
                    line,
                    column,
                });
                continue;
            }

            return Err(format!(
                "رمز غير معروف '{}' في السطر {} العمود {}",
                c, line, column
            ));
        }

        Ok(tokens)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_diacritic(c: char) -> bool {
    let code = c as u32;
    (0x064B..=0x065F).contains(&code) || code == 0x0670
}