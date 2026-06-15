//! S表达式词法分析器
//!
//! 将S表达式字符串分解为Token流。

use std::str::Chars;
use std::iter::Peekable;

use super::error::{SExprError, SExprResult, Span};

/// Token类型
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// 左括号 `(`
    LParen,
    /// 右括号 `)`
    RParen,
    /// 符号（运算符名、变量名等）
    Symbol(String),
    /// 数值常量
    Number(f64),
    /// 冒号（用于 `:otherwise` 等）
    Colon,
    /// 文件结束
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::LParen => write!(f, "'('"),
            TokenKind::RParen => write!(f, "')'"),
            TokenKind::Symbol(s) => write!(f, "符号 '{}'", s),
            TokenKind::Number(n) => write!(f, "数字 {}", n),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Eof => write!(f, "文件结束"),
        }
    }
}

/// 带位置信息的Token
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token类型
    pub kind: TokenKind,
    /// 位置信息
    pub span: Span,
}

impl Token {
    /// 创建新Token
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
    
    /// 检查是否为特定类型
    pub fn is(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.kind) == std::mem::discriminant(kind)
    }
    
    /// 检查是否为EOF
    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }
}

/// 词法分析器
pub struct Lexer<'a> {
    /// 输入字符迭代器
    chars: Peekable<Chars<'a>>,
    /// 原始输入（用于错误报告）
    input: &'a str,
    /// 当前位置（字符偏移）
    offset: usize,
    /// 当前行号（从1开始）
    line: usize,
    /// 当前列号（从1开始）
    col: usize,
}

