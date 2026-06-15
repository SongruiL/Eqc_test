//! AST 访问者模式（强类型版本）
//!
//! 提供遍历和转换 AST 的标准接口。

#![allow(dead_code)]

use super::Expr;

/// 表达式访问者 trait（强类型版本）
///
/// 使用访问者模式遍历 AST，实现不同的处理逻辑。
/// 每个节点类型都有对应的 visit 方法。
pub trait ExprVisitor {
    /// 输出类型
    type Output;

    // === 叶子节点 ===

    /// 访问常量节点
    fn visit_const(&mut self, value: f64) -> Self::Output;

    /// 访问变量引用节点
    fn visit_var(&mut self, name: &str) -> Self::Output;

    /// 访问参数引用节点
    fn visit_param(&mut self, name: &str) -> Self::Output;

    /// 访问 π 常量
    fn visit_pi(&mut self) -> Self::Output;

    /// 访问 e 常量
    fn visit_e(&mut self) -> Self::Output;

    // === 算术运算 ===

    /// 访问加法节点
    fn visit_add(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问减法节点
    fn visit_sub(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问乘法节点
    fn visit_mul(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问除法节点
    fn visit_div(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问取负节点
    fn visit_neg(&mut self, arg: &Expr) -> Self::Output;

    /// 访问幂运算节点
    fn visit_pow(&mut self, base: &Expr, exp: &Expr) -> Self::Output;

    /// 访问绝对值节点
    fn visit_abs(&mut self, arg: &Expr) -> Self::Output;

    /// 访问取余节点
    fn visit_mod(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问向上取整节点
    fn visit_ceil(&mut self, arg: &Expr) -> Self::Output;

    /// 访问向下取整节点
    fn visit_floor(&mut self, arg: &Expr) -> Self::Output;

    /// 访问四舍五入节点
    fn visit_round(&mut self, arg: &Expr) -> Self::Output;

    /// 访问截断取整节点
    fn visit_trunc(&mut self, arg: &Expr) -> Self::Output;

    /// 访问符号函数节点
    fn visit_sign(&mut self, arg: &Expr) -> Self::Output;

    // === 超越函数 ===

    /// 访问指数函数节点
    fn visit_exp(&mut self, arg: &Expr) -> Self::Output;

    /// 访问自然对数节点
    fn visit_ln(&mut self, arg: &Expr) -> Self::Output;

    /// 访问常用对数节点
    fn visit_log10(&mut self, arg: &Expr) -> Self::Output;

    /// 访问以2为底对数节点
    fn visit_log2(&mut self, arg: &Expr) -> Self::Output;

    /// 访问平方根节点
    fn visit_sqrt(&mut self, arg: &Expr) -> Self::Output;

    /// 访问立方根节点
    fn visit_cbrt(&mut self, arg: &Expr) -> Self::Output;

    // === 三角函数 ===

    /// 访问正弦节点
    fn visit_sin(&mut self, arg: &Expr) -> Self::Output;

    /// 访问余弦节点
    fn visit_cos(&mut self, arg: &Expr) -> Self::Output;

    /// 访问正切节点
    fn visit_tan(&mut self, arg: &Expr) -> Self::Output;

    /// 访问反正弦节点
    fn visit_asin(&mut self, arg: &Expr) -> Self::Output;

    /// 访问反余弦节点
    fn visit_acos(&mut self, arg: &Expr) -> Self::Output;

    /// 访问反正切节点
    fn visit_atan(&mut self, arg: &Expr) -> Self::Output;

    /// 访问二参数反正切节点
    fn visit_atan2(&mut self, y: &Expr, x: &Expr) -> Self::Output;

    // === 双曲函数 ===

    /// 访问双曲正弦节点
    fn visit_sinh(&mut self, arg: &Expr) -> Self::Output;

    /// 访问双曲余弦节点
    fn visit_cosh(&mut self, arg: &Expr) -> Self::Output;

    /// 访问双曲正切节点
    fn visit_tanh(&mut self, arg: &Expr) -> Self::Output;

    /// 访问反双曲正弦节点
    fn visit_asinh(&mut self, arg: &Expr) -> Self::Output;

    /// 访问反双曲余弦节点
    fn visit_acosh(&mut self, arg: &Expr) -> Self::Output;

    /// 访问反双曲正切节点
    fn visit_atanh(&mut self, arg: &Expr) -> Self::Output;

    // === 聚合函数 ===

    /// 访问最大值节点
    fn visit_max(&mut self, args: &[Expr]) -> Self::Output;

    /// 访问最小值节点
    fn visit_min(&mut self, args: &[Expr]) -> Self::Output;

    /// 访问求和节点
    fn visit_sum(&mut self, index: &str, lower: &Expr, upper: &Expr, body: &Expr) -> Self::Output;

    /// 访问连乘节点
    fn visit_product(&mut self, index: &str, lower: &Expr, upper: &Expr, body: &Expr) -> Self::Output;

    // === 关系运算 ===

    /// 访问等于节点
    fn visit_eq(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问小于节点
    fn visit_lt(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问大于节点
    fn visit_gt(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问小于等于节点
    fn visit_leq(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问大于等于节点
    fn visit_geq(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问不等于节点
    fn visit_neq(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    // === 逻辑运算 ===

    /// 访问逻辑与节点
    fn visit_and(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问逻辑或节点
    fn visit_or(&mut self, left: &Expr, right: &Expr) -> Self::Output;

    /// 访问逻辑非节点
    fn visit_not(&mut self, arg: &Expr) -> Self::Output;

    // === 条件表达式 ===

    /// 访问条件表达式节点
    fn visit_if_then_else(
        &mut self,
        cond: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
    ) -> Self::Output;

    /// 访问分段函数节点
    fn visit_piecewise(&mut self, pieces: &[(Expr, Expr)], otherwise: &Expr) -> Self::Output;
}

/// 遍历 AST
pub fn walk_expr<V: ExprVisitor>(visitor: &mut V, expr: &Expr) -> V::Output {
    match expr {
        // 叶子节点
        Expr::Const(value) => visitor.visit_const(*value),
        Expr::Var(name) => visitor.visit_var(name),
        Expr::Param(name) => visitor.visit_param(name),
        Expr::Pi => visitor.visit_pi(),
        Expr::E => visitor.visit_e(),

        // 算术运算
        Expr::Add(a, b) => visitor.visit_add(a, b),
        Expr::Sub(a, b) => visitor.visit_sub(a, b),
        Expr::Mul(a, b) => visitor.visit_mul(a, b),
        Expr::Div(a, b) => visitor.visit_div(a, b),
        Expr::Neg(a) => visitor.visit_neg(a),
        Expr::Pow(a, b) => visitor.visit_pow(a, b),
        Expr::Abs(a) => visitor.visit_abs(a),
        Expr::Mod(a, b) => visitor.visit_mod(a, b),
        Expr::Ceil(a) => visitor.visit_ceil(a),
        Expr::Floor(a) => visitor.visit_floor(a),
        Expr::Round(a) => visitor.visit_round(a),
        Expr::Trunc(a) => visitor.visit_trunc(a),
        Expr::Sign(a) => visitor.visit_sign(a),

        // 超越函数
        Expr::Exp(a) => visitor.visit_exp(a),
        Expr::Ln(a) => visitor.visit_ln(a),
        Expr::Log10(a) => visitor.visit_log10(a),
        Expr::Log2(a) => visitor.visit_log2(a),
        Expr::Sqrt(a) => visitor.visit_sqrt(a),
        Expr::Cbrt(a) => visitor.visit_cbrt(a),

        // 三角函数
        Expr::Sin(a) => visitor.visit_sin(a),
        Expr::Cos(a) => visitor.visit_cos(a),
        Expr::Tan(a) => visitor.visit_tan(a),
        Expr::ASin(a) => visitor.visit_asin(a),
        Expr::ACos(a) => visitor.visit_acos(a),
        Expr::ATan(a) => visitor.visit_atan(a),
        Expr::ATan2(y, x) => visitor.visit_atan2(y, x),

        // 双曲函数
        Expr::Sinh(a) => visitor.visit_sinh(a),
        Expr::Cosh(a) => visitor.visit_cosh(a),
        Expr::Tanh(a) => visitor.visit_tanh(a),
        Expr::ASinh(a) => visitor.visit_asinh(a),
        Expr::ACosh(a) => visitor.visit_acosh(a),
        Expr::ATanh(a) => visitor.visit_atanh(a),

        // 聚合函数
        Expr::Max(args) => visitor.visit_max(args),
        Expr::Min(args) => visitor.visit_min(args),
        Expr::Sum { index, lower, upper, body } => visitor.visit_sum(index, lower, upper, body),
        Expr::Product { index, lower, upper, body } => visitor.visit_product(index, lower, upper, body),

        // 关系运算
        Expr::Eq(a, b) => visitor.visit_eq(a, b),
        Expr::Lt(a, b) => visitor.visit_lt(a, b),
        Expr::Gt(a, b) => visitor.visit_gt(a, b),
        Expr::Leq(a, b) => visitor.visit_leq(a, b),
        Expr::Geq(a, b) => visitor.visit_geq(a, b),
        Expr::Neq(a, b) => visitor.visit_neq(a, b),

        // 逻辑运算
        Expr::And(a, b) => visitor.visit_and(a, b),
        Expr::Or(a, b) => visitor.visit_or(a, b),
        Expr::Not(a) => visitor.visit_not(a),

        // 条件表达式
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            visitor.visit_if_then_else(cond, then_branch, else_branch)
        }
        Expr::Piecewise { pieces, otherwise } => visitor.visit_piecewise(pieces, otherwise),

        // 扩展运算符 - 使用默认处理
        _ => visitor.visit_const(0.0), // 占位：扩展运算符暂时返回默认值
    }
}

// ============================================
// 默认实现的基类 trait
// ============================================

/// 提供默认递归遍历的访问者基类
///
/// 继承此 trait 可以只覆盖需要的方法，其他方法会自动递归遍历子节点。
pub trait DefaultVisitor {
    /// 默认处理：不做任何操作
    fn default_result(&mut self) {}

    /// 递归遍历子表达式
    fn visit_children(&mut self, expr: &Expr) {
        match expr {
            Expr::Const(_) | Expr::Var(_) | Expr::Param(_) | Expr::Pi | Expr::E => {}

            Expr::Neg(a)
            | Expr::Abs(a)
            | Expr::Ceil(a)
            | Expr::Floor(a)
            | Expr::Round(a)
            | Expr::Trunc(a)
            | Expr::Sign(a)
            | Expr::Exp(a)
            | Expr::Ln(a)
            | Expr::Log10(a)
            | Expr::Log2(a)
            | Expr::Sqrt(a)
            | Expr::Cbrt(a)
            | Expr::Sin(a)
            | Expr::Cos(a)
            | Expr::Tan(a)
            | Expr::ASin(a)
            | Expr::ACos(a)
            | Expr::ATan(a)
            | Expr::Sinh(a)
            | Expr::Cosh(a)
            | Expr::Tanh(a)
            | Expr::ASinh(a)
            | Expr::ACosh(a)
            | Expr::ATanh(a)
            | Expr::Not(a) => {
                self.visit_children(a);
            }

            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b)
            | Expr::Mod(a, b)
            | Expr::ATan2(a, b)
            | Expr::Eq(a, b)
            | Expr::Lt(a, b)
            | Expr::Gt(a, b)
            | Expr::Leq(a, b)
            | Expr::Geq(a, b)
            | Expr::Neq(a, b)
            | Expr::And(a, b)
            | Expr::Or(a, b) => {
                self.visit_children(a);
                self.visit_children(b);
            }

            Expr::Max(args) | Expr::Min(args) => {
                for arg in args {
                    self.visit_children(arg);
                }
            }

            Expr::Sum { lower, upper, body, .. }
            | Expr::Product { lower, upper, body, .. } => {
                self.visit_children(lower);
                self.visit_children(upper);
                self.visit_children(body);
            }

            Expr::IfThenElse { cond, then_branch, else_branch } => {
                self.visit_children(cond);
                self.visit_children(then_branch);
                self.visit_children(else_branch);
            }

            Expr::Piecewise { pieces, otherwise } => {
                for (cond, value) in pieces {
                    self.visit_children(cond);
                    self.visit_children(value);
                }
                self.visit_children(otherwise);
            }

            // 扩展运算符 - 递归遍历
            _ => {
                // 扩展运算符由 expr.rs 中的 collect_refs/depth 处理
            }
        }
    }
}

// ============================================
// 实用访问者实现
// ============================================

/// 引用收集器
///
/// 收集表达式中所有的变量和参数引用。
pub struct RefCollector {
    /// 变量引用列表
    pub vars: Vec<String>,
    /// 参数引用列表
    pub params: Vec<String>,
}

impl RefCollector {
    /// 创建新的收集器
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            params: Vec::new(),
        }
    }

    /// 收集表达式中的所有引用
    pub fn collect(expr: &Expr) -> Self {
        let mut collector = Self::new();
        collector.visit(expr);
        collector
    }

    fn visit(&mut self, expr: &Expr) {
        match expr {
            Expr::Var(name) => {
                if !self.vars.contains(name) {
                    self.vars.push(name.clone());
                }
            }
            Expr::Param(name) => {
                if !self.params.contains(name) {
                    self.params.push(name.clone());
                }
            }
            _ => {
                self.visit_children(expr);
            }
        }
    }

    fn visit_children(&mut self, expr: &Expr) {
        match expr {
            Expr::Const(_) | Expr::Var(_) | Expr::Param(_) | Expr::Pi | Expr::E => {}

            Expr::Neg(a)
            | Expr::Abs(a)
            | Expr::Ceil(a)
            | Expr::Floor(a)
            | Expr::Round(a)
            | Expr::Trunc(a)
            | Expr::Sign(a)
            | Expr::Exp(a)
            | Expr::Ln(a)
            | Expr::Log10(a)
            | Expr::Log2(a)
            | Expr::Sqrt(a)
            | Expr::Cbrt(a)
            | Expr::Sin(a)
            | Expr::Cos(a)
            | Expr::Tan(a)
            | Expr::ASin(a)
            | Expr::ACos(a)
            | Expr::ATan(a)
            | Expr::Sinh(a)
            | Expr::Cosh(a)
            | Expr::Tanh(a)
            | Expr::ASinh(a)
            | Expr::ACosh(a)
            | Expr::ATanh(a)
            | Expr::Not(a) => {
                self.visit(a);
            }

            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b)
            | Expr::Mod(a, b)
            | Expr::ATan2(a, b)
            | Expr::Eq(a, b)
            | Expr::Lt(a, b)
            | Expr::Gt(a, b)
            | Expr::Leq(a, b)
            | Expr::Geq(a, b)
            | Expr::Neq(a, b)
            | Expr::And(a, b)
            | Expr::Or(a, b) => {
                self.visit(a);
                self.visit(b);
            }

            Expr::Max(args) | Expr::Min(args) => {
                for arg in args {
                    self.visit(arg);
                }
            }

            Expr::Sum { lower, upper, body, .. }
            | Expr::Product { lower, upper, body, .. } => {
                self.visit(lower);
                self.visit(upper);
                self.visit(body);
            }

            Expr::IfThenElse { cond, then_branch, else_branch } => {
                self.visit(cond);
                self.visit(then_branch);
                self.visit(else_branch);
            }

            Expr::Piecewise { pieces, otherwise } => {
                for (cond, value) in pieces {
                    self.visit(cond);
                    self.visit(value);
                }
                self.visit(otherwise);
            }

            // 扩展运算符 - 递归处理
            _ => {
                // 使用 expr.rs 中的 collect_refs 方法处理
            }
        }
    }
}

impl Default for RefCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 表达式深度计算器
pub struct DepthCalculator;

impl DepthCalculator {
    /// 计算表达式深度
    pub fn calculate(expr: &Expr) -> usize {
        expr.depth()
    }
}

/// 运算符统计器
pub struct OperatorCounter {
    /// 各运算符出现次数
    pub counts: std::collections::HashMap<String, usize>,
}

impl OperatorCounter {
    /// 创建新的统计器
    pub fn new() -> Self {
        Self {
            counts: std::collections::HashMap::new(),
        }
    }

    /// 统计表达式中的运算符
    pub fn count(expr: &Expr) -> Self {
        let mut counter = Self::new();
        counter.visit(expr);
        counter
    }

    fn visit(&mut self, expr: &Expr) {
        let op_name = match expr {
            Expr::Const(_) => "const",
            Expr::Var(_) => "var",
            Expr::Param(_) => "param",
            Expr::Pi => "pi",
            Expr::E => "e",
            Expr::Add(_, _) => "add",
            Expr::Sub(_, _) => "sub",
            Expr::Mul(_, _) => "mul",
            Expr::Div(_, _) => "div",
            Expr::Neg(_) => "neg",
            Expr::Pow(_, _) => "pow",
            Expr::Abs(_) => "abs",
            Expr::Mod(_, _) => "mod",
            Expr::Ceil(_) => "ceil",
            Expr::Floor(_) => "floor",
            Expr::Round(_) => "round",
            Expr::Trunc(_) => "trunc",
            Expr::Sign(_) => "sign",
            Expr::Exp(_) => "exp",
            Expr::Ln(_) => "ln",
            Expr::Log10(_) => "log10",
            Expr::Log2(_) => "log2",
            Expr::Sqrt(_) => "sqrt",
            Expr::Cbrt(_) => "cbrt",
            Expr::Sin(_) => "sin",
            Expr::Cos(_) => "cos",
            Expr::Tan(_) => "tan",
            Expr::ASin(_) => "asin",
            Expr::ACos(_) => "acos",
            Expr::ATan(_) => "atan",
            Expr::ATan2(_, _) => "atan2",
            Expr::Sinh(_) => "sinh",
            Expr::Cosh(_) => "cosh",
            Expr::Tanh(_) => "tanh",
            Expr::ASinh(_) => "asinh",
            Expr::ACosh(_) => "acosh",
            Expr::ATanh(_) => "atanh",
            Expr::Max(_) => "max",
            Expr::Min(_) => "min",
            Expr::Sum { .. } => "sum",
            Expr::Product { .. } => "product",
            Expr::Eq(_, _) => "eq",
            Expr::Lt(_, _) => "lt",
            Expr::Gt(_, _) => "gt",
            Expr::Leq(_, _) => "leq",
            Expr::Geq(_, _) => "geq",
            Expr::Neq(_, _) => "neq",
            Expr::And(_, _) => "and",
            Expr::Or(_, _) => "or",
            Expr::Not(_) => "not",
            Expr::IfThenElse { .. } => "if_then_else",
            Expr::Piecewise { .. } => "piecewise",
            // 扩展运算符
            _ => "extended_op",
        };

        *self.counts.entry(op_name.to_string()).or_insert(0) += 1;

        // 递归遍历子节点
        match expr {
            Expr::Const(_) | Expr::Var(_) | Expr::Param(_) | Expr::Pi | Expr::E => {}

            Expr::Neg(a)
            | Expr::Abs(a)
            | Expr::Ceil(a)
            | Expr::Floor(a)
            | Expr::Round(a)
            | Expr::Trunc(a)
            | Expr::Sign(a)
            | Expr::Exp(a)
            | Expr::Ln(a)
            | Expr::Log10(a)
            | Expr::Log2(a)
            | Expr::Sqrt(a)
            | Expr::Cbrt(a)
            | Expr::Sin(a)
            | Expr::Cos(a)
            | Expr::Tan(a)
            | Expr::ASin(a)
            | Expr::ACos(a)
            | Expr::ATan(a)
            | Expr::Sinh(a)
            | Expr::Cosh(a)
            | Expr::Tanh(a)
            | Expr::ASinh(a)
            | Expr::ACosh(a)
            | Expr::ATanh(a)
            | Expr::Not(a) => {
                self.visit(a);
            }

            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b)
            | Expr::Mod(a, b)
            | Expr::ATan2(a, b)
            | Expr::Eq(a, b)
            | Expr::Lt(a, b)
            | Expr::Gt(a, b)
            | Expr::Leq(a, b)
            | Expr::Geq(a, b)
            | Expr::Neq(a, b)
            | Expr::And(a, b)
            | Expr::Or(a, b) => {
                self.visit(a);
                self.visit(b);
            }

            Expr::Max(args) | Expr::Min(args) => {
                for arg in args {
                    self.visit(arg);
                }
            }

            Expr::Sum { lower, upper, body, .. }
            | Expr::Product { lower, upper, body, .. } => {
                self.visit(lower);
                self.visit(upper);
                self.visit(body);
            }

            Expr::IfThenElse { cond, then_branch, else_branch } => {
                self.visit(cond);
                self.visit(then_branch);
                self.visit(else_branch);
            }

            Expr::Piecewise { pieces, otherwise } => {
                for (cond, value) in pieces {
                    self.visit(cond);
                    self.visit(value);
                }
                self.visit(otherwise);
            }

            // 扩展运算符 - 递归处理
            _ => {
                // 扩展运算符的子节点遍历由 expr.rs 中的 depth 方法处理
            }
        }
    }
}

impl Default for OperatorCounter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_collector() {
        let expr = Expr::add(
            Expr::param("p1"),
            Expr::mul(Expr::param("p2"), Expr::var("x")),
        );

        let collector = RefCollector::collect(&expr);

        assert_eq!(collector.params.len(), 2);
        assert!(collector.params.contains(&"p1".to_string()));
        assert!(collector.params.contains(&"p2".to_string()));

        assert_eq!(collector.vars.len(), 1);
        assert!(collector.vars.contains(&"x".to_string()));
    }

    #[test]
    fn test_operator_counter() {
        let expr = Expr::add(
            Expr::mul(Expr::var("a"), Expr::var("b")),
            Expr::mul(Expr::var("c"), Expr::var("d")),
        );

        let counter = OperatorCounter::count(&expr);

        assert_eq!(counter.counts.get("add"), Some(&1));
        assert_eq!(counter.counts.get("mul"), Some(&2));
        assert_eq!(counter.counts.get("var"), Some(&4));
    }

    #[test]
    fn test_new_operators_counter() {
        let expr = Expr::atan2(Expr::sinh(Expr::var("y")), Expr::cosh(Expr::var("x")));
        let counter = OperatorCounter::count(&expr);

        assert_eq!(counter.counts.get("atan2"), Some(&1));
        assert_eq!(counter.counts.get("sinh"), Some(&1));
        assert_eq!(counter.counts.get("cosh"), Some(&1));
    }

    #[test]
    fn test_depth_calculator() {
        // a + b 深度为 2
        let simple = Expr::add(Expr::var("a"), Expr::var("b"));
        assert_eq!(DepthCalculator::calculate(&simple), 2);

        // a + (b * c) 深度为 3
        let nested = Expr::add(Expr::var("a"), Expr::mul(Expr::var("b"), Expr::var("c")));
        assert_eq!(DepthCalculator::calculate(&nested), 3);
    }

    #[test]
    fn test_conditional_refs() {
        let expr = Expr::if_then_else(
            Expr::Gt(Box::new(Expr::var("x")), Box::new(Expr::Const(0.0))),
            Expr::var("a"),
            Expr::var("b"),
        );

        let collector = RefCollector::collect(&expr);
        assert_eq!(collector.vars.len(), 3);
        assert!(collector.vars.contains(&"x".to_string()));
        assert!(collector.vars.contains(&"a".to_string()));
        assert!(collector.vars.contains(&"b".to_string()));
    }
}
