//! 级联计算演示 (CASCADE)
//! 模型: CascadeDemo
//! 自动生成的代码，请勿手动编辑

use crate::lowcode::core::metadata::{InputDef, OutputDef};
use crate::lowcode::core::types::DataType;
use crate::lowcode::core::{LowcodeError, Operator, OperatorMetadata};
use serde_json::{json, Value};

/// 一阶变换
///
/// 方程 ID: `stage1`

pub struct Stage1Operator;

impl Operator for Stage1Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("cascade.stage1", "一阶变换")
            .category("cascade")
            .description("展示方程之间的 DAG 依赖关系")
            .version("1.0.0")
            .input(InputDef::required("x", DataType::Number).with_description("原始输入"))
            .input(InputDef::optional("p1", DataType::Number).with_description("一阶系数").with_default(json!(0.5)))
            .output(OutputDef::new("y1", DataType::Number).with_description("一阶变换 输出"))
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
            .unwrap_or(0.5);

        let result = p1 * x;

        Ok(json!({ "y1": result }))
    }
}

/// 二阶变换
///
/// 方程 ID: `stage2`

pub struct Stage2Operator;

impl Operator for Stage2Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("cascade.stage2", "二阶变换")
            .category("cascade")
            .description("展示方程之间的 DAG 依赖关系")
            .version("1.0.0")
            .input(InputDef::required("y1", DataType::Number))
            .input(InputDef::required("x", DataType::Number).with_description("原始输入"))
            .input(InputDef::optional("p2", DataType::Number).with_description("二阶系数").with_default(json!(0.3)))
            .output(OutputDef::new("y2", DataType::Number).with_description("二阶变换 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let y1 = input
            .get("y1")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("y1".into()))?;

        let x = input
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("x".into()))?;

        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3);

        let result = y1 + (p2 * x);

        Ok(json!({ "y2": result }))
    }
}

/// 最终输出
///
/// 方程 ID: `stage3`

pub struct Stage3Operator;

impl Operator for Stage3Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("cascade.stage3", "最终输出")
            .category("cascade")
            .description("展示方程之间的 DAG 依赖关系")
            .version("1.0.0")
            .input(InputDef::required("y2", DataType::Number))
            .output(OutputDef::new("y_final", DataType::Number).with_description("最终输出 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let y2 = input
            .get("y2")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| LowcodeError::MissingRequiredField("y2".into()))?;

        let result = y2.abs().sqrt();

        Ok(json!({ "y_final": result }))
    }
}
