//! 自动生成 - PhenoFlex冷量模型
//!
//! 基于Dynamic Model的冷量累积计算
//!
//! 由 equation-compiler 从 S表达式 自动生成，请勿手动修改。

use crate::lowcode::core::{Operator, OperatorMetadata, LowcodeError, DataType};
use crate::lowcode::core::metadata::{InputDef, OutputDef};
use serde_json::{Value, json};

/// 温度转开尔文
///
/// 将摄氏度转换为开尔文温度
pub struct TempKelvinOperator;

impl Operator for TempKelvinOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("phenoflex.temp_kelvin", "温度转开尔文")
            .category("物理转换")
            .description("将摄氏度转换为开尔文温度")
            .input(InputDef::required("T", DataType::Number)
                .with_description("温度(摄氏度)"))
            .input(InputDef::optional("offset", DataType::Number)
                .with_description("偏移量")
                .with_default(json!(273)))
            .output(OutputDef::new("TK", DataType::Number)
                .with_description("开尔文温度"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let t = input.get("T").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("T".into()))?;
        let offset = input.get("offset").and_then(|v| v.as_f64()).unwrap_or(273_f64);

        // (add T offset)
        let tk = (t + offset);

        Ok(json!({
            "TK": tk,
        }))
    }
}

/// Arrhenius速率常数
///
/// 计算Arrhenius方程的速率常数 k = A × exp(-E/(R×T))
pub struct ArrheniusOperator;

impl Operator for ArrheniusOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("phenoflex.arrhenius", "Arrhenius速率常数")
            .category("化学动力学")
            .description("计算Arrhenius方程的速率常数 k = A × exp(-E/(R×T))")
            .input(InputDef::required("A", DataType::Number)
                .with_description("频率因子"))
            .input(InputDef::required("E", DataType::Number)
                .with_description("活化能(cal/mol)"))
            .input(InputDef::optional("R", DataType::Number)
                .with_description("气体常数")
                .with_default(json!(1.987)))
            .input(InputDef::required("TK", DataType::Number)
                .with_description("温度(K)"))
            .output(OutputDef::new("k", DataType::Number)
                .with_description("速率常数"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let a = input.get("A").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("A".into()))?;
        let e = input.get("E").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("E".into()))?;
        let r = input.get("R").and_then(|v| v.as_f64()).unwrap_or(1.987_f64);
        let tk = input.get("TK").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("TK".into()))?;

        // (mul A (exp (neg (div E (mul R TK)))))
        let k = (a * (-(e / (r * tk))).exp());

        Ok(json!({
            "k": k,
        }))
    }
}

/// 冷量份额
///
/// 计算单时刻的冷量份额
pub struct ChillPortionOperator;

impl Operator for ChillPortionOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("phenoflex.chill_portion", "冷量份额")
            .category("物候模型")
            .description("计算单时刻的冷量份额")
            .input(InputDef::required("e0", DataType::Number)
                .with_description("基础活化"))
            .input(InputDef::required("e1", DataType::Number)
                .with_description("活化常数1"))
            .input(InputDef::required("Tc", DataType::Number)
                .with_description("临界温度"))
            .input(InputDef::required("T", DataType::Number)
                .with_description("当前温度"))
            .output(OutputDef::new("cp", DataType::Number)
                .with_description("冷量份额"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let e0 = input.get("e0").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("e0".into()))?;
        let e1 = input.get("e1").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("e1".into()))?;
        let tc = input.get("Tc").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("Tc".into()))?;
        let t = input.get("T").and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("T".into()))?;

        // (div 1 (add 1 (exp (mul e1 (sub Tc T)))))
        let cp = (1_f64 / (1_f64 + (e1 * (tc - t)).exp()));

        Ok(json!({
            "cp": cp,
        }))
    }
}

