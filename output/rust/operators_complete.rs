//! 完整运算符测试 (OPERATORS_COMPLETE)
//! 模型: OperatorsComplete
//! 自动生成的代码，请勿手动编辑

use crate::lowcode::core::metadata::{InputDef, OutputDef};
use crate::lowcode::core::types::DataType;
use crate::lowcode::core::{LowcodeError, Operator, OperatorMetadata};
use serde_json::{json, Value};

/// 加法测试
///
/// 方程 ID: `test_add`

pub struct TestAddOperator;

impl Operator for TestAddOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_add", "加法测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_add", DataType::Number).with_description("加法测试 输出"))
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

        let result = p1 + p2;

        Ok(json!({ "y_add": result }))
    }
}

/// 减法测试
///
/// 方程 ID: `test_sub`

pub struct TestSubOperator;

impl Operator for TestSubOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_sub", "减法测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_sub", DataType::Number).with_description("减法测试 输出"))
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

        let result = p1 - p2;

        Ok(json!({ "y_sub": result }))
    }
}

/// 乘法测试
///
/// 方程 ID: `test_mul`

pub struct TestMulOperator;

impl Operator for TestMulOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_mul", "乘法测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_mul", DataType::Number).with_description("乘法测试 输出"))
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

        let result = p1 * p2;

        Ok(json!({ "y_mul": result }))
    }
}

/// 除法测试
///
/// 方程 ID: `test_div`

pub struct TestDivOperator;

impl Operator for TestDivOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_div", "除法测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_div", DataType::Number).with_description("除法测试 输出"))
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

        let result = p1 / p2;

        Ok(json!({ "y_div": result }))
    }
}

/// 取负测试
///
/// 方程 ID: `test_neg`

pub struct TestNegOperator;

impl Operator for TestNegOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_neg", "取负测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_neg", DataType::Number).with_description("取负测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = -p1;

        Ok(json!({ "y_neg": result }))
    }
}

/// 幂运算测试
///
/// 方程 ID: `test_pow`

pub struct TestPowOperator;

impl Operator for TestPowOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_pow", "幂运算测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_pow", DataType::Number).with_description("幂运算测试 输出"))
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

        let result = p1.powf(p2);

        Ok(json!({ "y_pow": result }))
    }
}

/// 绝对值测试
///
/// 方程 ID: `test_abs`

pub struct TestAbsOperator;

impl Operator for TestAbsOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_abs", "绝对值测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_abs", DataType::Number).with_description("绝对值测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = (-p1).abs();

        Ok(json!({ "y_abs": result }))
    }
}

/// 取余测试
///
/// 方程 ID: `test_mod`

pub struct TestModOperator;

impl Operator for TestModOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_mod", "取余测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_mod", DataType::Number).with_description("取余测试 输出"))
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

        let result = p1.rem_euclid(p2);

        Ok(json!({ "y_mod": result }))
    }
}

/// 向上取整测试
///
/// 方程 ID: `test_ceil`

pub struct TestCeilOperator;

impl Operator for TestCeilOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_ceil", "向上取整测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_ceil", DataType::Number).with_description("向上取整测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.ceil();

        Ok(json!({ "y_ceil": result }))
    }
}

/// 向下取整测试
///
/// 方程 ID: `test_floor`

pub struct TestFloorOperator;

impl Operator for TestFloorOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_floor", "向下取整测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_floor", DataType::Number).with_description("向下取整测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.floor();

        Ok(json!({ "y_floor": result }))
    }
}

/// 四舍五入测试
///
/// 方程 ID: `test_round`

pub struct TestRoundOperator;

impl Operator for TestRoundOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_round", "四舍五入测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_round", DataType::Number).with_description("四舍五入测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.round();

        Ok(json!({ "y_round": result }))
    }
}

/// 截断取整测试
///
/// 方程 ID: `test_trunc`

pub struct TestTruncOperator;

impl Operator for TestTruncOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_trunc", "截断取整测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_trunc", DataType::Number).with_description("截断取整测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.trunc();

        Ok(json!({ "y_trunc": result }))
    }
}

