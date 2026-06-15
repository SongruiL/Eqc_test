//! S表达式解析错误类型
//!
//! 本模块定义了S表达式解析过程中可能出现的所有错误类型。

use std::fmt;
use thiserror::Error;

/// S表达式解析结果类型
pub type SExprResult<T> = Result<T, SExprError>;

/// 源代码位置信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 起始行号（从1开始）
    pub line: usize,
    /// 起始列号（从1开始）
    pub col: usize,
    /// 字符偏移量（从0开始）
    pub offset: usize,
    /// 长度（字符数）
    pub len: usize,
}

impl Span {
    /// 创建新的位置信息
    pub fn new(line: usize, col: usize, offset: usize, len: usize) -> Self {
        Self { line, col, offset, len }
    }
    
    /// 创建单字符位置
    pub fn single(line: usize, col: usize, offset: usize) -> Self {
        Self { line, col, offset, len: 1 }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// S表达式解析错误
#[derive(Debug, Error)]
pub enum SExprError {
    // ========== 词法错误 ==========
    
    /// 遇到意外字符
    #[error("意外字符 '{ch}' 位于 {span}")]
    UnexpectedChar {
        ch: char,
        span: Span,
    },
    
    /// 无效的数字格式
    #[error("无效的数字格式 '{text}' 位于 {span}")]
    InvalidNumber {
        text: String,
        span: Span,
    },
    
    /// 未终止的字符串
    #[error("未终止的字符串 位于 {span}")]
    UnterminatedString {
        span: Span,
    },
    
    // ========== 语法错误 ==========
    
    /// 意外的Token
    #[error("期望 {expected}，但发现 {found} 位于 {span}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },
    
    /// 未匹配的括号
    #[error("未匹配的括号 '{paren}' 位于 {span}")]
    UnmatchedParen {
        paren: char,
        span: Span,
    },
    
    /// 空表达式
    #[error("空表达式 位于 {span}")]
    EmptyExpression {
        span: Span,
    },
    
    /// 意外的文件结束
    #[error("意外的文件结束")]
    UnexpectedEof,
    
    // ========== 转换错误 ==========
    
    /// 未知运算符
    #[error("未知运算符 '{op}'")]
    UnknownOperator {
        op: String,
        span: Option<Span>,
    },
    
    /// 参数数量错误
    #[error("运算符 '{op}' 需要 {expected} 个参数，但提供了 {found} 个")]
    WrongArgCount {
        op: String,
        expected: String,
        found: usize,
    },
    
    /// 无效的if语法
    #[error("无效的 if 语法：需要 (if condition then else)")]
    InvalidIfSyntax,
    
    /// 无效的sum语法
    #[error("无效的 sum 语法：需要 (sum index lower upper body)")]
    InvalidSumSyntax,
    
    /// 无效的product语法
    #[error("无效的 product 语法：需要 (product index lower upper body)")]
    InvalidProductSyntax,
    
    /// 无效的piecewise语法
    #[error("无效的 piecewise 语法：需要 (piecewise (cond1 val1) ... :otherwise default)")]
    InvalidPiecewiseSyntax,
    
    /// 无效的lambda语法
    #[error("无效的 lambda 语法：需要 (lambda (params...) body)")]
    InvalidLambdaSyntax,
    
    /// 期望符号
    #[error("期望符号，但发现 {found}")]
    ExpectedSymbol {
        found: String,
    },
    
    /// 期望列表
    #[error("期望列表，但发现 {found}")]
    ExpectedList {
        found: String,
    },
    
    /// 期望数字
    #[error("期望数字，但发现 {found}")]
    ExpectedNumber {
        found: String,
    },
}

impl SExprError {
    /// 创建未知运算符错误，并提供相似运算符建议
    pub fn unknown_operator_with_suggestion(op: &str, known_ops: &[&str]) -> Self {
        // 简单的编辑距离相似度检查
        let _suggestions: Vec<_> = known_ops
            .iter()
            .filter(|&&known| {
                let dist = levenshtein_distance(op, known);
                dist <= 2 || known.contains(op) || op.contains(known)
            })
            .take(3)
            .copied()
            .collect();
        
        Self::UnknownOperator {
            op: op.to_string(),
            span: None,
        }
    }
}

/// 计算两个字符串的Levenshtein编辑距离
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    
    if m == 0 { return n; }
    if n == 0 { return m; }
    
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate().take(n + 1) {
        *val = j;
    }
    
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    
    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_display() {
        let span = Span::new(3, 5, 20, 4);
        assert_eq!(format!("{}", span), "3:5");
    }
    
    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("sin", "cos"), 3); // s->c, i->o, n->s
        // floor -> foobar: delete l, change o->b, insert a, change r (or similar)
        assert_eq!(levenshtein_distance("floor", "foobar"), 3);
    }
}
