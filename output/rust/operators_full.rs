//! 完整运算符测试 (OPERATORS_FULL)
//! 模型: OperatorsFull
//! 自动生成的代码，请勿手动编辑

use crate::lowcode::core::metadata::{InputDef, OutputDef};
use crate::lowcode::core::types::DataType;
use crate::lowcode::core::{LowcodeError, Operator, OperatorMetadata};
use serde_json::{json, Value};

/// 逆误差函数
///
/// 方程 ID: `test_erfinv`

pub struct TestErfinvOperator;

impl Operator for TestErfinvOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_erfinv", "逆误差函数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_erfinv", DataType::Number).with_description("逆误差函数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = statrs::function::erf::erf_inv(p1);

        Ok(json!({ "y_erfinv": result }))
    }
}

/// Sinc函数
///
/// 方程 ID: `test_sinc`

pub struct TestSincOperator;

impl Operator for TestSincOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_sinc", "Sinc函数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_sinc", DataType::Number).with_description("Sinc函数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = { let x = p1; if x == 0.0 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) } };

        Ok(json!({ "y_sinc": result }))
    }
}

/// Trigamma函数
///
/// 方程 ID: `test_trigamma`

pub struct TestTrigammaOperator;

impl Operator for TestTrigammaOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_trigamma", "Trigamma函数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .output(OutputDef::new("y_trigamma", DataType::Number).with_description("Trigamma函数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let result = statrs::function::gamma::trigamma(p3);

        Ok(json!({ "y_trigamma": result }))
    }
}

/// 指数分布CDF
///
/// 方程 ID: `test_exp_cdf`

pub struct TestExpCdfOperator;

impl Operator for TestExpCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_exp_cdf", "指数分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p5", DataType::Number).with_description("参数λ").with_default(json!(1.0)))
            .output(OutputDef::new("y_exp_cdf", DataType::Number).with_description("指数分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Exp, ContinuousCDF}; Exp::new(p5).unwrap().cdf(p1) };

        Ok(json!({ "y_exp_cdf": result }))
    }
}

/// 指数分布PDF
///
/// 方程 ID: `test_exp_pdf`

pub struct TestExpPdfOperator;

impl Operator for TestExpPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_exp_pdf", "指数分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p5", DataType::Number).with_description("参数λ").with_default(json!(1.0)))
            .output(OutputDef::new("y_exp_pdf", DataType::Number).with_description("指数分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Exp, Continuous}; Exp::new(p5).unwrap().pdf(p1) };

        Ok(json!({ "y_exp_pdf": result }))
    }
}

/// 均匀分布CDF
///
/// 方程 ID: `test_uniform_cdf`

pub struct TestUniformCdfOperator;

impl Operator for TestUniformCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_uniform_cdf", "均匀分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_uniform_cdf", DataType::Number).with_description("均匀分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = { use statrs::distribution::{Uniform, ContinuousCDF}; Uniform::new(0_f64, 1_f64).unwrap().cdf(p1) };

        Ok(json!({ "y_uniform_cdf": result }))
    }
}

/// 均匀分布PDF
///
/// 方程 ID: `test_uniform_pdf`

pub struct TestUniformPdfOperator;

impl Operator for TestUniformPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_uniform_pdf", "均匀分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_uniform_pdf", DataType::Number).with_description("均匀分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = { use statrs::distribution::{Uniform, Continuous}; Uniform::new(0_f64, 1_f64).unwrap().pdf(p1) };

        Ok(json!({ "y_uniform_pdf": result }))
    }
}

/// 伽马分布CDF
///
/// 方程 ID: `test_gamma_cdf`

pub struct TestGammaCdfOperator;

impl Operator for TestGammaCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_gamma_cdf", "伽马分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .input(InputDef::optional("p4", DataType::Number).with_description("参数β").with_default(json!(3.0)))
            .output(OutputDef::new("y_gamma_cdf", DataType::Number).with_description("伽马分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let result = { use statrs::distribution::{Gamma, ContinuousCDF}; Gamma::new(p3, 1.0/p4).unwrap().cdf(p1) };

        Ok(json!({ "y_gamma_cdf": result }))
    }
}

/// 伽马分布PDF
///
/// 方程 ID: `test_gamma_pdf`