/// 符号函数测试
///
/// 方程 ID: `test_sign`

pub struct TestSignOperator;

impl Operator for TestSignOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_sign", "符号函数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_sign", DataType::Number).with_description("符号函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.signum();

        Ok(json!({ "y_sign": result }))
    }
}

/// 小数部分测试
///
/// 方程 ID: `test_fract`

pub struct TestFractOperator;

impl Operator for TestFractOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_fract", "小数部分测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_fract", DataType::Number).with_description("小数部分测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.fract();

        Ok(json!({ "y_fract": result }))
    }
}

/// 倒数测试
///
/// 方程 ID: `test_recip`

pub struct TestRecipOperator;

impl Operator for TestRecipOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_recip", "倒数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_recip", DataType::Number).with_description("倒数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.recip();

        Ok(json!({ "y_recip": result }))
    }
}

/// 截断范围测试
///
/// 方程 ID: `test_clamp`

pub struct TestClampOperator;

impl Operator for TestClampOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_clamp", "截断范围测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_clamp", DataType::Number).with_description("截断范围测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.clamp(1_f64, 3_f64);

        Ok(json!({ "y_clamp": result }))
    }
}

/// 2的幂测试
///
/// 方程 ID: `test_exp2`

pub struct TestExp2Operator;

impl Operator for TestExp2Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_exp2", "2的幂测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_exp2", DataType::Number).with_description("2的幂测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p2 = input
            .get("p2")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.5);

        let result = p2.exp2();

        Ok(json!({ "y_exp2": result }))
    }
}

/// 高精度指数测试
///
/// 方程 ID: `test_expm1`

pub struct TestExpm1Operator;

impl Operator for TestExpm1Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_expm1", "高精度指数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_expm1", DataType::Number).with_description("高精度指数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.exp_m1();

        Ok(json!({ "y_expm1": result }))
    }
}

/// 高精度对数测试
///
/// 方程 ID: `test_ln1p`

pub struct TestLn1pOperator;

impl Operator for TestLn1pOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_ln1p", "高精度对数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_ln1p", DataType::Number).with_description("高精度对数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.ln_1p();

        Ok(json!({ "y_ln1p": result }))
    }
}

/// 任意底对数测试
///
/// 方程 ID: `test_logbase`

pub struct TestLogbaseOperator;

impl Operator for TestLogbaseOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_logbase", "任意底对数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_logbase", DataType::Number).with_description("任意底对数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 8_f64.log(2_f64);

        Ok(json!({ "y_logbase": result }))
    }
}

/// 斜边长测试
///
/// 方程 ID: `test_hypot`

pub struct TestHypotOperator;

impl Operator for TestHypotOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_hypot", "斜边长测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_hypot", DataType::Number).with_description("斜边长测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 3_f64.hypot(4_f64);

        Ok(json!({ "y_hypot": result }))
    }
}

/// 弧度转角度测试
///
/// 方程 ID: `test_degrees`

pub struct TestDegreesOperator;

impl Operator for TestDegreesOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_degrees", "弧度转角度测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_degrees", DataType::Number).with_description("弧度转角度测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = p5.to_degrees();

        Ok(json!({ "y_degrees": result }))
    }
}

/// 角度转弧度测试
///
/// 方程 ID: `test_radians`

pub struct TestRadiansOperator;

impl Operator for TestRadiansOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_radians", "角度转弧度测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p4", DataType::Number).with_description("角度参数").with_default(json!(45.0)))
            .output(OutputDef::new("y_radians", DataType::Number).with_description("角度转弧度测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p4 = input
            .get("p4")
            .and_then(|v| v.as_f64())
            .unwrap_or(45.0);

        let result = p4.to_radians();

        Ok(json!({ "y_radians": result }))
    }
}

/// 复制符号测试
///
/// 方程 ID: `test_copysign`

pub struct TestCopysignOperator;

