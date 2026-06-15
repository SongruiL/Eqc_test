//! S表达式语法分析器
//!
//! 将Token流解析为S表达式AST。

use super::ast::SExpr;
use super::error::{SExprError, SExprResult, Span};
use super::lexer::{Lexer, Token, TokenKind};

/// S表达式语法分析器
///
/// 使用递归下降方法解析S表达式。
pub struct Parser<'a> {
    /// 词法分析器
    lexer: Lexer<'a>,
    /// 当前Token
    current: Option<Token>,
    /// 预读的Token
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    /// 创建新的语法分析器
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer,
            current: None,
            peeked: None,
        }
    }
    
    /// 从字符串创建解析器
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'a str) -> Self {
        Self::new(Lexer::new(input))
    }
    
    /// 获取下一个Token
    fn advance(&mut self) -> SExprResult<Token> {
        if let Some(token) = self.peeked.take() {
            self.current = Some(token.clone());
            return Ok(token);
        }
        
        let token = self.lexer.next_token()?;
        self.current = Some(token.clone());
        Ok(token)
    }
    
    /// 查看下一个Token（不消费）
    fn peek(&mut self) -> SExprResult<&Token> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }
    
    /// 检查下一个Token是否为指定类型
    #[allow(dead_code)]
    fn check(&mut self, kind: &TokenKind) -> SExprResult<bool> {
        Ok(self.peek()?.is(kind))
    }
    
    /// 期望并消费特定类型的Token
    #[allow(dead_code)]
    fn expect(&mut self, expected: &TokenKind) -> SExprResult<Token> {
        let token = self.advance()?;
        if token.is(expected) {
            Ok(token)
        } else {
            Err(SExprError::UnexpectedToken {
                expected: format!("{}", expected),
                found: format!("{}", token.kind),
                span: token.span,
            })
        }
    }
    
    /// 解析完整的S表达式
    ///
    /// 入口点，解析单个完整的表达式
    pub fn parse(&mut self) -> SExprResult<SExpr> {
        let expr = self.parse_expr()?;
        
        // 检查是否有剩余内容
        let next = self.peek()?;
        if !next.is_eof() {
            return Err(SExprError::UnexpectedToken {
                expected: "文件结束".to_string(),
                found: format!("{}", next.kind),
                span: next.span,
            });
        }
        
        Ok(expr)
    }
    
    /// 解析多个表达式
    ///
    /// 解析文件中的所有顶层表达式
    pub fn parse_all(&mut self) -> SExprResult<Vec<SExpr>> {
        let mut exprs = Vec::new();
        
        loop {
            let token = self.peek()?;
            if token.is_eof() {
                break;
            }
            exprs.push(self.parse_expr()?);
        }
        
        Ok(exprs)
    }
    
    /// 解析单个表达式
    fn parse_expr(&mut self) -> SExprResult<SExpr> {
        let token = self.advance()?;
        
        match token.kind {
            TokenKind::Number(n) => Ok(SExpr::Number(n)),
            
            TokenKind::Symbol(s) => Ok(SExpr::Symbol(s)),
            
            TokenKind::LParen => self.parse_list(token.span),
            
            TokenKind::RParen => Err(SExprError::UnmatchedParen {
                paren: ')',
                span: token.span,
            }),
            
            TokenKind::Colon => {
                // 独立的冒号，可能是语法错误
                Err(SExprError::UnexpectedToken {
                    expected: "表达式".to_string(),
                    found: "':'".to_string(),
                    span: token.span,
                })
            }
            
            TokenKind::Eof => Err(SExprError::UnexpectedEof),
        }
    }
    
    /// 解析列表
    fn parse_list(&mut self, open_paren_span: Span) -> SExprResult<SExpr> {
        let mut items = Vec::new();
        
        loop {
            let token = self.peek()?;
            
            match &token.kind {
                TokenKind::RParen => {
                    self.advance()?; // 消费 )
                    return Ok(SExpr::List(items));
                }
                
                TokenKind::Eof => {
                    return Err(SExprError::UnmatchedParen {
                        paren: '(',
                        span: open_paren_span,
                    });
                }
                
                _ => {
                    items.push(self.parse_expr()?);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn parse(input: &str) -> SExprResult<SExpr> {
        Parser::from_str(input).parse()
    }
    
    #[test]
    fn test_number() {
        let expr = parse("42").unwrap();
        assert_eq!(expr, SExpr::Number(42.0));
    }
    
    #[test]
    fn test_symbol() {
        let expr = parse("add").unwrap();
        assert_eq!(expr, SExpr::Symbol("add".to_string()));
    }
    
    #[test]
    fn test_simple_list() {
        let expr = parse("(add 1 2)").unwrap();
        assert_eq!(expr, SExpr::List(vec![
            SExpr::Symbol("add".to_string()),
            SExpr::Number(1.0),
            SExpr::Number(2.0),
        ]));
    }
    
    #[test]
    fn test_nested_list() {
        let expr = parse("(mul (add x 1) y)").unwrap();
        assert_eq!(expr, SExpr::List(vec![
            SExpr::Symbol("mul".to_string()),
            SExpr::List(vec![
                SExpr::Symbol("add".to_string()),
                SExpr::Symbol("x".to_string()),
                SExpr::Number(1.0),
            ]),
            SExpr::Symbol("y".to_string()),
        ]));
    }
    
    #[test]
    fn test_empty_list() {
        let expr = parse("()").unwrap();
        assert_eq!(expr, SExpr::List(vec![]));
    }
    
    #[test]
    fn test_keyword() {
        let expr = parse("(piecewise (cond1 val1) :otherwise default)").unwrap();
        if let SExpr::List(items) = expr {
            // piecewise, (cond1 val1), :otherwise, default
            assert_eq!(items.len(), 4);
            assert_eq!(items[2], SExpr::Symbol(":otherwise".to_string()));
        } else {
            panic!("Expected list");
        }
    }
    
    #[test]
    fn test_with_comments() {
        let expr = parse("; comment\n(add 1 2)").unwrap();
        assert_eq!(expr, SExpr::List(vec![
            SExpr::Symbol("add".to_string()),
            SExpr::Number(1.0),
            SExpr::Number(2.0),
        ]));
    }
    
    #[test]
    fn test_multiline() {
        let expr = parse("(add\n  1\n  2)").unwrap();
        assert_eq!(expr, SExpr::List(vec![
            SExpr::Symbol("add".to_string()),
            SExpr::Number(1.0),
            SExpr::Number(2.0),
        ]));
    }
    
    #[test]
    fn test_unmatched_open_paren() {
        let result = parse("(add 1 2");
        assert!(result.is_err());
        if let Err(SExprError::UnmatchedParen { paren, .. }) = result {
            assert_eq!(paren, '(');
        } else {
            panic!("Expected UnmatchedParen error");
        }
    }
    
    #[test]
    fn test_unmatched_close_paren() {
        let result = parse("add 1 2)");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_all() {
        let mut parser = Parser::from_str("(add 1 2) (mul 3 4)");
        let exprs = parser.parse_all().unwrap();
        assert_eq!(exprs.len(), 2);
    }
    
    #[test]
    fn test_complex_expression() {
        let input = r#"
            ; 复杂表达式测试
            (if (gt x 0)
                (sqrt x)
                (neg (sqrt (neg x))))
        "#;
        let expr = parse(input).unwrap();
        
        if let SExpr::List(items) = &expr {
            assert_eq!(items.len(), 4);
            assert_eq!(items[0].as_symbol(), Some("if"));
        } else {
            panic!("Expected list");
        }
    }
    
    #[test]
    fn test_scientific_notation() {
        let expr = parse("(mul 1.5e-3 2e10)").unwrap();
        if let SExpr::List(items) = expr {
            assert!(matches!(items[1], SExpr::Number(n) if (n - 1.5e-3).abs() < 1e-15));
            assert!(matches!(items[2], SExpr::Number(n) if (n - 2e10).abs() < 1e-5));
        }
    }
}
