//! 数学运算演示 (MATH_DEMO)
//! 模型: MathDemo
//! 自动生成的代码，请勿手动编辑

use crate::lowcode::core::metadata::{InputDef, OutputDef};
use crate::lowcode::core::types::DataType;
use crate::lowcode::core::{LowcodeError, Operator, OperatorMetadata};
use serde_json::{json, Value};

/// 阻尼振荡
///
/// 方程 ID: `damped_oscillation`
///
/// 公式: `y = A \cdot e^{-\lambda t} \cdot \sin(2\pi \omega t)`
pub struct DampedOscillationOperator;

impl Operator for DampedOscillationOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.damped_oscillation", "阻尼振荡")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("t", DataType::Number).with_description("时间变量"))
            .input(InputDef::optional("p1", DataType::Number).with_description("振幅参数").with_default(json!(2.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("阻尼系数").with_default(json!(0.1)))
            .input(InputDef::optional("p2", DataType::Number).with_description("频率参数").with_default(json!(1.0)))
            .output(OutputDef::new("y_damped", DataType::Number).with_description("阻尼振荡 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let t = input
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("t".into()))?;

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let result = p1 * ((-(p3 * t)).exp() * ((2_f64 * std::f64::consts::PI) * (p2 * t)).sin());

        Ok(json!({ "y_damped": result }))
    }
}

/// 双曲正切激活
///
/// 方程 ID: `tanh_activation`
///
/// 公式: `y = \tanh(p_1 \cdot x)`
pub struct TanhActivationOperator;

impl Operator for TanhActivationOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.tanh_activation", "双曲正切激活")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .input(InputDef::optional("p1", DataType::Number).with_description("振幅参数").with_default(json!(2.5)))
            .output(OutputDef::new("y_tanh", DataType::Number).with_description("双曲正切激活 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = (p1 * x).tanh();

        Ok(json!({ "y_tanh": result }))
    }
}

/// 角度计算
///
/// 方程 ID: `angle_calc`
///
/// 公式: `\theta = \arctan2(x, t)`
pub struct AngleCalcOperator;

impl Operator for AngleCalcOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.angle_calc", "角度计算")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .input(InputDef::required("t", DataType::Number).with_description("时间变量"))
            .output(OutputDef::new("theta", DataType::Number).with_description("角度计算 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let t = input
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("t".into()))?;

        let result = x.atan2(t);

        Ok(json!({ "theta": result }))
    }
}

/// 取整运算
///
/// 方程 ID: `rounding_demo`
///
/// 公式: `y = \lfloor x \rfloor + \lceil t/2 \rceil`
pub struct RoundingDemoOperator;

impl Operator for RoundingDemoOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.rounding_demo", "取整运算")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .input(InputDef::required("t", DataType::Number).with_description("时间变量"))
            .output(OutputDef::new("y_round", DataType::Number).with_description("取整运算 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let t = input
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("t".into()))?;

        let result = x.floor() + (t / 2_f64).ceil();

        Ok(json!({ "y_round": result }))
    }
}

/// 符号与绝对值
///
/// 方程 ID: `sign_abs_demo`
///
/// 公式: `y = \text{sgn}(x) \cdot \sqrt{|x|}`
pub struct SignAbsDemoOperator;

impl Operator for SignAbsDemoOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.sign_abs_demo", "符号与绝对值")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .output(OutputDef::new("y_sign", DataType::Number).with_description("符号与绝对值 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let result = x.signum() * x.abs().sqrt();

        Ok(json!({ "y_sign": result }))
    }
}

/// 立方根与对数
///
/// 方程 ID: `root_log_demo`
///
/// 公式: `y = \sqrt[3]{x} + \log_2(t + 1)`
pub struct RootLogDemoOperator;

impl Operator for RootLogDemoOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.root_log_demo", "立方根与对数")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .input(InputDef::required("t", DataType::Number).with_description("时间变量"))
            .output(OutputDef::new("y_log", DataType::Number).with_description("立方根与对数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let t = input
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("t".into()))?;

        let result = x.cbrt() + (t + 1_f64).log2();

        Ok(json!({ "y_log": result }))
    }
}

/// ReLU激活
///
/// 方程 ID: `relu_activation`
///
/// 公式: `y = \max(0, x)`
pub struct ReluActivationOperator;

impl Operator for ReluActivationOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.relu_activation", "ReLU激活")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .output(OutputDef::new("y_relu", DataType::Number).with_description("ReLU激活 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let result = if (x > 0_f64) { x } else { 0_f64 };

        Ok(json!({ "y_relu": result }))
    }
}

/// 取余运算
///
/// 方程 ID: `modulo_demo`
///
/// 公式: `y = \lfloor 10t \rfloor \mod 3`
pub struct ModuloDemoOperator;

impl Operator for ModuloDemoOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.modulo_demo", "取余运算")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("t", DataType::Number).with_description("时间变量"))
            .output(OutputDef::new("y_mod", DataType::Number).with_description("取余运算 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let t = input
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("t".into()))?;

        let result = (t * 10_f64).floor().rem_euclid(3_f64);

        Ok(json!({ "y_mod": result }))
    }
}

/// 反三角函数
///
/// 方程 ID: `inverse_trig`
///
/// 公式: `y = 2 \arcsin\left(\frac{x}{|x| + 1}\right)`
pub struct InverseTrigOperator;

impl Operator for InverseTrigOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.inverse_trig", "反三角函数")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("空间位置"))
            .output(OutputDef::new("y_asin", DataType::Number).with_description("反三角函数 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let result = 2_f64 * (x / (x.abs() + 1_f64)).asin();

        Ok(json!({ "y_asin": result }))
    }
}

/// 指数增长
///
/// 方程 ID: `exp_growth`
///
/// 公式: `y = e^{\lambda t}`
pub struct ExpGrowthOperator;

impl Operator for ExpGrowthOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("math_demo.exp_growth", "指数增长")
            .category("math_demo")
            .description("演示 equation-compiler 支持的各种数学运算符")
            .version("1.0.0")
            .input(InputDef::required("t", DataType::Number).with_description("时间变量"))
            .input(InputDef::optional("p3", DataType::Number).with_description("阻尼系数").with_default(json!(0.1)))
            .output(OutputDef::new("y_exp", DataType::Number).with_description("指数增长 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let t = input
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("t".into()))?;

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);

        let result = std::f64::consts::E.powf((p3 * t));

        Ok(json!({ "y_exp": result }))
    }
}