impl Operator for TestCopysignOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_copysign", "复制符号测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_copysign", DataType::Number).with_description("复制符号测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.copysign((-1_f64));

        Ok(json!({ "y_copysign": result }))
    }
}

/// 融合乘加测试
///
/// 方程 ID: `test_mul_add`

pub struct TestMulAddOperator;

impl Operator for TestMulAddOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_mul_add", "融合乘加测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_mul_add", DataType::Number).with_description("融合乘加测试 输出"))
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

        let result = p1.mul_add(p2, p3);

        Ok(json!({ "y_mul_add": result }))
    }
}

/// 欧几里得除法测试
///
/// 方程 ID: `test_div_euclid`

pub struct TestDivEuclidOperator;

impl Operator for TestDivEuclidOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_div_euclid", "欧几里得除法测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_div_euclid", DataType::Number).with_description("欧几里得除法测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 7_f64.div_euclid(3_f64);

        Ok(json!({ "y_div_euclid": result }))
    }
}

/// 欧几里得取余测试
///
/// 方程 ID: `test_rem_euclid`

pub struct TestRemEuclidOperator;

impl Operator for TestRemEuclidOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_rem_euclid", "欧几里得取余测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_rem_euclid", DataType::Number).with_description("欧几里得取余测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 7_f64.rem_euclid(3_f64);

        Ok(json!({ "y_rem_euclid": result }))
    }
}

/// 银行家舍入测试
///
/// 方程 ID: `test_round_ties_even`

pub struct TestRoundTiesEvenOperator;

impl Operator for TestRoundTiesEvenOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_round_ties_even", "银行家舍入测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_round_even", DataType::Number).with_description("银行家舍入测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.round_ties_even();

        Ok(json!({ "y_round_even": result }))
    }
}

/// 中点测试
///
/// 方程 ID: `test_midpoint`

pub struct TestMidpointOperator;

impl Operator for TestMidpointOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_midpoint", "中点测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_midpoint", DataType::Number).with_description("中点测试 输出"))
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

        let result = p1.midpoint(p2);

        Ok(json!({ "y_midpoint": result }))
    }
}

/// 正割测试
///
/// 方程 ID: `test_sec`

pub struct TestSecOperator;

impl Operator for TestSecOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_sec", "正割测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_sec", DataType::Number).with_description("正割测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = 1.0 / p5.cos();

        Ok(json!({ "y_sec": result }))
    }
}

/// 余割测试
///
/// 方程 ID: `test_csc`

pub struct TestCscOperator;

impl Operator for TestCscOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_csc", "余割测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_csc", DataType::Number).with_description("余割测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = 1.0 / p5.sin();

        Ok(json!({ "y_csc": result }))
    }
}

/// 余切测试
///
/// 方程 ID: `test_cot`

pub struct TestCotOperator;

impl Operator for TestCotOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_cot", "余切测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_cot", DataType::Number).with_description("余切测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = 1.0 / p5.tan();

        Ok(json!({ "y_cot": result }))
    }
}

/// 指数函数测试
///
/// 方程 ID: `test_exp`

pub struct TestExpOperator;

impl Operator for TestExpOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_exp", "指数函数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_exp", DataType::Number).with_description("指数函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 1_f64.exp();

        Ok(json!({ "y_exp": result }))
    }
}

/// 自然对数测试
///
/// 方程 ID: `test_ln`

pub struct TestLnOperator;

impl Operator for TestLnOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_ln", "自然对数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_ln", DataType::Number).with_description("自然对数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.ln();

        Ok(json!({ "y_ln": result }))
    }
}

/// 常用对数测试
///
/// 方程 ID: `test_log10`

pub struct TestLog10Operator;

impl Operator for TestLog10Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_log10", "常用对数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_log10", DataType::Number).with_description("常用对数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 100_f64.log10();

        Ok(json!({ "y_log10": result }))
    }
}

/// 二进制对数测试
///
/// 方程 ID: `test_log2`

pub struct TestLog2Operator;