pub struct TestGammaPdfOperator;

impl Operator for TestGammaPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_gamma_pdf", "伽马分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .input(InputDef::optional("p4", DataType::Number).with_description("参数β").with_default(json!(3.0)))
            .output(OutputDef::new("y_gamma_pdf", DataType::Number).with_description("伽马分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let result = { use statrs::distribution::{Gamma, Continuous}; Gamma::new(p3, 1.0/p4).unwrap().pdf(p1) };

        Ok(json!({ "y_gamma_pdf": result }))
    }
}

/// 贝塔分布CDF
///
/// 方程 ID: `test_beta_cdf`

pub struct TestBetaCdfOperator;

impl Operator for TestBetaCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_beta_cdf", "贝塔分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .input(InputDef::optional("p4", DataType::Number).with_description("参数β").with_default(json!(3.0)))
            .output(OutputDef::new("y_beta_cdf", DataType::Number).with_description("贝塔分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let result = { use statrs::distribution::{Beta, ContinuousCDF}; Beta::new(p3, p4).unwrap().cdf(p1) };

        Ok(json!({ "y_beta_cdf": result }))
    }
}

/// 贝塔分布PDF
///
/// 方程 ID: `test_beta_pdf`

pub struct TestBetaPdfOperator;

impl Operator for TestBetaPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_beta_pdf", "贝塔分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .input(InputDef::optional("p4", DataType::Number).with_description("参数β").with_default(json!(3.0)))
            .output(OutputDef::new("y_beta_pdf", DataType::Number).with_description("贝塔分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let result = { use statrs::distribution::{Beta, Continuous}; Beta::new(p3, p4).unwrap().pdf(p1) };

        Ok(json!({ "y_beta_pdf": result }))
    }
}

/// F分布CDF
///
/// 方程 ID: `test_f_cdf`

pub struct TestFCdfOperator;

impl Operator for TestFCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_f_cdf", "F分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p11", DataType::Number).with_description("自由度1").with_default(json!(5.0)))
            .input(InputDef::optional("p12", DataType::Number).with_description("自由度2").with_default(json!(10.0)))
            .output(OutputDef::new("y_f_cdf", DataType::Number).with_description("F分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p11 = input
            .get("p11")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let p12 = input
            .get("p12")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let result = { use statrs::distribution::{FisherSnedecor, ContinuousCDF}; FisherSnedecor::new(p11, p12).unwrap().cdf(p1) };

        Ok(json!({ "y_f_cdf": result }))
    }
}

/// F分布PDF
///
/// 方程 ID: `test_f_pdf`

pub struct TestFPdfOperator;

impl Operator for TestFPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_f_pdf", "F分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p11", DataType::Number).with_description("自由度1").with_default(json!(5.0)))
            .input(InputDef::optional("p12", DataType::Number).with_description("自由度2").with_default(json!(10.0)))
            .output(OutputDef::new("y_f_pdf", DataType::Number).with_description("F分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p11 = input
            .get("p11")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let p12 = input
            .get("p12")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let result = { use statrs::distribution::{FisherSnedecor, Continuous}; FisherSnedecor::new(p11, p12).unwrap().pdf(p1) };

        Ok(json!({ "y_f_pdf": result }))
    }
}

/// 威布尔分布CDF
///
/// 方程 ID: `test_weibull_cdf`

pub struct TestWeibullCdfOperator;

impl Operator for TestWeibullCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_weibull_cdf", "威布尔分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .input(InputDef::optional("p5", DataType::Number).with_description("参数λ").with_default(json!(1.0)))
            .output(OutputDef::new("y_weibull_cdf", DataType::Number).with_description("威布尔分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Weibull, ContinuousCDF}; Weibull::new(p3, p5).unwrap().cdf(p1) };

        Ok(json!({ "y_weibull_cdf": result }))
    }
}

/// 威布尔分布PDF
///
/// 方程 ID: `test_weibull_pdf`

pub struct TestWeibullPdfOperator;

impl Operator for TestWeibullPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_weibull_pdf", "威布尔分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("参数α").with_default(json!(2.0)))
            .input(InputDef::optional("p5", DataType::Number).with_description("参数λ").with_default(json!(1.0)))
            .output(OutputDef::new("y_weibull_pdf", DataType::Number).with_description("威布尔分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Weibull, Continuous}; Weibull::new(p3, p5).unwrap().pdf(p1) };

        Ok(json!({ "y_weibull_pdf": result }))
    }
}

