//! Rust 代码生成
//!
//! 将 Expr 表达式转换为 Rust 代码

/// Rust 代码生成 trait
pub trait ToRust {
    /// 转换为 Rust 代码
    ///
    /// # 返回
    /// Rust 代码字符串
    fn to_rust(&self) -> String;
}

// 注意：ToRust trait 的实现保留在 expr.rs 中
// 这里只定义 trait 接口