impl Operator for TestLog2Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_log2", "二进制对数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_log2", DataType::Number).with_description("二进制对数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 8_f64.log2();

        Ok(json!({ "y_log2": result }))
    }
}

/// 平方根测试
///
/// 方程 ID: `test_sqrt`

pub struct TestSqrtOperator;

impl Operator for TestSqrtOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_sqrt", "平方根测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_sqrt", DataType::Number).with_description("平方根测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.sqrt();

        Ok(json!({ "y_sqrt": result }))
    }
}

/// 立方根测试
///
/// 方程 ID: `test_cbrt`

pub struct TestCbrtOperator;

impl Operator for TestCbrtOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_cbrt", "立方根测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_cbrt", DataType::Number).with_description("立方根测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 27_f64.cbrt();

        Ok(json!({ "y_cbrt": result }))
    }
}

/// 正弦测试
///
/// 方程 ID: `test_sin`

pub struct TestSinOperator;

impl Operator for TestSinOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_sin", "正弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_sin", DataType::Number).with_description("正弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = p5.sin();

        Ok(json!({ "y_sin": result }))
    }
}

/// 余弦测试
///
/// 方程 ID: `test_cos`

pub struct TestCosOperator;

impl Operator for TestCosOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_cos", "余弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_cos", DataType::Number).with_description("余弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = p5.cos();

        Ok(json!({ "y_cos": result }))
    }
}

/// 正切测试
///
/// 方程 ID: `test_tan`

pub struct TestTanOperator;

impl Operator for TestTanOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_tan", "正切测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p5", DataType::Number).with_description("弧度参数").with_default(json!(0.7854)))
            .output(OutputDef::new("y_tan", DataType::Number).with_description("正切测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p5 = input
            .get("p5")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7854);

        let result = p5.tan();

        Ok(json!({ "y_tan": result }))
    }
}

/// 反正弦测试
///
/// 方程 ID: `test_asin`

pub struct TestAsinOperator;

impl Operator for TestAsinOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_asin", "反正弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_asin", DataType::Number).with_description("反正弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.asin();

        Ok(json!({ "y_asin": result }))
    }
}

/// 反余弦测试
///
/// 方程 ID: `test_acos`

pub struct TestAcosOperator;

impl Operator for TestAcosOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_acos", "反余弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_acos", DataType::Number).with_description("反余弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.acos();

        Ok(json!({ "y_acos": result }))
    }
}

/// 反正切测试
///
/// 方程 ID: `test_atan`

pub struct TestAtanOperator;

impl Operator for TestAtanOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_atan", "反正切测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_atan", DataType::Number).with_description("反正切测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.atan();

        Ok(json!({ "y_atan": result }))
    }
}

/// 二参数反正切测试
///
/// 方程 ID: `test_atan2`

pub struct TestAtan2Operator;

impl Operator for TestAtan2Operator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_atan2", "二参数反正切测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_atan2", DataType::Number).with_description("二参数反正切测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = 1_f64.atan2(1_f64);

        Ok(json!({ "y_atan2": result }))
    }
}

/// 双曲正弦测试
///
/// 方程 ID: `test_sinh`

pub struct TestSinhOperator;

impl Operator for TestSinhOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_sinh", "双曲正弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_sinh", DataType::Number).with_description("双曲正弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.sinh();

        Ok(json!({ "y_sinh": result }))
    }
}

/// 双曲余弦测试
///
/// 方程 ID: `test_cosh`

pub struct TestCoshOperator;

impl Operator for TestCoshOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_cosh", "双曲余弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_cosh", DataType::Number).with_description("双曲余弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.cosh();

        Ok(json!({ "y_cosh": result }))
    }
}

/// 双曲正切测试
///
/// 方程 ID: `test_tanh`

pub struct TestTanhOperator;

impl Operator for TestTanhOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_tanh", "双曲正切测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_tanh", DataType::Number).with_description("双曲正切测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.tanh();

        Ok(json!({ "y_tanh": result }))
    }
}

/// 反双曲正弦测试
///
/// 方程 ID: `test_asinh`

pub struct TestAsinhOperator;

