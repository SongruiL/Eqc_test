//! 高级运算符测试 (OPERATORS_ADVANCED)
//! 模型: OperatorsAdvanced
//! 自动生成的代码，请勿手动编辑

use crate::lowcode::core::metadata::{InputDef, OutputDef};
use crate::lowcode::core::types::DataType;
use crate::lowcode::core::{LowcodeError, Operator, OperatorMetadata};
use serde_json::{json, Value};

/// Gamma函数测试
///
/// 方程 ID: `test_gamma`

pub struct TestGammaOperator;

impl Operator for TestGammaOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_gamma", "Gamma函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_gamma", DataType::Number).with_description("Gamma函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = puruspe::gamma(p1);

        Ok(json!({ "y_gamma": result }))
    }
}

/// LogGamma函数测试
///
/// 方程 ID: `test_loggamma`

pub struct TestLoggammaOperator;

impl Operator for TestLoggammaOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_loggamma", "LogGamma函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_loggamma", DataType::Number).with_description("LogGamma函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = puruspe::loggamma(p1);

        Ok(json!({ "y_loggamma": result }))
    }
}

/// Beta函数测试
///
/// 方程 ID: `test_beta`

pub struct TestBetaOperator;

impl Operator for TestBetaOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_beta", "Beta函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_beta", DataType::Number).with_description("Beta函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.5);

        let result = puruspe::beta(p1, p2);

        Ok(json!({ "y_beta": result }))
    }
}

/// 不完全Beta函数测试
///
/// 方程 ID: `test_betainc`

pub struct TestBetaincOperator;

impl Operator for TestBetaincOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_betainc", "不完全Beta函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_betainc", DataType::Number).with_description("不完全Beta函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = puruspe::betainc(p1, p2, p3);

        Ok(json!({ "y_betainc": result }))
    }
}

/// 误差函数测试
///
/// 方程 ID: `test_erf`

pub struct TestErfOperator;

impl Operator for TestErfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_erf", "误差函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_erf", DataType::Number).with_description("误差函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = puruspe::erf(p3);

        Ok(json!({ "y_erf": result }))
    }
}

/// 补误差函数测试
///
/// 方程 ID: `test_erfc`

pub struct TestErfcOperator;

impl Operator for TestErfcOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_erfc", "补误差函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_erfc", DataType::Number).with_description("补误差函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = puruspe::erfc(p3);

        Ok(json!({ "y_erfc": result }))
    }
}

/// 第一类贝塞尔函数测试
///
/// 方程 ID: `test_besselj`

pub struct TestBesseljOperator;

impl Operator for TestBesseljOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_besselj", "第一类贝塞尔函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("阶数参数").with_default(json!(2.0)))
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_besselj", DataType::Number).with_description("第一类贝塞尔函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = puruspe::besselj(p5 as i32, p1);

        Ok(json!({ "y_besselj": result }))
    }
}

/// 第二类贝塞尔函数测试
///
/// 方程 ID: `test_bessely`

pub struct TestBesselyOperator;

impl Operator for TestBesselyOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_bessely", "第二类贝塞尔函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("阶数参数").with_default(json!(2.0)))
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_bessely", DataType::Number).with_description("第二类贝塞尔函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = puruspe::bessely(p5 as i32, p1);

        Ok(json!({ "y_bessely": result }))
    }
}

/// 修正贝塞尔第一类测试
///
/// 方程 ID: `test_besseli`

pub struct TestBesseliOperator;

impl Operator for TestBesseliOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_besseli", "修正贝塞尔第一类测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("阶数参数").with_default(json!(2.0)))
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_besseli", DataType::Number).with_description("修正贝塞尔第一类测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = puruspe::besseli(p5 as i32, p1);

        Ok(json!({ "y_besseli": result }))
    }
}

/// 修正贝塞尔第二类测试
///
/// 方程 ID: `test_besselk`

pub struct TestBesselkOperator;

impl Operator for TestBesselkOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_besselk", "修正贝塞尔第二类测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("阶数参数").with_default(json!(2.0)))
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_besselk", DataType::Number).with_description("修正贝塞尔第二类测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = puruspe::besselk(p5 as i32, p1);

        Ok(json!({ "y_besselk": result }))
    }
}

/// Digamma函数测试
///
/// 方程 ID: `test_digamma`

pub struct TestDigammaOperator;

impl Operator for TestDigammaOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_digamma", "Digamma函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_digamma", DataType::Number).with_description("Digamma函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = statrs::function::gamma::digamma(p1);

        Ok(json!({ "y_digamma": result }))
    }
}

/// 阶乘测试
///
/// 方程 ID: `test_factorial`

pub struct TestFactorialOperator;

impl Operator for TestFactorialOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_factorial", "阶乘测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p4", DataType::Number).with_description("整数参数").with_default(json!(5.0)))
            .output(OutputDef::new("y_factorial", DataType::Number).with_description("阶乘测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let result = statrs::function::factorial::factorial(p4 as u64);

        Ok(json!({ "y_factorial": result }))
    }
}

