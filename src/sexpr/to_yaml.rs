//! Expr到YAML的序列化
//!
//! 将Expr AST转换为符合现有YAML格式的serde_yaml::Value。

use crate::ast::Expr;
use serde_yaml::{Mapping, Value};

/// 将Expr转换为YAML Value
///
/// 生成的YAML格式与现有的方程文件格式兼容。
pub fn to_yaml_value(expr: &Expr) -> Value {
    ExprToYaml::convert(expr)
}

/// Expr到YAML转换器
struct ExprToYaml;

impl ExprToYaml {
    /// 主转换函数
    fn convert(expr: &Expr) -> Value {
        match expr {
            // 常量
            Expr::Const(n) => {
                let mut map = Mapping::new();
                map.insert(Value::String("const".to_string()), Value::Number((*n).into()));
                Value::Mapping(map)
            }
            
            // 变量引用
            Expr::Var(name) => Self::make_ref(name),
            
            // 参数引用
            Expr::Param(name) => Self::make_ref(name),
            
            // 常量 Pi 和 E
            Expr::Pi => Self::make_op("pi", vec![]),
            Expr::E => Self::make_op("e", vec![]),
            
            // 算术运算
            Expr::Add(a, b) => Self::binary("add", a, b),
            Expr::Sub(a, b) => Self::binary("sub", a, b),
            Expr::Mul(a, b) => Self::binary("mul", a, b),
            Expr::Div(a, b) => Self::binary("div", a, b),
            Expr::Neg(a) => Self::unary("neg", a),
            Expr::Pow(a, b) => Self::binary("pow", a, b),
            Expr::Abs(a) => Self::unary("abs", a),
            Expr::Mod(a, b) => Self::binary("mod", a, b),
            Expr::Ceil(a) => Self::unary("ceil", a),
            Expr::Floor(a) => Self::unary("floor", a),
            Expr::Round(a) => Self::unary("round", a),
            Expr::Trunc(a) => Self::unary("trunc", a),
            Expr::Sign(a) => Self::unary("sign", a),
            
            // 超越函数
            Expr::Exp(a) => Self::unary("exp", a),
            Expr::Ln(a) => Self::unary("ln", a),
            Expr::Log10(a) => Self::unary("log10", a),
            Expr::Log2(a) => Self::unary("log2", a),
            Expr::Sqrt(a) => Self::unary("sqrt", a),
            Expr::Cbrt(a) => Self::unary("cbrt", a),
            
            // 三角函数
            Expr::Sin(a) => Self::unary("sin", a),
            Expr::Cos(a) => Self::unary("cos", a),
            Expr::Tan(a) => Self::unary("tan", a),
            Expr::ASin(a) => Self::unary("asin", a),
            Expr::ACos(a) => Self::unary("acos", a),
            Expr::ATan(a) => Self::unary("atan", a),
            Expr::ATan2(a, b) => Self::binary("atan2", a, b),
            
            // 双曲函数
            Expr::Sinh(a) => Self::unary("sinh", a),
            Expr::Cosh(a) => Self::unary("cosh", a),
            Expr::Tanh(a) => Self::unary("tanh", a),
            Expr::ASinh(a) => Self::unary("asinh", a),
            Expr::ACosh(a) => Self::unary("acosh", a),
            Expr::ATanh(a) => Self::unary("atanh", a),
            
            // 聚合函数
            Expr::Max(args) => Self::variadic("max", args),
            Expr::Min(args) => Self::variadic("min", args),
            
            // 关系运算
            Expr::Eq(a, b) => Self::binary("eq", a, b),
            Expr::Lt(a, b) => Self::binary("lt", a, b),
            Expr::Gt(a, b) => Self::binary("gt", a, b),
            Expr::Leq(a, b) => Self::binary("leq", a, b),
            Expr::Geq(a, b) => Self::binary("geq", a, b),
            Expr::Neq(a, b) => Self::binary("neq", a, b),
            
            // 逻辑运算
            Expr::And(a, b) => Self::binary("and", a, b),
            Expr::Or(a, b) => Self::binary("or", a, b),
            Expr::Not(a) => Self::unary("not", a),
            
            // 条件表达式
            Expr::IfThenElse { cond, then_branch, else_branch } => {
                let mut map = Mapping::new();
                map.insert(Value::String("if".to_string()), Self::convert(cond));
                map.insert(Value::String("then".to_string()), Self::convert(then_branch));
                map.insert(Value::String("else".to_string()), Self::convert(else_branch));
                Value::Mapping(map)
            }
            
            // 求和
            Expr::Sum { index, lower, upper, body } => {
                let mut map = Mapping::new();
                map.insert(Value::String("sum".to_string()), Value::String(index.clone()));
                map.insert(Value::String("lower".to_string()), Self::convert(lower));
                map.insert(Value::String("upper".to_string()), Self::convert(upper));
                map.insert(Value::String("body".to_string()), Self::convert(body));
                Value::Mapping(map)
            }
            
            // 连乘
            Expr::Product { index, lower, upper, body } => {
                let mut map = Mapping::new();
                map.insert(Value::String("product".to_string()), Value::String(index.clone()));
                map.insert(Value::String("lower".to_string()), Self::convert(lower));
                map.insert(Value::String("upper".to_string()), Self::convert(upper));
                map.insert(Value::String("body".to_string()), Self::convert(body));
                Value::Mapping(map)
            }
            
            // 分段函数
            Expr::Piecewise { pieces, otherwise } => {
                let pieces_yaml: Vec<Value> = pieces.iter().map(|(cond, val)| {
                    let mut piece = Mapping::new();
                    piece.insert(Value::String("condition".to_string()), Self::convert(cond));
                    piece.insert(Value::String("value".to_string()), Self::convert(val));
                    Value::Mapping(piece)
                }).collect();
                
                let mut map = Mapping::new();
                map.insert(Value::String("pieces".to_string()), Value::Sequence(pieces_yaml));
                map.insert(Value::String("otherwise".to_string()), Self::convert(otherwise));
                Value::Mapping(map)
            }
            
            // Lambda
            Expr::Lambda { var, body } => {
                let mut map = Mapping::new();
                map.insert(Value::String("lambda".to_string()), Value::String(var.clone()));
                map.insert(Value::String("body".to_string()), Self::convert(body));
                Value::Mapping(map)
            }
            
            // 特殊函数
            Expr::Gamma(a) => Self::unary("gamma", a),
            Expr::Lgamma(a) => Self::unary("lgamma", a),
            Expr::Digamma(a) => Self::unary("digamma", a),
            Expr::Beta(a, b) => Self::binary("beta", a, b),
            Expr::Lbeta(a, b) => Self::binary("lbeta", a, b),
            Expr::Erf(a) => Self::unary("erf", a),
            Expr::Erfc(a) => Self::unary("erfc", a),
            Expr::Erfinv(a) => Self::unary("erfinv", a),
            Expr::Factorial(a) => Self::unary("factorial", a),
            Expr::Combination(a, b) => Self::binary("combination", a, b),
            Expr::Zeta(a) => Self::unary("zeta", a),
            
            // 贝塞尔函数
            Expr::BesselJ0(a) => Self::unary("bessel_j0", a),
            Expr::BesselJ1(a) => Self::unary("bessel_j1", a),
            Expr::BesselJn(a, b) => Self::binary("bessel_jn", a, b),
            Expr::BesselY0(a) => Self::unary("bessel_y0", a),
            Expr::BesselY1(a) => Self::unary("bessel_y1", a),
            Expr::BesselYn(a, b) => Self::binary("bessel_yn", a, b),
            Expr::BesselI0(a) => Self::unary("bessel_i0", a),
            Expr::BesselI1(a) => Self::unary("bessel_i1", a),
            Expr::BesselIn(a, b) => Self::binary("bessel_in", a, b),
            Expr::BesselK0(a) => Self::unary("bessel_k0", a),
            Expr::BesselK1(a) => Self::unary("bessel_k1", a),
            Expr::BesselKn(a, b) => Self::binary("bessel_kn", a, b),
            
            // 概率分布
            Expr::NormPdf(a, b, c) => Self::ternary("norm_pdf", a, b, c),
            Expr::NormCdf(a, b, c) => Self::ternary("norm_cdf", a, b, c),
            Expr::NormPpf(a, b, c) => Self::ternary("norm_ppf", a, b, c),
            Expr::TPdf(a, b) => Self::binary("t_pdf", a, b),
            Expr::TCdf(a, b) => Self::binary("t_cdf", a, b),
            Expr::TPpf(a, b) => Self::binary("t_ppf", a, b),
            Expr::Chi2Pdf(a, b) => Self::binary("chi2_pdf", a, b),
            Expr::Chi2Cdf(a, b) => Self::binary("chi2_cdf", a, b),
            Expr::Chi2Ppf(a, b) => Self::binary("chi2_ppf", a, b),
            Expr::FPdf(a, b, c) => Self::ternary("f_pdf", a, b, c),
            Expr::FCdf(a, b, c) => Self::ternary("f_cdf", a, b, c),
            Expr::FPpf(a, b, c) => Self::ternary("f_ppf", a, b, c),
            Expr::PoissonPmf(a, b) => Self::binary("poisson_pmf", a, b),
            Expr::PoissonCdf(a, b) => Self::binary("poisson_cdf", a, b),
            Expr::BinomialPmf(a, b, c) => Self::ternary("binomial_pmf", a, b, c),
            Expr::BinomialCdf(a, b, c) => Self::ternary("binomial_cdf", a, b, c),
            Expr::ExponentialPdf(a, b) => Self::binary("exponential_pdf", a, b),
            Expr::ExponentialCdf(a, b) => Self::binary("exponential_cdf", a, b),
            
            // 向量运算
            Expr::VectorLit(items) => Self::variadic("vector", items),
            Expr::Dot(a, b) => Self::binary("dot", a, b),
            Expr::Cross(a, b) => Self::binary("cross", a, b),
            Expr::VecNorm(a) => Self::unary("vec_norm", a),
            Expr::VecNormalize(a) => Self::unary("vec_normalize", a),
            Expr::Reduce { kind, arg } => Self::unary(kind.name(), arg),

            // 矩阵运算
            Expr::MatrixLit(rows) => {
                let rows_yaml: Vec<Value> = rows.iter().map(|row| {
                    Value::Sequence(row.iter().map(Self::convert).collect())
                }).collect();
                Self::make_op("matrix", vec![Value::Sequence(rows_yaml)])
            }
            Expr::MatMul(a, b) => Self::binary("matmul", a, b),
            Expr::Transpose(a) => Self::unary("transpose", a),
            Expr::Det(a) => Self::unary("det", a),
            Expr::Inv(a) => Self::unary("inv", a),
            Expr::Eigenvalues(a) => Self::unary("eigenvalues", a),
            Expr::Trace(a) => Self::unary("trace", a),
            Expr::MatNorm(a) => Self::unary("mat_norm", a),
            
            // 其他所有情况，使用通用处理
            _ => Self::convert_other(expr),
        }
    }
    