/// 对数正态分布CDF
///
/// 方程 ID: `test_lognorm_cdf`

pub struct TestLognormCdfOperator;

impl Operator for TestLognormCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_lognorm_cdf", "对数正态分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p6", DataType::Number).with_description("均值μ").with_default(json!(0.0)))
            .input(InputDef::optional("p7", DataType::Number).with_description("标准差σ").with_default(json!(1.0)))
            .output(OutputDef::new("y_lognorm_cdf", DataType::Number).with_description("对数正态分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p6 = input
            .get("p6")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let p7 = input
            .get("p7")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{LogNormal, ContinuousCDF}; LogNormal::new(p6, p7).unwrap().cdf(1_f64) };

        Ok(json!({ "y_lognorm_cdf": result }))
    }
}

/// 对数正态分布PDF
///
/// 方程 ID: `test_lognorm_pdf`

pub struct TestLognormPdfOperator;

impl Operator for TestLognormPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_lognorm_pdf", "对数正态分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p6", DataType::Number).with_description("均值μ").with_default(json!(0.0)))
            .input(InputDef::optional("p7", DataType::Number).with_description("标准差σ").with_default(json!(1.0)))
            .output(OutputDef::new("y_lognorm_pdf", DataType::Number).with_description("对数正态分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p6 = input
            .get("p6")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let p7 = input
            .get("p7")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{LogNormal, Continuous}; LogNormal::new(p6, p7).unwrap().pdf(1_f64) };

        Ok(json!({ "y_lognorm_pdf": result }))
    }
}

/// 柯西分布CDF
///
/// 方程 ID: `test_cauchy_cdf`

pub struct TestCauchyCdfOperator;

impl Operator for TestCauchyCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_cauchy_cdf", "柯西分布CDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p6", DataType::Number).with_description("均值μ").with_default(json!(0.0)))
            .input(InputDef::optional("p7", DataType::Number).with_description("标准差σ").with_default(json!(1.0)))
            .output(OutputDef::new("y_cauchy_cdf", DataType::Number).with_description("柯西分布CDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p6 = input
            .get("p6")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let p7 = input
            .get("p7")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Cauchy, ContinuousCDF}; Cauchy::new(p6, p7).unwrap().cdf(p1) };

        Ok(json!({ "y_cauchy_cdf": result }))
    }
}

/// 柯西分布PDF
///
/// 方程 ID: `test_cauchy_pdf`

pub struct TestCauchyPdfOperator;

impl Operator for TestCauchyPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_cauchy_pdf", "柯西分布PDF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .input(InputDef::optional("p6", DataType::Number).with_description("均值μ").with_default(json!(0.0)))
            .input(InputDef::optional("p7", DataType::Number).with_description("标准差σ").with_default(json!(1.0)))
            .output(OutputDef::new("y_cauchy_pdf", DataType::Number).with_description("柯西分布PDF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p6 = input
            .get("p6")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let p7 = input
            .get("p7")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Cauchy, Continuous}; Cauchy::new(p6, p7).unwrap().pdf(p1) };

        Ok(json!({ "y_cauchy_pdf": result }))
    }
}

/// 二项分布PMF
///
/// 方程 ID: `test_binom_pmf`

pub struct TestBinomPmfOperator;

impl Operator for TestBinomPmfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_binom_pmf", "二项分布PMF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p9", DataType::Number).with_description("成功次数").with_default(json!(3.0)))
            .input(InputDef::optional("p8", DataType::Number).with_description("试验次数").with_default(json!(10.0)))
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .output(OutputDef::new("y_binom_pmf", DataType::Number).with_description("二项分布PMF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p9 = input
            .get("p9")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let p8 = input
            .get("p8")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = { use statrs::distribution::{Binomial, Discrete}; Binomial::new(p2, p8 as u64).unwrap().pmf(p9 as u64) };

        Ok(json!({ "y_binom_pmf": result }))
    }
}

/// 几何分布PMF
///
/// 方程 ID: `test_geom_pmf`

pub struct TestGeomPmfOperator;

