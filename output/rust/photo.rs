//! 光合作用 (PHOTO)
//! 模型: QualiTree
//! 自动生成的代码，请勿手动编辑

use crate::lowcode::core::metadata::{InputDef, OutputDef};
use crate::lowcode::core::types::DataType;
use crate::lowcode::core::{LowcodeError, Operator, OperatorMetadata};
use serde_json::{json, Value};

/// 动态Pmax
///
/// 方程 ID: `PHOTO-01`
///
/// 公式: `Pmax_l = p1 + p2 × reserve_ratio`
pub struct Photo01Operator;

impl Operator for Photo01Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("photo.photo_01", "动态Pmax")
            .category("photo")
            .description("计算叶片光合速率")
            .version("1.0.0")
            .input(InputDef::required("reserve_ratio", DataType::Number).with_description("储备/生物量比"))
            .input(InputDef::optional("p1", DataType::Number).with_description("基础光饱和光合").with_default(json!(20.14)))
            .input(InputDef::optional("p2", DataType::Number).with_description("储备反馈系数").with_default(json!(-66.95)))
            .output(OutputDef::new("Pmax_l", DataType::Number).with_description("动态Pmax 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let reserve_ratio = input
            .get("reserve_ratio")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("reserve_ratio".into()))?;

        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(20.14);

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(-66.95);

        let result = p1 + (p2 * reserve_ratio);

        Ok(json!({ "Pmax_l": result }))
    }
}

/// Higgins光响应
///
/// 方程 ID: `PHOTO-02`
///
/// 公式: `A = (Pmax_l + p3) × (1 - exp(-p4 × PPFD / (Pmax_l + p3))) - p3`
pub struct Photo02Operator;

impl Operator for Photo02Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("photo.photo_02", "Higgins光响应")
            .category("photo")
            .description("计算叶片光合速率")
            .version("1.0.0")
            .input(InputDef::required("Pmax_l", DataType::Number).with_description("动态光饱和光合速率"))
            .input(InputDef::required("ppfd", DataType::Number).with_description("光合有效辐射"))
            .input(InputDef::optional("p3", DataType::Number).with_description("暗呼吸速率").with_default(json!(0.72)))
            .input(InputDef::optional("p4", DataType::Number).with_description("量子效率").with_default(json!(0.044)))
            .output(OutputDef::new("A_leaf", DataType::Number).with_description("Higgins光响应 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let Pmax_l = input
            .get("Pmax_l")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("Pmax_l".into()))?;

        let ppfd = input
            .get("ppfd")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("ppfd".into()))?;

        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.72);

        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.044);

        let result = ((Pmax_l + p3) * (1_f64 - (((-p4) * ppfd) / (Pmax_l + p3)).exp())) - p3;

        Ok(json!({ "A_leaf": result }))
    }
}