/// 正态分布CDF测试
///
/// 方程 ID: `test_norm_cdf`

pub struct TestNormCdfOperator;

impl Operator for TestNormCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_norm_cdf", "正态分布CDF测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_norm_cdf", DataType::Number).with_description("正态分布CDF测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = { use statrs::distribution::{Normal, ContinuousCDF}; Normal::new(0_f64, 1_f64).unwrap().cdf(0_f64) };

        Ok(json!({ "y_norm_cdf": result }))
    }
}

/// 正态分布PDF测试
///
/// 方程 ID: `test_norm_pdf`

pub struct TestNormPdfOperator;

impl Operator for TestNormPdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_norm_pdf", "正态分布PDF测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_norm_pdf", DataType::Number).with_description("正态分布PDF测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = { use statrs::distribution::{Normal, Continuous}; Normal::new(0_f64, 1_f64).unwrap().pdf(0_f64) };

        Ok(json!({ "y_norm_pdf": result }))
    }
}

/// 卡方分布CDF测试
///
/// 方程 ID: `test_chi2_cdf`

pub struct TestChi2CdfOperator;

impl Operator for TestChi2CdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_chi2_cdf", "卡方分布CDF测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p4", DataType::Number).with_description("整数参数").with_default(json!(5.0)))
            .output(OutputDef::new("y_chi2_cdf", DataType::Number).with_description("卡方分布CDF测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let result = { use statrs::distribution::{ChiSquared, ContinuousCDF}; ChiSquared::new(p4).unwrap().cdf(p1) };

        Ok(json!({ "y_chi2_cdf": result }))
    }
}

/// t分布CDF测试
///
/// 方程 ID: `test_t_cdf`

pub struct TestTCdfOperator;

impl Operator for TestTCdfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_t_cdf", "t分布CDF测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p4", DataType::Number).with_description("整数参数").with_default(json!(5.0)))
            .output(OutputDef::new("y_t_cdf", DataType::Number).with_description("t分布CDF测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let result = { use statrs::distribution::{StudentsT, ContinuousCDF}; StudentsT::new(0.0, 1.0, p4).unwrap().cdf(1_f64) };

        Ok(json!({ "y_t_cdf": result }))
    }
}

/// 泊松分布PMF测试
///
/// 方程 ID: `test_poisson_pmf`

pub struct TestPoissonPmfOperator;

impl Operator for TestPoissonPmfOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_poisson_pmf", "泊松分布PMF测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_poisson", DataType::Number).with_description("泊松分布PMF测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = { use statrs::distribution::{Poisson, Discrete}; Poisson::new(p1).unwrap().pmf(3_f64 as u64) };

        Ok(json!({ "y_poisson": result }))
    }
}

/// 组合数测试
///
/// 方程 ID: `test_binomial`

pub struct TestBinomialOperator;

impl Operator for TestBinomialOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_binomial", "组合数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_binomial", DataType::Number).with_description("组合数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = statrs::function::factorial::binomial(10_f64 as u64, 3_f64 as u64);

        Ok(json!({ "y_binomial": result }))
    }
}

/// 复数构造测试
///
/// 方程 ID: `test_complex`

pub struct TestComplexOperator;

impl Operator for TestComplexOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_complex", "复数构造测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_complex", DataType::Number).with_description("复数构造测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.5);

        let result = num_complex::Complex::new(p1, p2);

        Ok(json!({ "y_complex": result }))
    }
}

/// 复数实部测试
///
/// 方程 ID: `test_complex_real`

pub struct TestComplexRealOperator;

impl Operator for TestComplexRealOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_complex_real", "复数实部测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_real", DataType::Number).with_description("复数实部测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.5);

        let result = num_complex::Complex::new(p1, p2).re;

        Ok(json!({ "y_real": result }))
    }
}

/// 复数虚部测试
///
/// 方程 ID: `test_complex_imag`

pub struct TestComplexImagOperator;

impl Operator for TestComplexImagOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_complex_imag", "复数虚部测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_imag", DataType::Number).with_description("复数虚部测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.5);

        let result = num_complex::Complex::new(p1, p2).im;

        Ok(json!({ "y_imag": result }))
    }
}

/// 组合特殊函数测试
///
/// 方程 ID: `test_combined_special`

pub struct TestCombinedSpecialOperator;

impl Operator for TestCombinedSpecialOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_advanced.test_combined_special", "组合特殊函数测试")
            .category("operators_advanced")
            .description("测试特殊函数、概率分布、复数运算等高级数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_combined", DataType::Number).with_description("组合特殊函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = puruspe::gamma(p1) + puruspe::erf(p3);

        Ok(json!({ "y_combined": result }))
    }
}