impl Operator for TestGeomPmfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_geom_pmf", "几何分布PMF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p9", DataType::Number).with_description("成功次数").with_default(json!(3.0)))
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .output(OutputDef::new("y_geom_pmf", DataType::Number).with_description("几何分布PMF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p9 = input
            .get("p9")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = { use statrs::distribution::{Geometric, Discrete}; Geometric::new(p2).unwrap().pmf(p9 as u64) };

        Ok(json!({ "y_geom_pmf": result }))
    }
}

/// 超几何分布PMF
///
/// 方程 ID: `test_hypergeom_pmf`

pub struct TestHypergeomPmfOperator;

impl Operator for TestHypergeomPmfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_hypergeom_pmf", "超几何分布PMF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")

            .output(OutputDef::new("y_hypergeom_pmf", DataType::Number).with_description("超几何分布PMF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = { use statrs::distribution::{Hypergeometric, Discrete}; Hypergeometric::new(20_f64 as u64, 7_f64 as u64, 12_f64 as u64).unwrap().pmf(2_f64 as u64) };

        Ok(json!({ "y_hypergeom_pmf": result }))
    }
}

/// 负二项分布PMF
///
/// 方程 ID: `test_neg_binom_pmf`

pub struct TestNegBinomPmfOperator;

impl Operator for TestNegBinomPmfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_neg_binom_pmf", "负二项分布PMF")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p9", DataType::Number).with_description("成功次数").with_default(json!(3.0)))
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .output(OutputDef::new("y_neg_binom_pmf", DataType::Number).with_description("负二项分布PMF 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p9 = input
            .get("p9")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = { use statrs::distribution::{NegativeBinomial, Discrete}; NegativeBinomial::new(5_f64, p2).unwrap().pmf(p9 as u64) };

        Ok(json!({ "y_neg_binom_pmf": result }))
    }
}

/// 正态分布分位数
///
/// 方程 ID: `test_norm_ppf`

pub struct TestNormPpfOperator;

impl Operator for TestNormPpfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_norm_ppf", "正态分布分位数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .input(InputDef::optional("p6", DataType::Number).with_description("均值μ").with_default(json!(0.0)))
            .input(InputDef::optional("p7", DataType::Number).with_description("标准差σ").with_default(json!(1.0)))
            .output(OutputDef::new("y_norm_ppf", DataType::Number).with_description("正态分布分位数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p6 = input
            .get("p6")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let p7 = input
            .get("p7")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = { use statrs::distribution::{Normal, InverseCDF}; Normal::new(p6, p7).unwrap().inverse_cdf(p2) };

        Ok(json!({ "y_norm_ppf": result }))
    }
}

/// t分布分位数
///
/// 方程 ID: `test_t_ppf`

pub struct TestTPpfOperator;

impl Operator for TestTPpfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_t_ppf", "t分布分位数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .input(InputDef::optional("p10", DataType::Number).with_description("自由度").with_default(json!(5.0)))
            .output(OutputDef::new("y_t_ppf", DataType::Number).with_description("t分布分位数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p10 = input
            .get("p10")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let result = { use statrs::distribution::{StudentsT, InverseCDF}; StudentsT::new(0.0, 1.0, p10).unwrap().inverse_cdf(p2) };

        Ok(json!({ "y_t_ppf": result }))
    }
}

/// 卡方分布分位数
///
/// 方程 ID: `test_chi2_ppf`

pub struct TestChi2PpfOperator;

impl Operator for TestChi2PpfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_chi2_ppf", "卡方分布分位数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .input(InputDef::optional("p10", DataType::Number).with_description("自由度").with_default(json!(5.0)))
            .output(OutputDef::new("y_chi2_ppf", DataType::Number).with_description("卡方分布分位数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p10 = input
            .get("p10")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let result = { use statrs::distribution::{ChiSquared, InverseCDF}; ChiSquared::new(p10).unwrap().inverse_cdf(p2) };

        Ok(json!({ "y_chi2_ppf": result }))
    }
}

/// F分布分位数
///
/// 方程 ID: `test_f_ppf`

pub struct TestFPpfOperator;

