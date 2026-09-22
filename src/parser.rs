//! المحلل النحوي (Parser) للغة Rino.

use crate::ast::{BinOp, Event, Expression, Program, Statement, StyleRule};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        let t = self.peek().clone();
        if t.kind == kind {
            Ok(self.advance())
        } else {
            Err(format!(
                "متوقع {:?} لكن وجد {:?} في السطر {}",
                kind, t.kind, t.line
            ))
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut page_title = None;
        let mut styles = Vec::new();
        let mut statements = Vec::new();

        while self.peek().kind != TokenKind::EOF {
            match self.peek().kind.clone() {
                TokenKind::Page => { page_title = Some(self.parse_page()?); }
                TokenKind::Style => { styles = self.parse_style_block()?; }
                _ => { statements.push(self.parse_statement()?); }
            }
        }

        Ok(Program { page_title, styles, statements })
    }

    fn parse_page(&mut self) -> Result<Expression, String> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let title = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;
        Ok(title)
    }

    fn parse_style_block(&mut self) -> Result<Vec<StyleRule>, String> {
        self.advance(); // نمط
        self.expect(TokenKind::LBrace)?;
        let mut rules = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            let selector = match self.advance().kind {
                TokenKind::H1 => "h1".to_string(),
                TokenKind::P => "p".to_string(),
                TokenKind::Button => "button".to_string(),
                other => return Err(format!("محدد غير مدعوم: {:?}", other)),
            };
            self.expect(TokenKind::LBrace)?;
            let mut props = Vec::new();
            while self.peek().kind != TokenKind::RBrace {
                let prop_name = match self.advance().kind {
                    TokenKind::Identifier(s) => s,
                    other => return Err(format!("متوقع اسم خاصية، وجد {:?}", other)),
                };
                self.expect(TokenKind::Colon)?;
                let prop_value = match self.advance().kind {
                    TokenKind::String(s) => s,
                    TokenKind::Number(n) => {
                        if n.fract() == 0.0 { format!("{}", n as i64) }
                        else { format!("{}", n) }
                    }
                    other => return Err(format!("متوقع قيمة، وجد {:?}", other)),
                };
                props.push((prop_name, prop_value));
                if self.peek().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace)?;
            rules.push(StyleRule { selector, properties: props });
        }

        self.expect(TokenKind::RBrace)?;
        Ok(rules)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        let token = self.peek().clone();
        match &token.kind {
            TokenKind::Let => self.parse_let(),
            TokenKind::Print => self.parse_print(),
            TokenKind::H1 => self.parse_html_element("h1"),
            TokenKind::P => self.parse_html_element("p"),
            TokenKind::Button => self.parse_html_element("button"),
            _ => Err(format!(
                "جملة غير متوقعة {:?} في السطر {}",
                token.kind, token.line
            )),
        }
    }

    fn parse_let(&mut self) -> Result<Statement, String> {
        let kw = self.advance();
        let name = match self.advance().kind {
            TokenKind::Identifier(n) => n,
            other => return Err(format!("متوقع اسم متغير، وجد {:?}", other)),
        };
        self.expect(TokenKind::Assign)?;
        let value = self.parse_expression()?;
        Ok(Statement::Let { name, value, line: kw.line })
    }

    fn parse_print(&mut self) -> Result<Statement, String> {
        let kw = self.advance();
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            args.push(self.parse_expression()?);
            while self.peek().kind == TokenKind::Comma {
                self.advance();
                args.push(self.parse_expression()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(Statement::Call { name: "اطبع".to_string(), args, line: kw.line })
    }

    /// عنوان("نص") عند_الضغط { ... }
    fn parse_html_element(&mut self, tag: &str) -> Result<Statement, String> {
        let kw = self.advance();
        self.expect(TokenKind::LParen)?;
        let content = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;

        let mut events = Vec::new();
        loop {
            let kind = match self.peek().kind {
                TokenKind::OnClick => "click",
                TokenKind::OnChange => "change",
                _ => break,
            };
            self.advance();
            self.expect(TokenKind::LBrace)?;
            let body = self.parse_block()?;
            self.expect(TokenKind::RBrace)?;
            events.push(Event { kind: kind.to_string(), body });
        }

        Ok(Statement::HtmlElement {
            tag: tag.to_string(),
            content,
            events,
            line: kw.line,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        let mut stmts = Vec::new();
        while self.peek().kind != TokenKind::RBrace
            && self.peek().kind != TokenKind::EOF
        {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_primary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let token = self.advance();
        match token.kind {
            TokenKind::String(s) => Ok(Expression::String(s)),
            TokenKind::Number(n) => Ok(Expression::Number(n)),
            TokenKind::True => Ok(Expression::Boolean(true)),
            TokenKind::False => Ok(Expression::Boolean(false)),
            TokenKind::Null => Ok(Expression::Null),
            TokenKind::Identifier(name) => Ok(Expression::Identifier(name)),
            TokenKind::LParen => {
                let e = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            other => Err(format!(
                "تعبير غير متوقع {:?} في السطر {}",
                other, token.line
            )),
        }
    }
}