    /// 创建引用
    fn make_ref(name: &str) -> Value {
        let mut map = Mapping::new();
        map.insert(Value::String("ref".to_string()), Value::String(name.to_string()));
        Value::Mapping(map)
    }
    
    /// 创建运算符节点
    fn make_op(op: &str, args: Vec<Value>) -> Value {
        let mut map = Mapping::new();
        map.insert(Value::String("op".to_string()), Value::String(op.to_string()));
        if !args.is_empty() {
            map.insert(Value::String("args".to_string()), Value::Sequence(args));
        }
        Value::Mapping(map)
    }
    
    /// 一元运算符
    fn unary(op: &str, a: &Expr) -> Value {
        Self::make_op(op, vec![Self::convert(a)])
    }
    
    /// 二元运算符
    fn binary(op: &str, a: &Expr, b: &Expr) -> Value {
        Self::make_op(op, vec![Self::convert(a), Self::convert(b)])
    }
    
    /// 三元运算符
    fn ternary(op: &str, a: &Expr, b: &Expr, c: &Expr) -> Value {
        Self::make_op(op, vec![Self::convert(a), Self::convert(b), Self::convert(c)])
    }
    
    /// 可变参数运算符
    fn variadic(op: &str, args: &[Expr]) -> Value {
        Self::make_op(op, args.iter().map(Self::convert).collect())
    }
    
