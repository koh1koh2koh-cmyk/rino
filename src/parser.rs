//! المحلل النحوي (Parser) للغة Rino.

use crate::ast::*;
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token { &self.tokens[self.pos] }
    fn kind(&self) -> &TokenKind { &self.tokens[self.pos].kind }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 { self.pos += 1; }
        t
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        let t = self.peek().clone();
        if t.kind == kind { Ok(self.advance()) }
        else { Err(format!("متوقع {:?} لكن وجد {:?} في السطر {}", kind, t.kind, t.line)) }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance().kind {
            TokenKind::Identifier(n) => Ok(n),
            other => Err(format!("متوقع اسم، وجد {:?}", other)),
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut page_title = None;
        let mut styles = Vec::new();
        let mut state = Vec::new();
        let mut functions = Vec::new();
        let mut components = Vec::new();
        let mut body = Vec::new();
        let mut top_level = Vec::new();

        while *self.kind() != TokenKind::EOF {
            match self.kind().clone() {
                TokenKind::Page => page_title = Some(self.parse_page()?),
                TokenKind::Style => styles = self.parse_style_block()?,
                TokenKind::State => {
                    self.advance();
                    self.expect(TokenKind::LBrace)?;
                    while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::EOF {
                        let mut s = self.parse_statement()?;
                        if let Statement::Let { is_state, .. } = &mut s { *is_state = true; }
                        state.push(s);
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                TokenKind::Logic => {
                    self.advance();
                    self.expect(TokenKind::LBrace)?;
                    while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::EOF {
                        functions.push(self.parse_statement()?);
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                TokenKind::Body => {
                    self.advance();
                    self.expect(TokenKind::LBrace)?;
                    while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::EOF {
                        body.push(self.parse_statement()?);
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                TokenKind::Component => components.push(self.parse_component()?),
                _ => top_level.push(self.parse_statement()?),
            }
        }

        if body.is_empty() && !top_level.is_empty() {
            body = top_level;
        }

        Ok(Program { page_title, styles, state, functions, components, body, top_level: Vec::new() })
    }

    fn parse_page(&mut self) -> Result<Expression, String> {
        self.advance();
        self.expect(TokenKind::LParen)?;
        let e = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;
        Ok(e)
    }

    fn parse_component(&mut self) -> Result<Statement, String> {
        let tok = self.advance();
        let name = match self.advance().kind {
            TokenKind::String(s) => s,
            other => return Err(format!("متوقع اسم نصي للمكون، وجد {:?}", other)),
        };
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if *self.kind() != TokenKind::RParen {
            params.push(self.expect_ident()?);
            while *self.kind() == TokenKind::Comma {
                self.advance();
                params.push(self.expect_ident()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        let body = self.parse_block()?;
        Ok(Statement::ComponentDef { name, params, body, line: tok.line })
    }

    fn parse_selector(&mut self) -> Result<(String, SelectorKind), String> {
        let kind_marker = self.kind().clone();
        match kind_marker {
            TokenKind::Dot => {
                self.advance();
                let name = self.expect_ident()?;
                Ok((name, SelectorKind::Class))
            }
            TokenKind::Hash => {
                self.advance();
                let name = self.expect_ident()?;
                Ok((name, SelectorKind::Id))
            }
            _ => {
                let tag = match self.advance().kind {
                    TokenKind::H1 => "h1".to_string(),
                    TokenKind::P => "p".to_string(),
                    TokenKind::Button => "button".to_string(),
                    TokenKind::Image => "img".to_string(),
                    TokenKind::Link => "a".to_string(),
                    TokenKind::Input => "input".to_string(),
                    TokenKind::List => "ul".to_string(),
                    TokenKind::ListItem => "li".to_string(),
                    TokenKind::Div => "div".to_string(),
                    TokenKind::Section => "section".to_string(),
                    TokenKind::Header => "header".to_string(),
                    TokenKind::Footer => "footer".to_string(),
                    TokenKind::Bold => "b".to_string(),
                    TokenKind::Italic => "i".to_string(),
                    other => return Err(format!("محدد غير مدعوم: {:?}", other)),
                };
                Ok((tag, SelectorKind::Tag))
            }
        }
    }

    fn parse_properties(&mut self) -> Result<Vec<(String, String)>, String> {
        self.expect(TokenKind::LBrace)?;
        let mut props = Vec::new();
        while *self.kind() != TokenKind::RBrace {
            if *self.kind() == TokenKind::OnHover { break; }
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let val = match self.advance().kind {
                TokenKind::String(s) => s,
                TokenKind::Number(n) => {
                    if n.fract() == 0.0 { format!("{}", n as i64) }
                    else { format!("{}", n) }
                }
                other => return Err(format!("متوقع قيمة، وجد {:?}", other)),
            };
            props.push((name, val));
            if *self.kind() == TokenKind::Comma { self.advance(); }
        }
        Ok(props)
    }

    fn parse_style_block(&mut self) -> Result<Vec<StyleRule>, String> {
        self.advance();
        self.expect(TokenKind::LBrace)?;
        let mut rules = Vec::new();

        while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::EOF {
            let (selector, selector_kind) = self.parse_selector()?;
            self.expect(TokenKind::LBrace)?;

            let mut properties = Vec::new();
            let mut hover_properties = Vec::new();

            while *self.kind() != TokenKind::RBrace {
                if *self.kind() == TokenKind::OnHover {
                    self.advance();
                    hover_properties = self.parse_properties()?;
                    self.expect(TokenKind::RBrace)?;
                    continue;
                }
                let name = self.expect_ident()?;
                self.expect(TokenKind::Colon)?;
                let val = match self.advance().kind {
                    TokenKind::String(s) => s,
                    TokenKind::Number(n) => {
                        if n.fract() == 0.0 { format!("{}", n as i64) }
                        else { format!("{}", n) }
                    }
                    other => return Err(format!("متوقع قيمة، وجد {:?}", other)),
                };
                properties.push((name, val));
                if *self.kind() == TokenKind::Comma { self.advance(); }
            }
            self.expect(TokenKind::RBrace)?;

            rules.push(StyleRule { selector, selector_kind, properties, hover_properties });
        }
        self.expect(TokenKind::RBrace)?;
        Ok(rules)
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::EOF {
            stmts.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Let => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expression()?;
                Ok(Statement::Let { name, value, is_state: false, line: tok.line })
            }
            TokenKind::Const => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expression()?;
                Ok(Statement::Const { name, value, line: tok.line })
            }
            TokenKind::Function => self.parse_function(),
            TokenKind::Return => {
                self.advance();
                let value = if matches!(*self.kind(), TokenKind::RBrace | TokenKind::EOF) {
                    None
                } else { Some(self.parse_expression()?) };
                Ok(Statement::Return { value, line: tok.line })
            }
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::Try => self.parse_try(),
            TokenKind::H1 => self.parse_html("h1"),
            TokenKind::P => self.parse_html("p"),
            TokenKind::Button => self.parse_html("button"),
            TokenKind::Image => self.parse_html("img"),
            TokenKind::Link => self.parse_html("a"),
            TokenKind::Input => self.parse_html("input"),
            TokenKind::ListItem => self.parse_html("li"),
            TokenKind::Bold => self.parse_html("b"),
            TokenKind::Italic => self.parse_html("i"),
            TokenKind::Break => self.parse_html("br"),
            TokenKind::Div => self.parse_block_element("div"),
            TokenKind::Section => self.parse_block_element("section"),
            TokenKind::Header => self.parse_block_element("header"),
            TokenKind::Footer => self.parse_block_element("footer"),
            TokenKind::List => self.parse_block_element("ul"),
            TokenKind::Print => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let mut args = Vec::new();
                if *self.kind() != TokenKind::RParen {
                    args.push(self.parse_expression()?);
                    while *self.kind() == TokenKind::Comma {
                        self.advance();
                        args.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RParen)?;
                Ok(Statement::Call { name: "اطبع".into(), args, line: tok.line })
            }
            TokenKind::And | TokenKind::Or => {
                self.advance();
                Ok(Statement::Call { name: "_skip_".into(), args: vec![], line: tok.line })
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if *self.kind() == TokenKind::Assign {
                    self.advance();
                    let value = self.parse_expression()?;
                    Ok(Statement::Assignment { name, value, line: tok.line })
                } else if *self.kind() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.kind() != TokenKind::RParen {
                        args.push(self.parse_expression()?);
                        while *self.kind() == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Statement::Call { name, args, line: tok.line })
                } else {
                    Err(format!("جملة غير مفهومة للسطر {}", tok.line))
                }
            }
            _ => Err(format!("جملة غير متوقعة {:?} في السطر {}", tok.kind, tok.line)),
        }
    }

    fn parse_block_element(&mut self, tag: &str) -> Result<Statement, String> {
        let tok = self.advance();
        self.expect(TokenKind::LBrace)?;
        let mut children = Vec::new();
        while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::EOF {
            children.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::HtmlElement {
            tag: tag.to_string(), content: None, attrs: vec![],
            children: Some(children), events: vec![], line: tok.line,
        })
    }

    fn parse_function(&mut self) -> Result<Statement, String> {
        let tok = self.advance();
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if *self.kind() != TokenKind::RParen {
            params.push(self.expect_ident()?);
            while *self.kind() == TokenKind::Comma {
                self.advance();
                params.push(self.expect_ident()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        let body = self.parse_block()?;
        Ok(Statement::Function { name, params, body, line: tok.line })
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        let tok = self.advance();
        self.expect(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;
        let then_branch = self.parse_block()?;
        let else_branch = if *self.kind() == TokenKind::Else {
            self.advance();
            if *self.kind() == TokenKind::If {
                vec![self.parse_if()?]
            } else { self.parse_block()? }
        } else { Vec::new() };
        Ok(Statement::If { condition, then_branch, else_branch, line: tok.line })
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
        let tok = self.advance();
        self.expect(TokenKind::LParen)?;
        let var = self.expect_ident()?;

        // هل هو `لكل (i في قائمة)` أم `لكل (i من a إلى b)`؟
        if *self.kind() == TokenKind::In {
            self.advance();
            let iterable = self.parse_expression()?;
            self.expect(TokenKind::RParen)?;
            let body = self.parse_block()?;
            return Ok(Statement::ForEach { var, iterable, body, line: tok.line });
        }

        if *self.kind() == TokenKind::From {
            self.advance();
            let start = self.parse_expression()?;
            self.expect(TokenKind::To)?;
            let end = self.parse_expression()?;
            let step = if *self.kind() == TokenKind::Step {
                self.advance();
                Some(self.parse_expression()?)
            } else { None };
            self.expect(TokenKind::RParen)?;
            let body = self.parse_block()?;
            return Ok(Statement::RangeFor { var, start, end, step, body, line: tok.line });
        }

        Err(format!("متوقع 'في' أو 'من' في السطر {}", tok.line))
    }

    fn parse_try(&mut self) -> Result<Statement, String> {
        let tok = self.advance();
        let try_body = self.parse_block()?;
        self.expect(TokenKind::Catch)?;
        self.expect(TokenKind::LParen)?;
        let catch_var = self.expect_ident()?;
        self.expect(TokenKind::RParen)?;
        let catch_body = self.parse_block()?;
        Ok(Statement::TryCatch { try_body, catch_var, catch_body, line: tok.line })
    }

    fn parse_html(&mut self, tag: &str) -> Result<Statement, String> {
        let tok = self.advance();
        self.expect(TokenKind::LParen)?;

        let mut args = Vec::new();
        if *self.kind() != TokenKind::RParen {
            args.push(self.parse_expression()?);
            while *self.kind() == TokenKind::Comma {
                self.advance();
                args.push(self.parse_expression()?);
            }
        }
        self.expect(TokenKind::RParen)?;

        let (content, attrs): (Option<Expression>, Vec<(String, Expression)>) = match tag {
            "img" => {
                let a = args.into_iter().next().ok_or_else(|| "صورة تحتاج رابطًا".to_string())?;
                (None, vec![("src".into(), a)])
            }
            "input" => {
                let mut it = args.into_iter();
                let placeholder = it.next().ok_or_else(|| "مدخل يحتاج نصًا".to_string())?;
                let mut attrs = vec![("placeholder".into(), placeholder)];
                if let Some(id_expr) = it.next() { attrs.push(("id".into(), id_expr)); }
                if let Some(class_expr) = it.next() { attrs.push(("class".into(), class_expr)); }
                (None, attrs)
            }
            "a" => {
                let mut it = args.into_iter();
                let text = it.next().ok_or_else(|| "رابط يحتاج نصًا".to_string())?;
                let url = it.next().ok_or_else(|| "رابط يحتاج URL".to_string())?;
                (Some(text), vec![("href".into(), url)])
            }
            "br" => (None, vec![]),
            _ => {
                let mut it = args.into_iter();
                let c = it.next();
                let mut attrs = vec![];
                if let Some(id_expr) = it.next() { attrs.push(("id".into(), id_expr)); }
                if let Some(class_expr) = it.next() { attrs.push(("class".into(), class_expr)); }
                (c, attrs)
            }
        };

        let mut events = Vec::new();
        loop {
            let kind = match self.kind() {
                TokenKind::OnClick => "click",
                TokenKind::OnChange => "change",
                _ => break,
            };
            self.advance();
            let body = self.parse_block()?;
            events.push(Event { kind: kind.to_string(), body });
        }

        Ok(Statement::HtmlElement {
            tag: tag.to_string(), content, attrs,
            children: None, events, line: tok.line,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and()?;
        while *self.kind() == TokenKind::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::Logical { left: Box::new(left), op: LogOp::Or, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison()?;
        while *self.kind() == TokenKind::And {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::Logical { left: Box::new(left), op: LogOp::And, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let left = self.parse_additive()?;
        let op = match self.kind() {
            TokenKind::EqEq => CmpOp::Eq,
            TokenKind::NotEq => CmpOp::Ne,
            TokenKind::Greater => CmpOp::Gt,
            TokenKind::Less => CmpOp::Lt,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_additive()?;
        Ok(Expression::Comparison { left: Box::new(left), op, right: Box::new(right) })
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expression::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expression::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        if *self.kind() == TokenKind::Not {
            self.advance();
            let e = self.parse_unary()?;
            Ok(Expression::Not(Box::new(e)))
        } else { self.parse_primary() }
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_atom()?;
        while *self.kind() == TokenKind::Dot {
            self.advance();
            let prop = self.expect_ident()?;
            expr = Expression::MemberAccess { object: Box::new(expr), property: prop };
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<Expression, String> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::String(s) => Ok(Expression::String(s)),
            TokenKind::Number(n) => Ok(Expression::Number(n)),
            TokenKind::True => Ok(Expression::Boolean(true)),
            TokenKind::False => Ok(Expression::Boolean(false)),
            TokenKind::Null => Ok(Expression::Null),
            TokenKind::Identifier(name) => {
                if *self.kind() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.kind() != TokenKind::RParen {
                        args.push(self.parse_expression()?);
                        while *self.kind() == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expression::Call { name, args })
                } else { Ok(Expression::Identifier(name)) }
            }
            TokenKind::LParen => {
                let e = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBracket => {
                let mut items = Vec::new();
                if *self.kind() != TokenKind::RBracket {
                    items.push(self.parse_expression()?);
                    while *self.kind() == TokenKind::Comma {
                        self.advance();
                        items.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expression::List(items))
            }
            TokenKind::LBrace => {
                let mut pairs = Vec::new();
                if *self.kind() != TokenKind::RBrace {
                    loop {
                        let key = match self.advance().kind {
                            TokenKind::Identifier(s) => s,
                            TokenKind::String(s) => s,
                            other => return Err(format!("متوقع اسم خاصية، وجد {:?}", other)),
                        };
                        self.expect(TokenKind::Colon)?;
                        let value = self.parse_expression()?;
                        pairs.push((key, value));
                        if *self.kind() == TokenKind::Comma { self.advance(); }
                        else { break; }
                    }
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Expression::Dict(pairs))
            }
            other => Err(format!("تعبير غير متوقع {:?} في السطر {}", other, tok.line)),
        }
    }
}