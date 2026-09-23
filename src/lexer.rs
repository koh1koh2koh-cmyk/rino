//! المحلل اللغوي (Lexer) للغة Rino.

use crate::correction::suggest_keyword;
use crate::token::{keyword_to_token, Token, TokenKind};

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    pub corrections: Vec<String>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            corrections: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek_next(&self) -> Option<char> { self.chars.get(self.pos + 1).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' { self.line += 1; self.column = 1; }
            else { self.column += 1; }
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
            if c == '"' { self.advance(); return Ok(s); }
            s.push(c);
            self.advance();
        }
        Err(format!("نص غير مغلق في السطر {}", self.line))
    }

    fn read_number(&mut self) -> f64 {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' { s.push(c); self.advance(); }
            else { break; }
        }
        s.parse().unwrap_or(0.0)
    }

    fn read_identifier(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_char(c) { s.push(c); self.advance(); }
            else { break; }
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

            // النقطة: إما عدد عشري أو محدد كلاس
            if c == '.' {
                if let Some(next) = self.peek_next() {
                    if next.is_ascii_digit() {
                        let n = self.read_number();
                        tokens.push(Token { kind: TokenKind::Number(n), line, column });
                        continue;
                    }
                }
                self.advance();
                tokens.push(Token { kind: TokenKind::Dot, line, column });
                continue;
            }

            // الهاش: محدد معرف
            if c == '#' {
                self.advance();
                tokens.push(Token { kind: TokenKind::Hash, line, column });
                continue;
            }

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
                tokens.push(Token { kind, line, column });
                continue;
            }

            if c == '=' {
                self.advance();
                let kind = if self.peek() == Some('=') { self.advance(); TokenKind::EqEq }
                    else { TokenKind::Assign };
                tokens.push(Token { kind, line, column });
                continue;
            }

            if c == '!' {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    tokens.push(Token { kind: TokenKind::NotEq, line, column });
                    continue;
                }
                return Err(format!("رمز غير متوقع '!' في السطر {}", line));
            }

            if c == '"' {
                let s = self.read_string()?;
                tokens.push(Token { kind: TokenKind::String(s), line, column });
                continue;
            }

            if c.is_ascii_digit() {
                let n = self.read_number();
                tokens.push(Token { kind: TokenKind::Number(n), line, column });
                continue;
            }

            if is_ident_start(c) {
                let word = self.read_identifier();
                if let Some(kind) = keyword_to_token(&word) {
                    tokens.push(Token { kind, line, column });
                    continue;
                }
                if word.chars().count() >= 2 {
                    if let Some(fixed) = suggest_keyword(&word) {
                        self.corrections.push(format!(
                            "⚠ السطر {}: \"{}\" → \"{}\"", line, word, fixed
                        ));
                        let kind = keyword_to_token(fixed).unwrap();
                        tokens.push(Token { kind, line, column });
                        continue;
                    }
                }
                tokens.push(Token { kind: TokenKind::Identifier(word), line, column });
                continue;
            }

            return Err(format!("رمز غير معروف '{}' في السطر {} العمود {}", c, line, column));
        }

        Ok(tokens)
    }
}

fn is_ident_start(c: char) -> bool { c.is_alphabetic() || c == '_' }
fn is_ident_char(c: char) -> bool { c.is_alphanumeric() || c == '_' }
fn is_diacritic(c: char) -> bool {
    let code = c as u32;
    (0x064B..=0x065F).contains(&code) || code == 0x0670
}