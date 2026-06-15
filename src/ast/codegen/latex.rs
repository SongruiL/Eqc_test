//! LaTeX 代码生成
//!
//! 将 Expr 表达式转换为 LaTeX 数学公式

/// LaTeX 代码生成 trait
pub trait ToLatex {
    /// 转换为 LaTeX 代码
    ///
    /// # 返回
    /// LaTeX 代码字符串
    fn to_latex(&self) -> String;
}

// 注意：ToLatex trait 的实现保留在 expr.rs 中
// 这里只定义 trait 接口