impl Operator for TestAsinhOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_asinh", "反双曲正弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_asinh", DataType::Number).with_description("反双曲正弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.asinh();

        Ok(json!({ "y_asinh": result }))
    }
}

/// 反双曲余弦测试
///
/// 方程 ID: `test_acosh`

pub struct TestAcoshOperator;

impl Operator for TestAcoshOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_acosh", "反双曲余弦测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_acosh", DataType::Number).with_description("反双曲余弦测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = p1.acosh();

        Ok(json!({ "y_acosh": result }))
    }
}

/// 反双曲正切测试
///
/// 方程 ID: `test_atanh`

pub struct TestAtanhOperator;

impl Operator for TestAtanhOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_atanh", "反双曲正切测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_atanh", DataType::Number).with_description("反双曲正切测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p3 = input
            .get("p3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let result = p3.atanh();

        Ok(json!({ "y_atanh": result }))
    }
}

/// 最大值测试
///
/// 方程 ID: `test_max`

pub struct TestMaxOperator;

impl Operator for TestMaxOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_max", "最大值测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_max", DataType::Number).with_description("最大值测试 输出"))
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

        let result = [p1, p2, p3].into_iter().fold(f64::NEG_INFINITY, f64::max);

        Ok(json!({ "y_max": result }))
    }
}

/// 最小值测试
///
/// 方程 ID: `test_min`

pub struct TestMinOperator;

impl Operator for TestMinOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_min", "最小值测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .input(InputDef::optional("p3", DataType::Number).with_description("测试参数3").with_default(json!(0.5)))
            .output(OutputDef::new("y_min", DataType::Number).with_description("最小值测试 输出"))
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

        let result = [p1, p2, p3].into_iter().fold(f64::INFINITY, f64::min);

        Ok(json!({ "y_min": result }))
    }
}

/// 圆周率测试
///
/// 方程 ID: `test_pi`

pub struct TestPiOperator;

impl Operator for TestPiOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_pi", "圆周率测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_pi", DataType::Number).with_description("圆周率测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = std::f64::consts::PI * 2_f64;

        Ok(json!({ "y_pi": result }))
    }
}

/// 自然常数测试
///
/// 方程 ID: `test_e`

pub struct TestEOperator;

impl Operator for TestEOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_e", "自然常数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")

            .output(OutputDef::new("y_e", DataType::Number).with_description("自然常数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {


        let result = std::f64::consts::E.powf(2_f64);

        Ok(json!({ "y_e": result }))
    }
}

/// 比较运算测试
///
/// 方程 ID: `test_comparison`

pub struct TestComparisonOperator;

impl Operator for TestComparisonOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_comparison", "比较运算测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .input(InputDef::optional("p2", DataType::Number).with_description("测试参数2").with_default(json!(1.5)))
            .output(OutputDef::new("y_comp", DataType::Number).with_description("比较运算测试 输出"))
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

        let result = if (p1 > p2) { 1_f64 } else { 0_f64 };

        Ok(json!({ "y_comp": result }))
    }
}

/// 分段函数测试
///
/// 方程 ID: `test_piecewise`

pub struct TestPiecewiseOperator;

impl Operator for TestPiecewiseOperator {
    fn metadata(&self) -> OperatorMetadata {
        OperatorMetadata::builder("operators_complete.test_piecewise", "分段函数测试")
            .category("operators_complete")
            .description("测试所有支持的数学运算符")
            .version("1.0.0")
            .input(InputDef::optional("p1", DataType::Number).with_description("测试参数1").with_default(json!(2.5)))
            .output(OutputDef::new("y_piecewise", DataType::Number).with_description("分段函数测试 输出"))
            .build()
    }

    fn execute(&self, input: Value) -> Result<Value, LowcodeError> {
        let p1 = input
            .get("p1")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);

        let result = if (p1 < 1_f64) { 0_f64 } else if (p1 < 2_f64) { 1_f64 } else { 2_f64 };

        Ok(json!({ "y_piecewise": result }))
    }
}
