//! 类型检查器
//!
//! 检查表达式中运算数类型的兼容性。

use crate::ast::Expr;
use crate::error::ValidationError;
use crate::schema::{DataType, EquationFile};

/// 表达式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprType {
    /// 数值类型（float/int）
    Numeric,
    /// 布尔类型
    Boolean,
    /// 未知类型
    Unknown,
}

/// 检查文件中所有方程的类型正确性
pub fn check_types(file: &EquationFile) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for equation in &file.equations {
        if let Err(e) = infer_type(&equation.expression, file) {
            errors.push(ValidationError::TypeError {
                message: e,
                location: format!("方程 {}", equation.id),
            });
        }
    }

    errors
}

/// 推断表达式类型并检查类型兼容性
pub fn infer_type(expr: &Expr, file: &EquationFile) -> Result<ExprType, String> {
    match expr {
        // 常量始终是数值类型
        Expr::Const(_) | Expr::Pi | Expr::E => Ok(ExprType::Numeric),

        // 变量和参数类型从定义中获取
        Expr::Var(name) => {
            if let Some(var) = file.variables.get(name) {
                Ok(data_type_to_expr_type(&var.dtype))
            } else {
                // 可能是中间变量（其他方程的输出）
                Ok(ExprType::Numeric)
            }
        }
        Expr::Param(name) => {
            if let Some(param) = file.parameters.get(name) {
                Ok(data_type_to_expr_type(&param.dtype))
            } else {
                Ok(ExprType::Numeric) // 默认数值
            }
        }

        // 算术运算需要数值操作数，返回数值
        Expr::Add(a, b)
        | Expr::Sub(a, b)
        | Expr::Mul(a, b)
        | Expr::Div(a, b)
        | Expr::Pow(a, b)
        | Expr::Mod(a, b)
        | Expr::ATan2(a, b) => {
            check_numeric_binary(a, b, file, "算术运算")
        }

        Expr::Neg(a)
        | Expr::Abs(a)
        | Expr::Ceil(a)
        | Expr::Floor(a)
        | Expr::Round(a)
        | Expr::Trunc(a)
        | Expr::Sign(a) => {
            check_numeric_unary(a, file, "算术运算")
        }

        // 超越函数需要数值操作数，返回数值
        Expr::Exp(a)
        | Expr::Ln(a)
        | Expr::Log10(a)
        | Expr::Log2(a)
        | Expr::Sqrt(a)
        | Expr::Cbrt(a) => {
            check_numeric_unary(a, file, "超越函数")
        }

        // 三角函数需要数值操作数，返回数值
        Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::ASin(a)
        | Expr::ACos(a)
        | Expr::ATan(a) => {
            check_numeric_unary(a, file, "三角函数")
        }

        // 双曲函数需要数值操作数，返回数值
        Expr::Sinh(a)
        | Expr::Cosh(a)
        | Expr::Tanh(a)
        | Expr::ASinh(a)
        | Expr::ACosh(a)
        | Expr::ATanh(a) => {
            check_numeric_unary(a, file, "双曲函数")
        }

        // 聚合函数需要数值操作数，返回数值
        Expr::Max(args) | Expr::Min(args) => {
            for arg in args {
                let t = infer_type(arg, file)?;
                if t != ExprType::Numeric {
                    return Err(format!("聚合函数参数必须是数值类型，实际是 {:?}", t));
                }
            }
            Ok(ExprType::Numeric)
        }

        // 求和需要数值边界和体，返回数值
        Expr::Sum { lower, upper, body, .. } => {
            let lt = infer_type(lower, file)?;
            let ut = infer_type(upper, file)?;
            let bt = infer_type(body, file)?;

            if lt != ExprType::Numeric {
                return Err("求和下界必须是数值类型".to_string());
            }
            if ut != ExprType::Numeric {
                return Err("求和上界必须是数值类型".to_string());
            }
            if bt != ExprType::Numeric {
                return Err("求和体必须是数值类型".to_string());
            }
            Ok(ExprType::Numeric)
        }

        // 连乘需要数值边界和体，返回数值
        Expr::Product { lower, upper, body, .. } => {
            let lt = infer_type(lower, file)?;
            let ut = infer_type(upper, file)?;
            let bt = infer_type(body, file)?;

            if lt != ExprType::Numeric {
                return Err("连乘下界必须是数值类型".to_string());
            }
            if ut != ExprType::Numeric {
                return Err("连乘上界必须是数值类型".to_string());
            }
            if bt != ExprType::Numeric {
                return Err("连乘体必须是数值类型".to_string());
            }
            Ok(ExprType::Numeric)
        }

        // 关系运算需要数值操作数，返回布尔值
        Expr::Eq(a, b)
        | Expr::Lt(a, b)
        | Expr::Gt(a, b)
        | Expr::Leq(a, b)
        | Expr::Geq(a, b)
        | Expr::Neq(a, b) => {
            let ta = infer_type(a, file)?;
            let tb = infer_type(b, file)?;

            if ta != ExprType::Numeric || tb != ExprType::Numeric {
                return Err(format!(
                    "关系运算两侧必须是数值类型，实际是 {:?} 和 {:?}",
                    ta, tb
                ));
            }
            Ok(ExprType::Boolean)
        }

        // 逻辑运算需要布尔操作数，返回布尔值
        Expr::And(a, b) | Expr::Or(a, b) => {
            let ta = infer_type(a, file)?;
            let tb = infer_type(b, file)?;

            if ta != ExprType::Boolean || tb != ExprType::Boolean {
                return Err(format!(
                    "逻辑运算两侧必须是布尔类型，实际是 {:?} 和 {:?}",
                    ta, tb
                ));
            }
            Ok(ExprType::Boolean)
        }

        Expr::Not(a) => {
            let t = infer_type(a, file)?;
            if t != ExprType::Boolean {
                return Err(format!("逻辑非操作数必须是布尔类型，实际是 {:?}", t));
            }
            Ok(ExprType::Boolean)
        }

        // 条件表达式：条件必须是布尔，两个分支类型必须一致
        Expr::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => {
            let tc = infer_type(cond, file)?;
            if tc != ExprType::Boolean {
                return Err(format!("条件表达式的条件必须是布尔类型，实际是 {:?}", tc));
            }

            let tt = infer_type(then_branch, file)?;
            let te = infer_type(else_branch, file)?;

            if tt != te {
                return Err(format!(
                    "条件表达式的两个分支类型必须一致，实际是 {:?} 和 {:?}",
                    tt, te
                ));
            }
            Ok(tt)
        }

        // 分段函数：所有条件必须是布尔，所有值类型必须一致
        Expr::Piecewise { pieces, otherwise } => {
            let mut value_type = None;

            for (cond, value) in pieces {
                let tc = infer_type(cond, file)?;
                if tc != ExprType::Boolean {
                    return Err(format!(
                        "分段函数的条件必须是布尔类型，实际是 {:?}",
                        tc
                    ));
                }

                let tv = infer_type(value, file)?;
                if let Some(expected) = value_type {
                    if tv != expected {
                        return Err(format!(
                            "分段函数的所有值类型必须一致，期望 {:?}，实际 {:?}",
                            expected, tv
                        ));
                    }
                } else {
                    value_type = Some(tv);
                }
            }

            let to = infer_type(otherwise, file)?;
            if let Some(expected) = value_type {
                if to != expected {
                    return Err(format!(
                        "分段函数的 otherwise 类型必须与其他值一致，期望 {:?}，实际 {:?}",
                        expected, to
                    ));
                }
                Ok(expected)
            } else {
                Ok(to)
            }
        }

        // 扩展运算符 - 默认返回数值类型
        _ => Ok(ExprType::Numeric),
    }
}

