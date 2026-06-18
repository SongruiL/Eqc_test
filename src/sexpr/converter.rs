//! S表达式到Expr的转换器
//!
//! 将解析后的S表达式AST转换为内部Expr AST。

use crate::ast::Expr;
use super::ast::SExpr;
use super::error::{SExprError, SExprResult};

/// 将SExpr转换为Expr
///
/// # 参数
/// - `sexpr`: S表达式AST
///
/// # 返回
/// - `Ok(Expr)`: 转换成功
/// - `Err(SExprError)`: 转换失败（未知运算符、参数错误等）
pub fn convert(sexpr: &SExpr) -> SExprResult<Expr> {
    Converter::new().convert(sexpr)
}

/// S表达式转换器
struct Converter;

impl Converter {
    fn new() -> Self {
        Self
    }
    
    /// 主转换函数
    fn convert(&self, sexpr: &SExpr) -> SExprResult<Expr> {
        match sexpr {
            SExpr::Number(n) => Ok(Expr::Const(*n)),
            
            SExpr::Symbol(s) => self.convert_symbol(s),
            
            SExpr::List(items) => self.convert_list(items),
        }
    }
    
    /// 转换符号
    fn convert_symbol(&self, s: &str) -> SExprResult<Expr> {
        match s {
            // 只有精确匹配 "pi" 和 "e" 才是数学常量
            // 大写形式如 "E", "PI" 可能是变量名（如活化能 E）
            "pi" => Ok(Expr::Pi),
            "e" => Ok(Expr::E),
            _ => {
                // 变量或参数引用
                Ok(Expr::var(s))
            }
        }
    }
    
    /// 转换列表（函数调用）
    fn convert_list(&self, items: &[SExpr]) -> SExprResult<Expr> {
        if items.is_empty() {
            return Err(SExprError::EmptyExpression {
                span: super::error::Span::new(0, 0, 0, 0),
            });
        }
        
        // 获取操作符
        let op = items[0].as_symbol().ok_or_else(|| SExprError::ExpectedSymbol {
            found: items[0].type_name().to_string(),
        })?;
        
        let args = &items[1..];
        
        // 特殊形式
        match op {
            "if" => return self.convert_if(args),
            "sum" => return self.convert_sum(args),
            "product" => return self.convert_product(args),
            "piecewise" => return self.convert_piecewise(args),
            "lambda" => return self.convert_lambda(args),
            _ => {}
        }
        
        // 转换参数
        let converted_args: SExprResult<Vec<Expr>> = args.iter().map(|a| self.convert(a)).collect();
        let exprs = converted_args?;
        
        // 运算符映射
        self.convert_operator(op, exprs)
    }
    
    /// 转换if表达式
    fn convert_if(&self, args: &[SExpr]) -> SExprResult<Expr> {
        if args.len() != 3 {
            return Err(SExprError::InvalidIfSyntax);
        }
        
        let cond = self.convert(&args[0])?;
        let then_branch = self.convert(&args[1])?;
        let else_branch = self.convert(&args[2])?;
        
        Ok(Expr::if_then_else(cond, then_branch, else_branch))
    }
    
    /// 转换sum表达式
    fn convert_sum(&self, args: &[SExpr]) -> SExprResult<Expr> {
        // (sum index lower upper body)
        if args.len() != 4 {
            return Err(SExprError::InvalidSumSyntax);
        }
        
        let index = args[0].as_symbol().ok_or(SExprError::ExpectedSymbol {
            found: args[0].type_name().to_string(),
        })?;
        
        let lower = self.convert(&args[1])?;
        let upper = self.convert(&args[2])?;
        let body = self.convert(&args[3])?;
        
        Ok(Expr::Sum {
            index: index.to_string(),
            lower: Box::new(lower),
            upper: Box::new(upper),
            body: Box::new(body),
        })
    }
    
    /// 转换product表达式
    fn convert_product(&self, args: &[SExpr]) -> SExprResult<Expr> {
        // (product index lower upper body)
        if args.len() != 4 {
            return Err(SExprError::InvalidProductSyntax);
        }
        
        let index = args[0].as_symbol().ok_or(SExprError::ExpectedSymbol {
            found: args[0].type_name().to_string(),
        })?;
        
        let lower = self.convert(&args[1])?;
        let upper = self.convert(&args[2])?;
        let body = self.convert(&args[3])?;
        
        Ok(Expr::Product {
            index: index.to_string(),
            lower: Box::new(lower),
            upper: Box::new(upper),
            body: Box::new(body),
        })
    }
    
    /// 转换piecewise表达式
    fn convert_piecewise(&self, args: &[SExpr]) -> SExprResult<Expr> {
        // (piecewise (cond1 val1) (cond2 val2) ... :otherwise default)
        if args.len() < 2 {
            return Err(SExprError::InvalidPiecewiseSyntax);
        }
        
        let mut pieces = Vec::new();
        let mut otherwise = None;
        
        for (i, arg) in args.iter().enumerate() {
            // 检查 :otherwise
            if let Some(kw) = arg.as_keyword() {
                if kw == "otherwise" {
                    // 下一个参数是默认值
                    if i + 1 < args.len() {
                        otherwise = Some(self.convert(&args[i + 1])?);
                    }
                    break;
                }
            }
            
            // 条件-值对
            if let Some(pair) = arg.as_list() {
                if pair.len() == 2 {
                    let cond = self.convert(&pair[0])?;
                    let val = self.convert(&pair[1])?;
                    pieces.push((cond, val));
                } else {
                    return Err(SExprError::InvalidPiecewiseSyntax);
                }
            } else {
                return Err(SExprError::InvalidPiecewiseSyntax);
            }
        }
        
        let default = otherwise.ok_or(SExprError::InvalidPiecewiseSyntax)?;
        
        Ok(Expr::Piecewise {
            pieces,
            otherwise: Box::new(default),
        })
    }
    
    /// 转换lambda表达式
    fn convert_lambda(&self, args: &[SExpr]) -> SExprResult<Expr> {
        // (lambda var body)
        if args.len() != 2 {
            return Err(SExprError::InvalidLambdaSyntax);
        }
        
        let var = args[0].as_symbol().ok_or(SExprError::ExpectedSymbol {
            found: args[0].type_name().to_string(),
        })?;
        
        let body = self.convert(&args[1])?;
        
        Ok(Expr::Lambda {
            var: var.to_string(),
            body: Box::new(body),
        })
    }
    