impl Operator for TestFPpfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_f_ppf", "F分布分位数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p2", DataType::Number).with_description("概率p").with_default(json!(0.5)))
            .input(InputDef::optional("p11", DataType::Number).with_description("自由度1").with_default(json!(5.0)))
            .input(InputDef::optional("p12", DataType::Number).with_description("自由度2").with_default(json!(10.0)))
            .output(OutputDef::new("y_f_ppf", DataType::Number).with_description("F分布分位数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let p11 = input
            .get("p11")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let p12 = input
            .get("p12")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let result = { use statrs::distribution::{FisherSnedecor, InverseCDF}; FisherSnedecor::new(p11, p12).unwrap().inverse_cdf(p2) };

        Ok(json!({ "y_f_ppf": result }))
    }
}

/// 复数指数
///
/// 方程 ID: `test_complex_exp`

pub struct TestComplexExpOperator;

impl Operator for TestComplexExpOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_exp", "复数指数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")

            .output(OutputDef::new("y_complex_exp", DataType::Number).with_description("复数指数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = num_complex::Complex::new(0_f64, 3.14159_f64).exp();

        Ok(json!({ "y_complex_exp": result }))
    }
}

/// 复数对数
///
/// 方程 ID: `test_complex_ln`

pub struct TestComplexLnOperator;

impl Operator for TestComplexLnOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_ln", "复数对数")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")

            .output(OutputDef::new("y_complex_ln", DataType::Number).with_description("复数对数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = num_complex::Complex::new(1_f64, 1_f64).ln();

        Ok(json!({ "y_complex_ln": result }))
    }
}

/// 复数正弦
///
/// 方程 ID: `test_complex_sin`

pub struct TestComplexSinOperator;

impl Operator for TestComplexSinOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_sin", "复数正弦")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_complex_sin", DataType::Number).with_description("复数正弦 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = num_complex::Complex::new(p1, 0_f64).sin();

        Ok(json!({ "y_complex_sin": result }))
    }
}

/// 复数余弦
///
/// 方程 ID: `test_complex_cos`

pub struct TestComplexCosOperator;

impl Operator for TestComplexCosOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_cos", "复数余弦")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_complex_cos", DataType::Number).with_description("复数余弦 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = num_complex::Complex::new(p1, 0_f64).cos();

        Ok(json!({ "y_complex_cos": result }))
    }
}

/// 复数正切
///
/// 方程 ID: `test_complex_tan`

pub struct TestComplexTanOperator;

impl Operator for TestComplexTanOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_tan", "复数正切")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("输入x").with_default(json!(0.5)))
            .output(OutputDef::new("y_complex_tan", DataType::Number).with_description("复数正切 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = num_complex::Complex::new(p1, 0_f64).tan();

        Ok(json!({ "y_complex_tan": result }))
    }
}

/// 复数平方根
///
/// 方程 ID: `test_complex_sqrt`

pub struct TestComplexSqrtOperator;

impl Operator for TestComplexSqrtOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_sqrt", "复数平方根")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")

            .output(OutputDef::new("y_complex_sqrt", DataType::Number).with_description("复数平方根 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = num_complex::Complex::new(-1_f64, 0_f64).sqrt();

        Ok(json!({ "y_complex_sqrt": result }))
    }
}

/// 复数幂
///
/// 方程 ID: `test_complex_pow`

pub struct TestComplexPowOperator;

impl Operator for TestComplexPowOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_pow", "复数幂")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")

            .output(OutputDef::new("y_complex_pow", DataType::Number).with_description("复数幂 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = num_complex::Complex::new(2_f64, 1_f64).powc(num_complex::Complex::new(0.5_f64, 0_f64));

        Ok(json!({ "y_complex_pow": result }))
    }
}

/// 复数范数平方
///
/// 方程 ID: `test_complex_norm_sqr`

pub struct TestComplexNormSqrOperator;

impl Operator for TestComplexNormSqrOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_full.test_complex_norm_sqr", "复数范数平方")
            .category("operators_full")
            .description("测试特殊函数补充、概率分布扩展、分位数函数、复数运算扩展")
            .version("1.0.0")

            .output(OutputDef::new("y_complex_norm_sqr", DataType::Number).with_description("复数范数平方 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = num_complex::Complex::new(3_f64, 4_f64).norm_sqr();

        Ok(json!({ "y_complex_norm_sqr": result }))
    }
}