/// 检查一元数值运算
fn check_numeric_unary(arg: &Expr, file: &EquationFile, op_name: &str) -> Result<ExprType, String> {
    let t = infer_type(arg, file)?;
    if t != ExprType::Numeric {
        return Err(format!("{}操作数必须是数值类型，实际是 {:?}", op_name, t));
    }
    Ok(ExprType::Numeric)
}

/// 检查二元数值运算
fn check_numeric_binary(
    left: &Expr,
    right: &Expr,
    file: &EquationFile,
    op_name: &str,
) -> Result<ExprType, String> {
    let tl = infer_type(left, file)?;
    let tr = infer_type(right, file)?;

    if tl != ExprType::Numeric || tr != ExprType::Numeric {
        return Err(format!(
            "{}两侧必须是数值类型，实际是 {:?} 和 {:?}",
            op_name, tl, tr
        ));
    }
    Ok(ExprType::Numeric)
}

/// 将 DataType 转换为 ExprType
fn data_type_to_expr_type(dtype: &DataType) -> ExprType {
    match dtype {
        DataType::Float | DataType::Int => ExprType::Numeric,
        DataType::Bool => ExprType::Boolean,
        DataType::String => ExprType::Unknown,
        DataType::Array(_) => ExprType::Numeric, // 简化处理
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Metadata, Parameter};
    use indexmap::IndexMap;

    fn create_test_file() -> EquationFile {
        let mut parameters = IndexMap::new();
        parameters.insert(
            "p1".to_string(),
            Parameter {
                name_cn: "参数1".to_string(),
                name_en: None,
                dtype: DataType::Float,
                default: 1.0,
                values: None,
                unit: None,
                bounds: None,
                optimizable: false,
                management: false,
                description: None,
            },
        );

        EquationFile {
            meta: Metadata {
                id: "TEST".to_string(),
                model: "Test".to_string(),
                name_cn: "测试".to_string(),
                name_en: None,
                version: "1.0".to_string(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                calibration: None,
                modules: Default::default(),
            },
            parameters,
            variables: Default::default(),
            equations: vec![],
        }
    }

    #[test]
    fn test_arithmetic_type() {
        let file = create_test_file();
        let expr = Expr::add(Expr::param("p1"), Expr::constant(2.0));
        
        let result = infer_type(&expr, &file);
        assert_eq!(result.unwrap(), ExprType::Numeric);
    }

    #[test]
    fn test_new_operators_type() {
        let file = create_test_file();
        
        // 测试新增算子
        let ceil = Expr::ceil(Expr::param("p1"));
        assert_eq!(infer_type(&ceil, &file).unwrap(), ExprType::Numeric);

        let atan2 = Expr::atan2(Expr::param("p1"), Expr::constant(1.0));
        assert_eq!(infer_type(&atan2, &file).unwrap(), ExprType::Numeric);

        let sinh = Expr::sinh(Expr::param("p1"));
        assert_eq!(infer_type(&sinh, &file).unwrap(), ExprType::Numeric);

        let modulo = Expr::modulo(Expr::param("p1"), Expr::constant(2.0));
        assert_eq!(infer_type(&modulo, &file).unwrap(), ExprType::Numeric);
    }

    #[test]
    fn test_comparison_type() {
        let file = create_test_file();
        let expr = Expr::Gt(
            Box::new(Expr::param("p1")),
            Box::new(Expr::constant(0.0)),
        );
        
        let result = infer_type(&expr, &file);
        assert_eq!(result.unwrap(), ExprType::Boolean);
    }

    #[test]
    fn test_logical_type() {
        let file = create_test_file();
        let expr = Expr::And(
            Box::new(Expr::Gt(Box::new(Expr::constant(1.0)), Box::new(Expr::constant(0.0)))),
            Box::new(Expr::Lt(Box::new(Expr::constant(1.0)), Box::new(Expr::constant(2.0)))),
        );
        
        let result = infer_type(&expr, &file);
        assert_eq!(result.unwrap(), ExprType::Boolean);
    }

    #[test]
    fn test_if_then_else_type() {
        let file = create_test_file();
        let expr = Expr::if_then_else(
            Expr::Gt(Box::new(Expr::constant(1.0)), Box::new(Expr::constant(0.0))),
            Expr::constant(1.0),
            Expr::constant(0.0),
        );
        
        let result = infer_type(&expr, &file);
        assert_eq!(result.unwrap(), ExprType::Numeric);
    }

    #[test]
    fn test_type_mismatch_in_conditional() {
        let file = create_test_file();
        // 条件不是布尔类型
        let expr = Expr::if_then_else(
            Expr::constant(1.0), // 错误：应该是布尔类型
            Expr::constant(1.0),
            Expr::constant(0.0),
        );
        
        let result = infer_type(&expr, &file);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_mismatch_in_logical() {
        let file = create_test_file();
        // 逻辑运算操作数不是布尔类型
        let expr = Expr::And(
            Box::new(Expr::constant(1.0)), // 错误：应该是布尔类型
            Box::new(Expr::constant(2.0)), // 错误：应该是布尔类型
        );
        
        let result = infer_type(&expr, &file);
        assert!(result.is_err());
    }

    #[test]
    fn test_constants_type() {
        let file = create_test_file();
        
        assert_eq!(infer_type(&Expr::Pi, &file).unwrap(), ExprType::Numeric);
        assert_eq!(infer_type(&Expr::E, &file).unwrap(), ExprType::Numeric);
    }
}