    /// 一元运算符辅助函数
    fn unary_op<F>(&self, args: Vec<Expr>, f: F, name: &str) -> SExprResult<Expr>
    where
        F: FnOnce(Expr) -> Expr,
    {
        if args.len() != 1 {
            return Err(SExprError::WrongArgCount {
                op: name.to_string(),
                expected: "1".to_string(),
                found: args.len(),
            });
        }
        Ok(f(args.into_iter().next().unwrap()))
    }
    
    /// 二元运算符辅助函数
    fn binary_op<F>(&self, args: Vec<Expr>, f: F, name: &str) -> SExprResult<Expr>
    where
        F: FnOnce(Expr, Expr) -> Expr,
    {
        if args.len() != 2 {
            return Err(SExprError::WrongArgCount {
                op: name.to_string(),
                expected: "2".to_string(),
                found: args.len(),
            });
        }
        let mut iter = args.into_iter();
        Ok(f(iter.next().unwrap(), iter.next().unwrap()))
    }
    
    /// 三元运算符辅助函数
    fn ternary_op<F>(&self, args: Vec<Expr>, f: F, name: &str) -> SExprResult<Expr>
    where
        F: FnOnce(Expr, Expr, Expr) -> Expr,
    {
        if args.len() != 3 {
            return Err(SExprError::WrongArgCount {
                op: name.to_string(),
                expected: "3".to_string(),
                found: args.len(),
            });
        }
        let mut iter = args.into_iter();
        Ok(f(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
    }
    
    /// 一元Box运算符
    fn unary_op_boxed<F>(&self, args: Vec<Expr>, f: F, name: &str) -> SExprResult<Expr>
    where
        F: FnOnce(Box<Expr>) -> Expr,
    {
        if args.len() != 1 {
            return Err(SExprError::WrongArgCount {
                op: name.to_string(),
                expected: "1".to_string(),
                found: args.len(),
            });
        }
        Ok(f(Box::new(args.into_iter().next().unwrap())))
    }
    
    /// 二元Box运算符
    fn binary_op_boxed<F>(&self, args: Vec<Expr>, f: F, name: &str) -> SExprResult<Expr>
    where
        F: FnOnce(Box<Expr>, Box<Expr>) -> Expr,
    {
        if args.len() != 2 {
            return Err(SExprError::WrongArgCount {
                op: name.to_string(),
                expected: "2".to_string(),
                found: args.len(),
            });
        }
        let mut iter = args.into_iter();
        Ok(f(Box::new(iter.next().unwrap()), Box::new(iter.next().unwrap())))
    }
    
    /// 运算符映射
    fn convert_operator(&self, op: &str, args: Vec<Expr>) -> SExprResult<Expr> {
        match op {
            // 常量
            "pi" => Ok(Expr::Pi),
            "e" => Ok(Expr::E),

            // 算术运算
            "add" => self.binary_op(args, Expr::add, "add"),
            "sub" => self.binary_op(args, Expr::sub, "sub"),
            "mul" => self.binary_op(args, Expr::mul, "mul"),
            "div" => self.binary_op(args, Expr::div, "div"),
            "neg" => self.unary_op(args, Expr::neg, "neg"),
            "pow" => self.binary_op(args, Expr::pow, "pow"),
            "abs" => self.unary_op(args, Expr::abs, "abs"),
            "mod" | "rem" => self.binary_op(args, Expr::modulo, "mod"),
            "ceil" => self.unary_op(args, Expr::ceil, "ceil"),
            "floor" => self.unary_op(args, Expr::floor, "floor"),
            "round" => self.unary_op(args, Expr::round, "round"),
            "trunc" => self.unary_op(args, Expr::trunc, "trunc"),
            "sign" | "signum" => self.unary_op(args, Expr::sign, "sign"),

            // 超越函数
            "exp" => self.unary_op(args, Expr::exp, "exp"),
            "ln" | "log" => self.unary_op(args, Expr::ln, "ln"),
            "log10" => self.unary_op(args, Expr::log10, "log10"),
            "log2" => self.unary_op(args, Expr::log2, "log2"),
            "sqrt" => self.unary_op(args, Expr::sqrt, "sqrt"),
            "cbrt" => self.unary_op(args, Expr::cbrt, "cbrt"),

            // 三角函数
            "sin" => self.unary_op(args, Expr::sin, "sin"),
            "cos" => self.unary_op(args, Expr::cos, "cos"),
            "tan" => self.unary_op(args, Expr::tan, "tan"),
            "sec" | "secant" => self.unary_op(args, Expr::sec, "sec"),
            "csc" | "cosec" | "cosecant" => self.unary_op(args, Expr::csc, "csc"),
            "cot" | "cotangent" => self.unary_op(args, Expr::cot, "cot"),
            "asin" | "arcsin" => self.unary_op(args, Expr::asin, "asin"),
            "acos" | "arccos" => self.unary_op(args, Expr::acos, "acos"),
            "atan" | "arctan" => self.unary_op(args, Expr::atan, "atan"),
            "asec" | "arcsec" => self.unary_op(args, Expr::asec, "asec"),
            "acsc" | "arccsc" => self.unary_op(args, Expr::acsc, "acsc"),
            "acot" | "arccot" => self.unary_op(args, Expr::acot, "acot"),
            "atan2" => self.binary_op(args, Expr::atan2, "atan2"),

            // 双曲函数
            "sinh" => self.unary_op(args, Expr::sinh, "sinh"),
            "cosh" => self.unary_op(args, Expr::cosh, "cosh"),
            "tanh" => self.unary_op(args, Expr::tanh, "tanh"),
            "sech" => self.unary_op(args, Expr::sech, "sech"),
            "csch" => self.unary_op(args, Expr::csch, "csch"),
            "coth" => self.unary_op(args, Expr::coth, "coth"),
            "asinh" | "arcsinh" => self.unary_op(args, Expr::asinh, "asinh"),
            "acosh" | "arccosh" => self.unary_op(args, Expr::acosh, "acosh"),
            "atanh" | "arctanh" => self.unary_op(args, Expr::atanh, "atanh"),
            "asech" | "arsech" => self.unary_op(args, Expr::asech, "asech"),
            "acsch" | "arcsch" => self.unary_op(args, Expr::acsch, "acsch"),
            "acoth" | "arcoth" => self.unary_op(args, Expr::acoth, "acoth"),

            // 聚合函数
            "max" => Ok(Expr::max(args)),
            "min" => Ok(Expr::min(args)),

            // 关系运算
            "eq" => self.binary_op_boxed(args, Expr::Eq, "eq"),
            "lt" => self.binary_op_boxed(args, Expr::Lt, "lt"),
            "gt" => self.binary_op_boxed(args, Expr::Gt, "gt"),
            "leq" | "le" => self.binary_op_boxed(args, Expr::Leq, "leq"),
            "geq" | "ge" => self.binary_op_boxed(args, Expr::Geq, "geq"),
            "neq" | "ne" => self.binary_op_boxed(args, Expr::Neq, "neq"),

            // 逻辑运算
            "and" => self.binary_op_boxed(args, Expr::And, "and"),
            "or" => self.binary_op_boxed(args, Expr::Or, "or"),
            "not" => self.unary_op_boxed(args, Expr::Not, "not"),

            // 扩展分位数函数
            "exp_ppf" => self.binary_op(args, Expr::exp_ppf, "exp_ppf"),
            "gamma_ppf" => self.ternary_op(args, Expr::gamma_ppf, "gamma_ppf"),
            "beta_ppf" => self.ternary_op(args, Expr::beta_ppf, "beta_ppf"),
            "weibull_ppf" => self.ternary_op(args, Expr::weibull_ppf, "weibull_ppf"),
            "lognorm_ppf" => self.ternary_op(args, Expr::lognorm_ppf, "lognorm_ppf"),
            "uniform_ppf" => self.ternary_op(args, Expr::uniform_ppf, "uniform_ppf"),
            "cauchy_ppf" => self.ternary_op(args, Expr::cauchy_ppf, "cauchy_ppf"),

            // 复数扩展
            "complex_sinh" => self.unary_op(args, Expr::complex_sinh, "complex_sinh"),
            "complex_cosh" => self.unary_op(args, Expr::complex_cosh, "complex_cosh"),
            "complex_tanh" => self.unary_op(args, Expr::complex_tanh, "complex_tanh"),
            "complex_asinh" => self.unary_op(args, Expr::complex_asinh, "complex_asinh"),
            "complex_acosh" => self.unary_op(args, Expr::complex_acosh, "complex_acosh"),
            "complex_atanh" => self.unary_op(args, Expr::complex_atanh, "complex_atanh"),
            "complex_asin" => self.unary_op(args, Expr::complex_asin, "complex_asin"),
            "complex_acos" => self.unary_op(args, Expr::complex_acos, "complex_acos"),
            "complex_atan" => self.unary_op(args, Expr::complex_atan, "complex_atan"),

            // 数论函数
            "gcd" => self.binary_op(args, Expr::gcd, "gcd"),
            "lcm" => self.binary_op(args, Expr::lcm, "lcm"),
            "permutation" | "perm" => self.binary_op(args, Expr::permutation, "permutation"),

            // 正交多项式
            "legendre" => self.binary_op(args, Expr::legendre, "legendre"),
            "legendre_assoc" => self.ternary_op(args, Expr::legendre_assoc, "legendre_assoc"),
            "hermite" => self.binary_op(args, Expr::hermite, "hermite"),
            "laguerre" => self.binary_op(args, Expr::laguerre, "laguerre"),
            "laguerre_assoc" => self.ternary_op(args, Expr::laguerre_assoc, "laguerre_assoc"),
            "chebyshev_t" => self.binary_op(args, Expr::chebyshev_t, "chebyshev_t"),
            "chebyshev_u" => self.binary_op(args, Expr::chebyshev_u, "chebyshev_u"),

            // 椭圆积分
            "ellipk" => self.unary_op(args, Expr::ellip_k, "ellipk"),
            "ellipe" => self.unary_op(args, Expr::ellip_e, "ellipe"),
            "ellipf" | "ellipkinc" | "elliptic_f" => self.binary_op(args, Expr::ellipf, "ellipf"),
            "ellipe_inc" | "ellipeinc" | "elliptic_e_inc" => self.binary_op(args, Expr::ellipe_inc, "ellipe_inc"),
            "ellippi" | "elliptic_pi" => self.ternary_op(args, Expr::ellippi, "ellippi"),

            // 向量运算
            "vector" => Ok(Expr::VectorLit(args)),
            "dot" => self.binary_op(args, Expr::dot, "dot"),
            "cross" => self.binary_op(args, Expr::cross, "cross"),
            "vec_norm" => self.unary_op(args, Expr::vec_norm, "vec_norm"),
            "vec_normalize" | "normalize" => self.unary_op(args, Expr::vec_normalize, "normalize"),
            "vsum" => self.unary_op(args, Expr::vsum, "vsum"),
            "vprod" => self.unary_op(args, Expr::vprod, "vprod"),
            "vmean" => self.unary_op(args, Expr::vmean, "vmean"),
            "vmin" => self.unary_op(args, Expr::vmin, "vmin"),
            "vmax" => self.unary_op(args, Expr::vmax, "vmax"),

            // 矩阵运算
            "matmul" => self.binary_op(args, Expr::mat_mul, "matmul"),
            "transpose" => self.unary_op(args, Expr::transpose, "transpose"),
            "det" => self.unary_op(args, Expr::det, "det"),
            "inv" => self.unary_op(args, Expr::inv, "inv"),
            "eigenvalues" => self.unary_op(args, Expr::eigenvalues, "eigenvalues"),
            "trace" => self.unary_op(args, Expr::trace, "trace"),
            "mat_norm" => self.unary_op(args, Expr::mat_norm, "mat_norm"),

            // 特殊函数
            "gamma" => self.unary_op(args, Expr::gamma, "gamma"),
            "lgamma" | "gammaln" => self.unary_op(args, Expr::lgamma, "lgamma"),
            "digamma" | "psi" => self.unary_op(args, Expr::digamma, "digamma"),
            "beta" => self.binary_op(args, Expr::beta_fn, "beta"),
            "lbeta" | "logbeta" | "betaln" => self.binary_op(args, Expr::lbeta, "lbeta"),
            "erf" => self.unary_op(args, Expr::erf, "erf"),
            "erfc" => self.unary_op(args, Expr::erfc, "erfc"),
            "erfinv" => self.unary_op(args, Expr::erfinv, "erfinv"),
            "factorial" | "fact" => self.unary_op(args, Expr::factorial, "factorial"),
            "combination" | "comb" | "choose" => self.binary_op(args, Expr::combination, "combination"),
            "zeta" | "riemann_zeta" => self.unary_op(args, Expr::zeta, "zeta"),

            // 贝塞尔函数
            "bessel_j0" | "j0" => self.unary_op(args, Expr::bessel_j0, "bessel_j0"),
            "bessel_j1" | "j1" => self.unary_op(args, Expr::bessel_j1, "bessel_j1"),
            "bessel_jn" | "jn" | "jv" => self.binary_op(args, Expr::bessel_jn, "bessel_jn"),
            "bessel_y0" | "y0" => self.unary_op(args, Expr::bessel_y0, "bessel_y0"),
            "bessel_y1" | "y1" => self.unary_op(args, Expr::bessel_y1, "bessel_y1"),
            "bessel_yn" | "yn" | "yv" => self.binary_op(args, Expr::bessel_yn, "bessel_yn"),
            "bessel_i0" | "i0" => self.unary_op(args, Expr::bessel_i0, "bessel_i0"),
            "bessel_i1" | "i1" => self.unary_op(args, Expr::bessel_i1, "bessel_i1"),
            "bessel_in" | "in" | "iv" => self.binary_op(args, Expr::bessel_in, "bessel_in"),
            "bessel_k0" | "k0" => self.unary_op(args, Expr::bessel_k0, "bessel_k0"),
            "bessel_k1" | "k1" => self.unary_op(args, Expr::bessel_k1, "bessel_k1"),
            "bessel_kn" | "kn" | "kv" => self.binary_op(args, Expr::bessel_kn, "bessel_kn"),

            // 球贝塞尔函数
            "sph_bessel_j" | "spherical_jn" | "jl" => self.binary_op(args, Expr::sph_bessel_j, "sph_bessel_j"),
            "sph_bessel_y" | "spherical_yn" | "yl" => self.binary_op(args, Expr::sph_bessel_y, "sph_bessel_y"),
            "sph_bessel_i" | "spherical_in" | "il" => self.binary_op(args, Expr::sph_bessel_i, "sph_bessel_i"),
            "sph_bessel_k" | "spherical_kn" | "kl" => self.binary_op(args, Expr::sph_bessel_k, "sph_bessel_k"),

            // 概率分布
            "norm_pdf" => self.ternary_op(args, Expr::norm_pdf, "norm_pdf"),
            "norm_cdf" | "ndtr" => self.ternary_op(args, Expr::norm_cdf, "norm_cdf"),
            "norm_ppf" | "ndtri" => self.ternary_op(args, Expr::norm_ppf, "norm_ppf"),
            "t_pdf" => self.binary_op(args, Expr::t_pdf, "t_pdf"),
            "t_cdf" => self.binary_op(args, Expr::t_cdf, "t_cdf"),
            "t_ppf" => self.binary_op(args, Expr::t_ppf, "t_ppf"),
            "chi2_pdf" => self.binary_op(args, Expr::chi2_pdf, "chi2_pdf"),
            "chi2_cdf" => self.binary_op(args, Expr::chi2_cdf, "chi2_cdf"),
            "chi2_ppf" => self.binary_op(args, Expr::chi2_ppf, "chi2_ppf"),
            "f_pdf" => self.ternary_op(args, Expr::f_pdf, "f_pdf"),
            "f_cdf" => self.ternary_op(args, Expr::f_cdf, "f_cdf"),
            "f_ppf" => self.ternary_op(args, Expr::f_ppf, "f_ppf"),
            "poisson_pmf" => self.binary_op(args, Expr::poisson_pmf, "poisson_pmf"),
            "poisson_cdf" => self.binary_op(args, Expr::poisson_cdf, "poisson_cdf"),
            "binomial_pmf" => self.ternary_op(args, Expr::binomial_pmf, "binomial_pmf"),
            "binomial_cdf" => self.ternary_op(args, Expr::binomial_cdf, "binomial_cdf"),
            "exponential_pdf" | "exp_pdf" => self.binary_op(args, Expr::exponential_pdf, "exponential_pdf"),
            "exponential_cdf" | "exp_cdf" => self.binary_op(args, Expr::exponential_cdf, "exponential_cdf"),

            // 复数运算
            "complex" => self.binary_op(args, Expr::complex, "complex"),
            "real" | "re" => self.unary_op(args, Expr::real, "real"),
            "imag" | "im" => self.unary_op(args, Expr::imag, "imag"),
            "conj" | "conjugate" => self.unary_op(args, Expr::conj, "conj"),
            "carg" | "arg" => self.unary_op(args, Expr::carg, "carg"),
            "cabs" => self.unary_op(args, Expr::cabs, "cabs"),
            "polar" => self.binary_op(args, Expr::polar, "polar"),

            // 基础数学补充
            "hypot" => self.binary_op(args, Expr::hypot, "hypot"),
            "hypot3" => self.ternary_op(args, Expr::hypot3, "hypot3"),
            "clamp" => self.ternary_op(args, Expr::clamp, "clamp"),
            "copysign" => self.binary_op(args, Expr::copysign, "copysign"),
            "fma" | "mul_add" => self.ternary_op(args, Expr::fma, "fma"),
            "logn" | "log_base" => self.binary_op(args, Expr::logn, "logn"),
            "sinc" => self.unary_op(args, Expr::sinc, "sinc"),

            // 高精度数值函数
            "expm1" => self.unary_op(args, Expr::expm1, "expm1"),
            "log1p" => self.unary_op(args, Expr::log1p, "log1p"),
            "exp2" => self.unary_op(args, Expr::exp2, "exp2"),

            // 不完全伽马/贝塔函数
            "gammainc" | "gammainc_lower" => self.binary_op(args, Expr::gammainc, "gammainc"),
            "gammaincc" | "gammainc_upper" => self.binary_op(args, Expr::gammaincc, "gammaincc"),
            "betainc" | "betainc_regularized" | "regularized_betainc" => self.ternary_op(args, Expr::betainc, "betainc"),


            // Airy函数
            "airy_ai" => self.unary_op(args, Expr::airy_ai, "airy_ai"),
            "airy_bi" | "bi" | "airybi" => self.unary_op(args, Expr::airy_bi, "airy_bi"),

            // 球谐函数
            "sph_harmonic" | "spherical_harmonic" | "ylm" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "spherical_harmonic".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::spherical_harmonic(
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                ))
            }

            // Fresnel积分
            "fresnel_s" | "fresnel" => self.unary_op(args, Expr::fresnel_s, "fresnel_s"),
            "fresnel_c" | "fresnelc" => self.unary_op(args, Expr::fresnel_c, "fresnel_c"),

            // Dawson积分
            "dawson" | "dawsn" => self.unary_op(args, Expr::dawson, "dawson"),

            // 指数积分
            "expint" | "ei" | "expi" => self.unary_op(args, Expr::exp_int, "expint"),

            // 对数积分
            "logint" | "li" => self.unary_op(args, Expr::log_int, "logint"),

            // 三角积分
            "sinint" | "si" => self.unary_op(args, Expr::sin_int, "sinint"),
            "cosint" | "ci" => self.unary_op(args, Expr::cos_int, "cosint"),

            // Lambert W函数
            "lambertw" | "lambert_w" | "w0" => self.unary_op(args, Expr::lambertw, "lambertw"),
            "lambertwm1" | "lambert_wm1" | "wm1" => self.unary_op(args, Expr::lambertw_m1, "lambertwm1"),

            // 超几何函数
            "hyp0f1" | "0f1" => self.binary_op(args, Expr::hyp0f1, "hyp0f1"),
            "hyp1f1" | "kummer_m" => self.ternary_op(args, Expr::hyp1f1, "hyp1f1"),
            "hyp2f1" | "gauss_hypergeometric" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "hyp2f1".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::hyp2f1(
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                ))
            }