impl<'a> Lexer<'a> {
    /// 创建新的词法分析器
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            input,
            offset: 0,
            line: 1,
            col: 1,
        }
    }
    
    /// 获取原始输入
    pub fn input(&self) -> &str {
        self.input
    }
    
    /// 查看下一个字符（不消费）
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }
    
    /// 消费下一个字符
    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        self.offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }
    
    /// 创建当前位置的Span
    fn current_span(&self) -> Span {
        Span::single(self.line, self.col, self.offset)
    }
    
    /// 跳过空白字符和注释
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // 跳过空白
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            
            // 跳过注释（; 开头直到行末）
            if self.peek() == Some(';') {
                while let Some(ch) = self.advance() {
                    if ch == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }
    
    /// 解析数字
    fn scan_number(&mut self, start_line: usize, start_col: usize, start_offset: usize) -> SExprResult<Token> {
        let mut text = String::new();
        
        // 可选的负号
        if self.peek() == Some('-') {
            text.push(self.advance().unwrap());
        }
        
        // 整数部分
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                text.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        
        // 小数部分
        if self.peek() == Some('.') {
            text.push(self.advance().unwrap());
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    text.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
        }
        
        // 科学计数法
        if let Some(ch) = self.peek() {
            if ch == 'e' || ch == 'E' {
                text.push(self.advance().unwrap());
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        text.push(self.advance().unwrap());
                    }
                }
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        text.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
            }
        }
        
        let span = Span::new(start_line, start_col, start_offset, text.len());
        
        text.parse::<f64>()
            .map(|n| Token::new(TokenKind::Number(n), span))
            .map_err(|_| SExprError::InvalidNumber { text, span })
    }
    
    /// 解析符号
    fn scan_symbol(&mut self, start_line: usize, start_col: usize, start_offset: usize) -> Token {
        let mut text = String::new();
        
        while let Some(ch) = self.peek() {
            if is_symbol_char(ch) {
                text.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        
        let span = Span::new(start_line, start_col, start_offset, text.len());
        Token::new(TokenKind::Symbol(text), span)
    }
    
    /// 获取下一个Token
    pub fn next_token(&mut self) -> SExprResult<Token> {
        self.skip_whitespace_and_comments();
        
        let start_line = self.line;
        let start_col = self.col;
        let start_offset = self.offset;
        
        match self.peek() {
            None => Ok(Token::new(TokenKind::Eof, self.current_span())),
            
            Some('(') => {
                self.advance();
                Ok(Token::new(TokenKind::LParen, Span::single(start_line, start_col, start_offset)))
            }
            
            Some(')') => {
                self.advance();
                Ok(Token::new(TokenKind::RParen, Span::single(start_line, start_col, start_offset)))
            }
            
            Some(':') => {
                self.advance();
                // :keyword 形式
                if let Some(ch) = self.peek() {
                    if is_symbol_start(ch) {
                        let sym = self.scan_symbol(start_line, start_col + 1, start_offset + 1);
                        if let TokenKind::Symbol(s) = sym.kind {
                            let full_span = Span::new(start_line, start_col, start_offset, s.len() + 1);
                            return Ok(Token::new(TokenKind::Symbol(format!(":{}", s)), full_span));
                        }
                    }
                }
                Ok(Token::new(TokenKind::Colon, Span::single(start_line, start_col, start_offset)))
            }
            
            Some(ch) if ch.is_ascii_digit() => {
                self.scan_number(start_line, start_col, start_offset)
            }
            
            Some('-') => {
                // 可能是负数或减号符号
                let next_offset = start_offset + 1;
                if next_offset < self.input.len() {
                    let next_char = self.input[next_offset..].chars().next();
                    if let Some(nc) = next_char {
                        if nc.is_ascii_digit() || nc == '.' {
                            return self.scan_number(start_line, start_col, start_offset);
                        }
                    }
                }
                // 作为符号处理
                self.scan_symbol(start_line, start_col, start_offset);
                Ok(Token::new(TokenKind::Symbol("-".to_string()), Span::single(start_line, start_col, start_offset)))
            }
            
            Some(ch) if is_symbol_start(ch) => {
                Ok(self.scan_symbol(start_line, start_col, start_offset))
            }
            
            Some(ch) => {
                let span = Span::single(start_line, start_col, start_offset);
                Err(SExprError::UnexpectedChar { ch, span })
            }
        }
    }
    
    /// 收集所有Token
    pub fn tokenize(&mut self) -> SExprResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.is_eof();
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

/// 检查字符是否可以作为符号的开始
fn is_symbol_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_' || ch == '+' || ch == '*' || ch == '/' 
        || ch == '<' || ch == '>' || ch == '=' || ch == '!' || ch == '?' || ch == '&' || ch == '|'
}

/// 检查字符是否可以作为符号的一部分
fn is_symbol_char(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit() || ch == '-' || ch == '.'
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("(add 1 2)");
        
        let t1 = lexer.next_token().unwrap();
        assert!(matches!(t1.kind, TokenKind::LParen));
        
        let t2 = lexer.next_token().unwrap();
        assert!(matches!(t2.kind, TokenKind::Symbol(ref s) if s == "add"));
        
        let t3 = lexer.next_token().unwrap();
        assert!(matches!(t3.kind, TokenKind::Number(n) if (n - 1.0).abs() < 1e-10));
        
        let t4 = lexer.next_token().unwrap();
        assert!(matches!(t4.kind, TokenKind::Number(n) if (n - 2.0).abs() < 1e-10));
        
        let t5 = lexer.next_token().unwrap();
        assert!(matches!(t5.kind, TokenKind::RParen));
        
        let t6 = lexer.next_token().unwrap();
        assert!(matches!(t6.kind, TokenKind::Eof));
    }
    
    #[test]
    fn test_nested_expression() {
        let mut lexer = Lexer::new("(mul (add x 1) y)");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 10); // ( mul ( add x 1 ) y ) EOF
    }
    
    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 -5 1e10 2.5e-3");
        let tokens = lexer.tokenize().unwrap();
        
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 42.0).abs() < 1e-10));
        #[allow(clippy::approx_constant)]
        let expected_pi_approx = 3.14;
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if (n - expected_pi_approx).abs() < 1e-10));
        assert!(matches!(tokens[2].kind, TokenKind::Number(n) if (n - (-5.0)).abs() < 1e-10));
        assert!(matches!(tokens[3].kind, TokenKind::Number(n) if (n - 1e10).abs() < 1e-10));
        assert!(matches!(tokens[4].kind, TokenKind::Number(n) if (n - 2.5e-3).abs() < 1e-10));
    }
    
    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("; this is a comment\n(add 1 2)");
        let tokens = lexer.tokenize().unwrap();
        
        // 注释被跳过
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
    }
    
    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new(":otherwise :else");
        let tokens = lexer.tokenize().unwrap();
        
        assert!(matches!(tokens[0].kind, TokenKind::Symbol(ref s) if s == ":otherwise"));
        assert!(matches!(tokens[1].kind, TokenKind::Symbol(ref s) if s == ":else"));
    }
    
    #[test]
    fn test_special_symbols() {
        let mut lexer = Lexer::new("pi e x_1 param_name");
        let tokens = lexer.tokenize().unwrap();
        
        assert!(matches!(tokens[0].kind, TokenKind::Symbol(ref s) if s == "pi"));
        assert!(matches!(tokens[1].kind, TokenKind::Symbol(ref s) if s == "e"));
        assert!(matches!(tokens[2].kind, TokenKind::Symbol(ref s) if s == "x_1"));
        assert!(matches!(tokens[3].kind, TokenKind::Symbol(ref s) if s == "param_name"));
    }
    
    #[test]
    fn test_position_tracking() {
        let mut lexer = Lexer::new("(add\n  x\n  y)");
        
        let t1 = lexer.next_token().unwrap(); // (
        assert_eq!(t1.span.line, 1);
        assert_eq!(t1.span.col, 1);
        
        let t2 = lexer.next_token().unwrap(); // add
        assert_eq!(t2.span.line, 1);
        assert_eq!(t2.span.col, 2);
        
        let t3 = lexer.next_token().unwrap(); // x
        assert_eq!(t3.span.line, 2);
        assert_eq!(t3.span.col, 3);
        
        let t4 = lexer.next_token().unwrap(); // y
        assert_eq!(t4.span.line, 3);
        assert_eq!(t4.span.col, 3);
    }
}