    /// 处理其他表达式类型
    fn convert_other(expr: &Expr) -> Value {
        // 使用Debug trait获取运算符名称
        let debug_str = format!("{:?}", expr);
        let op_name = debug_str.split('(').next().unwrap_or("unknown");
        
        // 尝试提取参数并转换
        let op_lower = op_name.to_lowercase();
        
        // 创建一个占位符
        let mut map = Mapping::new();
        map.insert(Value::String("op".to_string()), Value::String(op_lower));
        map.insert(Value::String("_note".to_string()), 
            Value::String("auto-generated from unsupported expression type".to_string()));
        Value::Mapping(map)
    }
}

/// 将YAML Value格式化为YAML字符串
pub fn to_yaml_string(expr: &Expr) -> Result<String, serde_yaml::Error> {
    let value = to_yaml_value(expr);
    serde_yaml::to_string(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_const() {
        let expr = Expr::Const(42.0);
        let yaml = to_yaml_value(&expr);
        
        if let Value::Mapping(map) = yaml {
            assert!(map.contains_key(Value::String("const".to_string())));
        } else {
            panic!("Expected Mapping");
        }
    }
    
    #[test]
    fn test_var() {
        let expr = Expr::var("x");
        let yaml = to_yaml_value(&expr);
        
        if let Value::Mapping(map) = yaml {
            assert!(map.contains_key(Value::String("ref".to_string())));
        } else {
            panic!("Expected Mapping");
        }
    }
    
    #[test]
    fn test_add() {
        let expr = Expr::add(Expr::Const(1.0), Expr::Const(2.0));
        let yaml = to_yaml_value(&expr);
        
        if let Value::Mapping(map) = yaml {
            assert_eq!(map.get(Value::String("op".to_string())), 
                Some(&Value::String("add".to_string())));
        } else {
            panic!("Expected Mapping");
        }
    }
    
    #[test]
    fn test_if_then_else() {
        let expr = Expr::if_then_else(
            Expr::Gt(Box::new(Expr::var("x")), Box::new(Expr::Const(0.0))),
            Expr::sqrt(Expr::var("x")),
            Expr::Const(0.0),
        );
        let yaml = to_yaml_value(&expr);
        
        if let Value::Mapping(map) = yaml {
            assert!(map.contains_key(Value::String("if".to_string())));
            assert!(map.contains_key(Value::String("then".to_string())));
            assert!(map.contains_key(Value::String("else".to_string())));
        } else {
            panic!("Expected Mapping");
        }
    }
    
    #[test]
    fn test_sum() {
        let expr = Expr::Sum {
            index: "i".to_string(),
            lower: Box::new(Expr::Const(1.0)),
            upper: Box::new(Expr::var("n")),
            body: Box::new(Expr::pow(Expr::var("i"), Expr::Const(2.0))),
        };
        let yaml = to_yaml_value(&expr);
        
        if let Value::Mapping(map) = yaml {
            assert!(map.contains_key(Value::String("sum".to_string())));
            assert!(map.contains_key(Value::String("lower".to_string())));
            assert!(map.contains_key(Value::String("upper".to_string())));
            assert!(map.contains_key(Value::String("body".to_string())));
        } else {
            panic!("Expected Mapping");
        }
    }
    
    #[test]
    fn test_to_yaml_string() {
        let expr = Expr::add(Expr::var("x"), Expr::Const(1.0));
        let yaml_str = to_yaml_string(&expr).unwrap();
        assert!(yaml_str.contains("add"));
    }
}