            // Kelvin函数
            "kelvin_ber" | "ber" | "kelvin" => self.unary_op(args, Expr::kelvin_ber, "kelvin_ber"),
            "kelvin_bei" | "bei" => self.unary_op(args, Expr::kelvin_bei, "kelvin_bei"),
            "kelvin_ker" | "ker" => self.unary_op(args, Expr::kelvin_ker, "kelvin_ker"),
            "kelvin_kei" | "kei" => self.unary_op(args, Expr::kelvin_kei, "kelvin_kei"),

            // 其他特殊函数
            "spence" | "dilog" | "li2" => self.unary_op(args, Expr::spence, "spence"),
            "polygamma" | "psi_n" => self.binary_op(args, Expr::polygamma, "polygamma"),
            "hankel1" | "hankel_1" => self.binary_op(args, Expr::hankel1, "hankel1"),
            "hankel2" | "hankel_2" => self.binary_op(args, Expr::hankel2, "hankel2"),
            "struve_h" | "struveh" | "struve" => self.binary_op(args, Expr::struve_h, "struve_h"),
            "struve_l" | "struvel" | "modstruve" => self.binary_op(args, Expr::struve_l, "struve_l"),
            "owens_t" | "owenst" => self.binary_op(args, Expr::owens_t, "owens_t"),
            "riemann_siegel_z" | "siegelz" => self.unary_op(args, Expr::riemann_siegel_z, "riemann_siegel_z"),
            "riemann_siegel_theta" | "siegeltheta" => self.unary_op(args, Expr::riemann_siegel_theta, "riemann_siegel_theta"),

