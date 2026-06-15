//! S表达式AST定义
//!
//! 定义S表达式的抽象语法树结构。

use std::fmt;

/// S表达式AST节点
///
/// S表达式是一种简单的树形结构，只有三种节点类型：
/// - 数字（叶节点）
/// - 符号（叶节点）
/// - 列表（内部节点，包含子表达式）
#[derive(Debug, Clone, PartialEq)]
pub enum SExpr {
    /// 数值常量
    ///
    /// 例如: `42`, `3.14`, `-5`, `1e10`
    Number(f64),
    
    /// 符号
    ///
    /// 包括运算符名、变量名、参数名、关键字等
    /// 例如: `add`, `x`, `param_1`, `:otherwise`
    Symbol(String),
    
    /// 列表（复合表达式）
    ///
    /// 表示函数调用或特殊形式
    /// 例如: `(add 1 2)`, `(if cond then else)`
    List(Vec<SExpr>),
}

impl SExpr {
    /// 创建数字节点
    pub fn number(n: f64) -> Self {
        SExpr::Number(n)
    }
    
    /// 创建符号节点
    pub fn symbol(s: impl Into<String>) -> Self {
        SExpr::Symbol(s.into())
    }
    
    /// 创建列表节点
    pub fn list(items: Vec<SExpr>) -> Self {
        SExpr::List(items)
    }
    
    /// 检查是否为数字
    pub fn is_number(&self) -> bool {
        matches!(self, SExpr::Number(_))
    }
    
    /// 检查是否为符号
    pub fn is_symbol(&self) -> bool {
        matches!(self, SExpr::Symbol(_))
    }
    
    /// 检查是否为列表
    pub fn is_list(&self) -> bool {
        matches!(self, SExpr::List(_))
    }
    
    /// 获取数字值
    pub fn as_number(&self) -> Option<f64> {
        match self {
            SExpr::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    /// 获取符号值
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            SExpr::Symbol(s) => Some(s),
            _ => None,
        }
    }
    
    /// 获取列表
    pub fn as_list(&self) -> Option<&[SExpr]> {
        match self {
            SExpr::List(items) => Some(items),
            _ => None,
        }
    }
    
    /// 获取可变列表
    pub fn as_list_mut(&mut self) -> Option<&mut Vec<SExpr>> {
        match self {
            SExpr::List(items) => Some(items),
            _ => None,
        }
    }
    
    /// 检查是否为关键字（以冒号开头的符号）
    pub fn is_keyword(&self) -> bool {
        match self {
            SExpr::Symbol(s) => s.starts_with(':'),
            _ => false,
        }
    }
    
    /// 获取关键字名称（去掉冒号）
    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            SExpr::Symbol(s) if s.starts_with(':') => Some(&s[1..]),
            _ => None,
        }
    }
    
    /// 获取类型描述（用于错误消息）
    pub fn type_name(&self) -> &'static str {
        match self {
            SExpr::Number(_) => "数字",
            SExpr::Symbol(_) => "符号",
            SExpr::List(_) => "列表",
        }
    }
}

impl fmt::Display for SExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SExpr::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            SExpr::Symbol(s) => write!(f, "{}", s),
            SExpr::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// 从字符串解析SExpr（便捷宏）
#[macro_export]
macro_rules! sexpr {
    // 数字
    ($n:literal) => {
        $crate::sexpr::SExpr::Number($n as f64)
    };
    
    // 符号
    ($s:ident) => {
        $crate::sexpr::SExpr::Symbol(stringify!($s).to_string())
    };
    
    // 列表
    (($($item:tt)*)) => {
        $crate::sexpr::SExpr::List(vec![$(sexpr!($item)),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_number() {
        let n = SExpr::number(42.0);
        assert!(n.is_number());
        assert!(!n.is_symbol());
        assert!(!n.is_list());
        assert_eq!(n.as_number(), Some(42.0));
        assert_eq!(format!("{}", n), "42");
    }
    
    #[test]
    fn test_symbol() {
        let s = SExpr::symbol("add");
        assert!(!s.is_number());
        assert!(s.is_symbol());
        assert!(!s.is_list());
        assert_eq!(s.as_symbol(), Some("add"));
        assert_eq!(format!("{}", s), "add");
    }
    
    #[test]
    fn test_keyword() {
        let k = SExpr::symbol(":otherwise");
        assert!(k.is_keyword());
        assert_eq!(k.as_keyword(), Some("otherwise"));
        
        let s = SExpr::symbol("otherwise");
        assert!(!s.is_keyword());
        assert_eq!(s.as_keyword(), None);
    }
    
    #[test]
    fn test_list() {
        let l = SExpr::list(vec![
            SExpr::symbol("add"),
            SExpr::number(1.0),
            SExpr::number(2.0),
        ]);
        assert!(!l.is_number());
        assert!(!l.is_symbol());
        assert!(l.is_list());
        assert_eq!(l.as_list().map(|v| v.len()), Some(3));
        assert_eq!(format!("{}", l), "(add 1 2)");
    }
    
    #[test]
    fn test_nested_list() {
        let l = SExpr::list(vec![
            SExpr::symbol("mul"),
            SExpr::list(vec![
                SExpr::symbol("add"),
                SExpr::symbol("x"),
                SExpr::number(1.0),
            ]),
            SExpr::symbol("y"),
        ]);
        assert_eq!(format!("{}", l), "(mul (add x 1) y)");
    }
    
    #[test]
    fn test_float_display() {
        #[allow(clippy::approx_constant)]
        let n1 = SExpr::number(3.14159);
        assert_eq!(format!("{}", n1), "3.14159");
        
        let n2 = SExpr::number(1e10);
        // 大整数格式
        assert!(format!("{}", n2).contains("10000000000"));
    }
}
