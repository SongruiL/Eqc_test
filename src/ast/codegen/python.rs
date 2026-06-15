//! Python 代码生成
//!
//! 将 Expr 表达式转换为 Python 代码（使用 NumPy 和 SciPy）

/// Python 代码生成 trait
pub trait ToPython {
    /// 转换为 Python 代码
    ///
    /// # 参数
    /// - `params_prefix`: 参数前缀（如 "params"）
    ///
    /// # 返回
    /// Python 代码字符串
    fn to_python(&self, params_prefix: &str) -> String;
}

// 注意：ToPython trait 的实现保留在 expr.rs 中
// 这里只定义 trait 接口，实现通过 impl ToPython for Expr 在 expr.rs 完成