            // Jacobi 椭圆函数
            "jacobi_sn" | "sn" | "ellipj" => self.binary_op(args, Expr::jacobi_sn, "jacobi_sn"),
            "jacobi_cn" | "cn" => self.binary_op(args, Expr::jacobi_cn, "jacobi_cn"),
            "jacobi_dn" | "dn" => self.binary_op(args, Expr::jacobi_dn, "jacobi_dn"),

            // 广义正交多项式
            "gegenbauer" | "ultraspherical" => self.ternary_op(args, Expr::gegenbauer, "gegenbauer"),
            "jacobi_p" | "jacobi_poly" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "jacobi_p".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::jacobi_p(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }

            // Mathieu 函数
            "mathieu_a" => self.binary_op(args, Expr::mathieu_a, "mathieu_a"),
            "mathieu_b" => self.binary_op(args, Expr::mathieu_b, "mathieu_b"),
            "mathieu_ce" | "mathieu_cem" => self.ternary_op(args, Expr::mathieu_ce, "mathieu_ce"),
            "mathieu_se" | "mathieu_sem" => self.ternary_op(args, Expr::mathieu_se, "mathieu_se"),

            // Coulomb 波函数
            "coulomb_f" | "coulombf" => self.ternary_op(args, Expr::coulomb_f, "coulomb_f"),
            "coulomb_g" | "coulombg" => self.ternary_op(args, Expr::coulomb_g, "coulomb_g"),

            // Wigner 符号
            "wigner_3j" | "wigner3j" => {
                if args.len() != 6 {
                    return Err(SExprError::WrongArgCount {
                        op: "wigner_3j".to_string(),
                        expected: "6".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::wigner_3j(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }
            "wigner_6j" | "wigner6j" => {
                if args.len() != 6 {
                    return Err(SExprError::WrongArgCount {
                        op: "wigner_6j".to_string(),
                        expected: "6".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::wigner_6j(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }
            "wigner_9j" | "wigner9j" => {
                if args.len() != 9 {
                    return Err(SExprError::WrongArgCount {
                        op: "wigner_9j".to_string(),
                        expected: "9".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::wigner_9j(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }

            // Theta 函数
            "theta1" | "jtheta1" => self.binary_op(args, Expr::theta1, "theta1"),
            "theta2" | "jtheta2" => self.binary_op(args, Expr::theta2, "theta2"),
            "theta3" | "jtheta3" => self.binary_op(args, Expr::theta3, "theta3"),
            "theta4" | "jtheta4" => self.binary_op(args, Expr::theta4, "theta4"),

            // 抛物柱面函数
            "pbdv" | "parabolic_d" => self.binary_op(args, Expr::pbdv, "pbdv"),
            "pbvv" | "parabolic_v" => self.binary_op(args, Expr::pbvv, "pbvv"),
            "pbwa" | "parabolic_w" => self.binary_op(args, Expr::pbwa, "pbwa"),

            // 球扁旋转体波函数（长球）
            "pro_ang1" | "prolate_ang1" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "pro_ang1".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::pro_ang1(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }
            "pro_rad1" | "prolate_rad1" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "pro_rad1".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::pro_rad1(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }
            "pro_rad2" | "prolate_rad2" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "pro_rad2".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::pro_rad2(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }

            // 球扁旋转体波函数（扁球）
            "obl_ang1" | "oblate_ang1" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "obl_ang1".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::obl_ang1(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }
            "obl_rad1" | "oblate_rad1" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "obl_rad1".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::obl_rad1(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }
            "obl_rad2" | "oblate_rad2" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "obl_rad2".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::obl_rad2(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }

            // 修改 Fresnel 积分
            "modfresnelp" | "mod_fresnel_plus" => self.unary_op(args, Expr::mod_fresnel_p, "modfresnelp"),
            "modfresnelm" | "mod_fresnel_minus" => self.unary_op(args, Expr::mod_fresnel_m, "modfresnelm"),

            // Wright 函数
            "wright_bessel" => self.ternary_op(args, Expr::wright_bessel, "wright_bessel"),
            "wright_omega" | "wrightomega" => self.unary_op(args, Expr::wright_omega, "wright_omega"),

            // Voigt
            "voigt" | "voigt_profile" => self.ternary_op(args, Expr::voigt, "voigt"),

            // Sigmoid/Logistic
            "logit" => self.unary_op(args, Expr::logit, "logit"),
            "expit" | "sigmoid" | "logistic" => self.unary_op(args, Expr::expit, "expit"),

            // Box-Cox
            "boxcox" => self.binary_op(args, Expr::boxcox, "boxcox"),
            "boxcox1p" => self.binary_op(args, Expr::boxcox1p, "boxcox1p"),
            "inv_boxcox" => self.binary_op(args, Expr::inv_boxcox, "inv_boxcox"),
            "inv_boxcox1p" => self.binary_op(args, Expr::inv_boxcox1p, "inv_boxcox1p"),

            // 信息论
            "entr" | "entropy" => self.unary_op(args, Expr::entr, "entr"),
            "rel_entr" | "relative_entropy" => self.binary_op(args, Expr::rel_entr, "rel_entr"),
            "kl_div" | "kl_divergence" => self.binary_op(args, Expr::kl_div, "kl_div"),

            // 阶乘扩展
            "factorial2" | "double_factorial" => self.unary_op(args, Expr::factorial2, "factorial2"),
            "factorialk" => self.binary_op(args, Expr::factorialk, "factorialk"),
            "stirling2" => self.binary_op(args, Expr::stirling2, "stirling2"),
            "poch" | "pochhammer" => self.binary_op(args, Expr::poch, "poch"),

            // Carlson 椭圆积分
            "elliprc" => self.binary_op(args, Expr::elliprc, "elliprc"),
            "elliprd" => self.ternary_op(args, Expr::elliprd, "elliprd"),
            "elliprf" => self.ternary_op(args, Expr::elliprf, "elliprf"),
            "elliprg" => self.ternary_op(args, Expr::elliprg, "elliprg"),
            "elliprj" => {
                if args.len() != 4 {
                    return Err(SExprError::WrongArgCount {
                        op: "elliprj".to_string(),
                        expected: "4".to_string(),
                        found: args.len(),
                    });
                }
                let mut iter = args.into_iter();
                Ok(Expr::elliprj(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap()))
            }

            // 扩展误差函数
            "erfcx" => self.unary_op(args, Expr::erfcx, "erfcx"),
            "erfi" => self.unary_op(args, Expr::erfi, "erfi"),
            "erfcinv" => self.unary_op(args, Expr::erfcinv, "erfcinv"),

            // 扩展 Gamma
            "hyperu" => self.ternary_op(args, Expr::hyperu, "hyperu"),
            "rgamma" => self.unary_op(args, Expr::rgamma, "rgamma"),
            "gammasgn" => self.unary_op(args, Expr::gammasgn, "gammasgn"),

            // 便利函数
            "agm" => self.binary_op(args, Expr::agm, "agm"),
            "exprel" => self.unary_op(args, Expr::exprel, "exprel"),
            "xlogy" => self.binary_op(args, Expr::xlogy, "xlogy"),
            "xlog1py" => self.binary_op(args, Expr::xlog1py, "xlog1py"),

            // Zeta 扩展
            "hurwitz_zeta" => self.binary_op(args, Expr::hurwitz_zeta, "hurwitz_zeta"),
            "zetac" => self.unary_op(args, Expr::zetac, "zetac"),
            "polylog" => self.binary_op(args, Expr::polylog, "polylog"),

            // 缩放贝塞尔函数
            "i0e" | "bessel_i0e" => self.unary_op(args, Expr::bessel_i0e, "i0e"),
            "i1e" | "bessel_i1e" => self.unary_op(args, Expr::bessel_i1e, "i1e"),
            "ive" | "bessel_ine" => self.binary_op(args, Expr::bessel_ine, "ive"),
            "k0e" | "bessel_k0e" => self.unary_op(args, Expr::bessel_k0e, "k0e"),
            "k1e" | "bessel_k1e" => self.unary_op(args, Expr::bessel_k1e, "k1e"),
            "kve" | "bessel_kne" => self.binary_op(args, Expr::bessel_kne, "kve"),
            "jve" | "bessel_jne" => self.binary_op(args, Expr::bessel_jne, "jve"),
            "yve" | "bessel_yne" => self.binary_op(args, Expr::bessel_yne, "yve"),
            "hankel1e" => self.binary_op(args, Expr::hankel1e, "hankel1e"),
            "hankel2e" => self.binary_op(args, Expr::hankel2e, "hankel2e"),

            // 贝塞尔函数导数
            "jvp" | "bessel_jnp" => self.binary_op(args, Expr::bessel_jnp, "jvp"),
            "yvp" | "bessel_ynp" => self.binary_op(args, Expr::bessel_ynp, "yvp"),
            "ivp" | "bessel_inp" => self.binary_op(args, Expr::bessel_inp, "ivp"),
            "kvp" | "bessel_knp" => self.binary_op(args, Expr::bessel_knp, "kvp"),
            "h1vp" | "hankel1p" => self.binary_op(args, Expr::hankel1p, "h1vp"),
            "h2vp" | "hankel2p" => self.binary_op(args, Expr::hankel2p, "h2vp"),

            // Huber 损失
            "huber" => self.binary_op(args, Expr::huber, "huber"),
            "pseudo_huber" => self.binary_op(args, Expr::pseudo_huber, "pseudo_huber"),

            // Kolmogorov-Smirnov
            "kolmogorov" => self.unary_op(args, Expr::kolmogorov, "kolmogorov"),
            "kolmogi" => self.unary_op(args, Expr::kolmogi, "kolmogi"),
            "smirnov" => self.binary_op(args, Expr::smirnov, "smirnov"),
            "smirnovi" => self.binary_op(args, Expr::smirnovi, "smirnovi"),

            // Faddeeva
            "wofz" | "faddeeva" => self.unary_op(args, Expr::wofz, "wofz"),

            // Dirichlet 核
            "diric" | "dirichlet" => self.binary_op(args, Expr::diric, "diric"),

            // Tukey lambda
            "tklmbda" | "tukey_lambda" => self.binary_op(args, Expr::tklmbda, "tklmbda"),

            // Gamma/Beta 逆函数
            "gammaincinv" => self.binary_op(args, Expr::gammaincinv, "gammaincinv"),
            "gammainccinv" => self.binary_op(args, Expr::gammainccinv, "gammainccinv"),
            "betaincinv" => self.ternary_op(args, Expr::betaincinv, "betaincinv"),
            "betaincc" => self.ternary_op(args, Expr::betaincc, "betaincc"),
            "betainccinv" => self.ternary_op(args, Expr::betainccinv, "betainccinv"),

            // 高精度便利函数
            "cosm1" => self.unary_op(args, Expr::cosm1, "cosm1"),
            "powm1" => self.binary_op(args, Expr::powm1, "powm1"),
            "exp10" => self.unary_op(args, Expr::exp10, "exp10"),
            "log1pmx" => self.unary_op(args, Expr::log1pmx, "log1pmx"),
            "loggamma" => self.unary_op(args, Expr::loggamma, "loggamma"),

            // 度数三角函数
            "cosdg" => self.unary_op(args, Expr::cosdg, "cosdg"),
            "sindg" => self.unary_op(args, Expr::sindg, "sindg"),
            "tandg" => self.unary_op(args, Expr::tandg, "tandg"),
            "cotdg" => self.unary_op(args, Expr::cotdg, "cotdg"),
            "radian" => self.ternary_op(args, Expr::radian, "radian"),

            // Airy 扩展
            "airy" | "airyai" => self.unary_op(args, Expr::airy_ai, "airy"),
            "airye" => self.unary_op(args, Expr::airy_aie, "airye"),
            "aie" | "airy_aie" => self.unary_op(args, Expr::airy_aie, "aie"),
            "bie" | "airy_bie" => self.unary_op(args, Expr::airy_bie, "bie"),
            "aip" | "airy_aip" => self.unary_op(args, Expr::airy_aip, "aip"),
            "bip" | "airy_bip" => self.unary_op(args, Expr::airy_bip, "bip"),
            "itairy" => self.unary_op(args, Expr::itairy, "itairy"),

            // 指数积分扩展
            "expn" => self.binary_op(args, Expr::expn, "expn"),
            "exp1" | "e1" => self.unary_op(args, Expr::exp1, "exp1"),
            "shi" => self.unary_op(args, Expr::shi, "shi"),
            "chi" => self.unary_op(args, Expr::chi, "chi"),

            // Struve 积分
            "itstruve0" => self.unary_op(args, Expr::itstruve0, "itstruve0"),
            "it2struve0" => self.unary_op(args, Expr::it2struve0, "it2struve0"),
            "itmodstruve0" => self.unary_op(args, Expr::itmodstruve0, "itmodstruve0"),

            // ML/统计扩展
            "log_expit" => self.unary_op(args, Expr::log_expit, "log_expit"),
            "softplus" => self.unary_op(args, Expr::softplus, "softplus"),
            "log_ndtr" => self.unary_op(args, Expr::log_ndtr, "log_ndtr"),
            "softmax" => self.unary_op(args, Expr::softmax, "softmax"),
            "log_softmax" => self.unary_op(args, Expr::log_softmax, "log_softmax"),
            "logsumexp" => self.unary_op(args, Expr::logsumexp, "logsumexp"),

            // 数论函数
            "bernoulli" => self.unary_op(args, Expr::bernoulli, "bernoulli"),
            "euler" => self.unary_op(args, Expr::euler, "euler"),

            // 椭圆扩展
            "ellipkm1" => self.unary_op(args, Expr::ellipkm1, "ellipkm1"),

            // Kelvin 导数
            "berp" | "kelvin_berp" => self.unary_op(args, Expr::kelvin_berp, "berp"),
            "beip" | "kelvin_beip" => self.unary_op(args, Expr::kelvin_beip, "beip"),
            "kerp" | "kelvin_kerp" => self.unary_op(args, Expr::kelvin_kerp, "kerp"),
            "keip" | "kelvin_keip" => self.unary_op(args, Expr::kelvin_keip, "keip"),

            // 贝塞尔积分
            "besselpoly" => self.ternary_op(args, Expr::besselpoly, "besselpoly"),

            // Wright Bessel 扩展
            "log_wright_bessel" => self.ternary_op(args, Expr::log_wright_bessel, "log_wright_bessel"),

            // 二项系数扩展
            "binom" => self.binary_op(args, Expr::binom, "binom"),

            // scipy 分布函数别名
            "bdtr" => self.ternary_op(args, Expr::bdtr, "bdtr"),
            "bdtrc" => self.ternary_op(args, Expr::bdtrc, "bdtrc"),
            "bdtri" => self.ternary_op(args, Expr::bdtri, "bdtri"),
            "chdtr" => self.binary_op(args, Expr::chdtr, "chdtr"),
            "chdtrc" => self.binary_op(args, Expr::chdtrc, "chdtrc"),
            "chdtri" => self.binary_op(args, Expr::chdtri, "chdtri"),
            "fdtr" => self.ternary_op(args, Expr::fdtr, "fdtr"),
            "fdtrc" => self.ternary_op(args, Expr::fdtrc, "fdtrc"),
            "fdtri" => self.ternary_op(args, Expr::fdtri, "fdtri"),
            "stdtr" => self.binary_op(args, Expr::stdtr, "stdtr"),
            "stdtrc" => self.binary_op(args, Expr::stdtrc, "stdtrc"),
            "stdtrit" => self.binary_op(args, Expr::stdtrit, "stdtrit"),
            "pdtr" => self.binary_op(args, Expr::pdtr, "pdtr"),
            "pdtrc" => self.binary_op(args, Expr::pdtrc, "pdtrc"),
            "pdtri" => self.binary_op(args, Expr::pdtri, "pdtri"),
            "btdtr" => self.ternary_op(args, Expr::btdtr, "btdtr"),
            "btdtrc" => self.ternary_op(args, Expr::btdtrc, "btdtrc"),
            "gdtr" => self.ternary_op(args, Expr::gdtr, "gdtr"),
            "gdtrc" => self.ternary_op(args, Expr::gdtrc, "gdtrc"),

            // 积分组合
            "sici" => self.unary_op(args, Expr::sici, "sici"),
            "shichi" => self.unary_op(args, Expr::shichi, "shichi"),

            // GSL 扩展
            "ai_zero" | "airy_zero_ai" => self.unary_op(args, Expr::airy_zero_ai, "ai_zero"),
            "bi_zero" | "airy_zero_bi" => self.unary_op(args, Expr::airy_zero_bi, "bi_zero"),
            "bessel_zero_j0" | "j0_zero" => self.unary_op(args, Expr::bessel_zero_j0, "bessel_zero_j0"),
            "bessel_zero_j1" | "j1_zero" => self.unary_op(args, Expr::bessel_zero_j1, "bessel_zero_j1"),
            "bessel_zero_jnu" | "jnu_zero" => self.binary_op(args, Expr::bessel_zero_jnu, "bessel_zero_jnu"),
            "sph_legendre" | "lpmv" => self.ternary_op(args, Expr::sph_legendre, "sph_legendre"),
            "clausen" => self.unary_op(args, Expr::clausen, "clausen"),
            "debye" => self.binary_op(args, Expr::debye, "debye"),
            "synchrotron1" => self.unary_op(args, Expr::synchrotron1, "synchrotron1"),
            "synchrotron2" => self.unary_op(args, Expr::synchrotron2, "synchrotron2"),
            "transport" => self.binary_op(args, Expr::transport, "transport"),
            "fermi_dirac" => self.binary_op(args, Expr::fermi_dirac, "fermi_dirac"),

            // 未知运算符 - 作为变量处理
            _ => {
                Err(SExprError::UnknownOperator {
                    op: op.to_string(),
                    span: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::parse;
    
    fn parse_and_convert(input: &str) -> SExprResult<Expr> {
        let sexpr = parse(input)?;
        convert(&sexpr)
    }
    
    #[test]
    fn test_number() {
        let expr = parse_and_convert("42").unwrap();
        assert!(matches!(expr, Expr::Const(n) if (n - 42.0).abs() < 1e-10));
    }
    
    #[test]
    fn test_symbol() {
        let expr = parse_and_convert("x").unwrap();
        assert!(matches!(expr, Expr::Var(ref s) if s == "x"));
    }
    
    #[test]
    fn test_pi() {
        let expr = parse_and_convert("pi").unwrap();
        assert!(matches!(expr, Expr::Pi));
    }
    
    #[test]
    fn test_e() {
        let expr = parse_and_convert("e").unwrap();
        assert!(matches!(expr, Expr::E));
    }
    
    #[test]
    fn test_add() {
        let expr = parse_and_convert("(add 1 2)").unwrap();
        if let Expr::Add(a, b) = expr {
            assert!(matches!(*a, Expr::Const(n) if (n - 1.0).abs() < 1e-10));
            assert!(matches!(*b, Expr::Const(n) if (n - 2.0).abs() < 1e-10));
        } else {
            panic!("Expected Add");
        }
    }
    
    #[test]
    fn test_nested() {
        let expr = parse_and_convert("(mul (add x 1) y)").unwrap();
        assert!(matches!(expr, Expr::Mul(_, _)));
    }
    
    #[test]
    fn test_if() {
        let expr = parse_and_convert("(if (gt x 0) (sqrt x) 0)").unwrap();
        assert!(matches!(expr, Expr::IfThenElse { .. }));
    }
    
    #[test]
    fn test_sum() {
        let expr = parse_and_convert("(sum i 1 n (pow i 2))").unwrap();
        if let Expr::Sum { index, .. } = expr {
            assert_eq!(index, "i");
        } else {
            panic!("Expected Sum");
        }
    }
    
    #[test]
    fn test_trig_functions() {
        let _ = parse_and_convert("(sin x)").unwrap();
        let _ = parse_and_convert("(cos x)").unwrap();
        let _ = parse_and_convert("(tan x)").unwrap();
        let _ = parse_and_convert("(asin x)").unwrap();
        let _ = parse_and_convert("(acos x)").unwrap();
        let _ = parse_and_convert("(atan x)").unwrap();
    }
    
    #[test]
    fn test_hyperbolic() {
        let _ = parse_and_convert("(sinh x)").unwrap();
        let _ = parse_and_convert("(cosh x)").unwrap();
        let _ = parse_and_convert("(tanh x)").unwrap();
    }
    
    #[test]
    fn test_special_functions() {
        let _ = parse_and_convert("(gamma x)").unwrap();
        let _ = parse_and_convert("(erf x)").unwrap();
        let _ = parse_and_convert("(factorial 5)").unwrap();
    }
    
    #[test]
    fn test_wrong_arg_count() {
        let result = parse_and_convert("(add 1 2 3)");
        assert!(result.is_err());
        if let Err(SExprError::WrongArgCount { op, expected, found }) = result {
            assert_eq!(op, "add");
            assert_eq!(expected, "2");
            assert_eq!(found, 3);
        }
    }
    
    #[test]
    fn test_unknown_operator() {
        let result = parse_and_convert("(foobar x y)");
        assert!(result.is_err());
        assert!(matches!(result, Err(SExprError::UnknownOperator { .. })));
    }
    
    #[test]
    fn test_complex_expression() {
        let expr = parse_and_convert("(div (add (mul 2 x) y) (sub z 1))").unwrap();
        assert!(matches!(expr, Expr::Div(_, _)));
    }
}
