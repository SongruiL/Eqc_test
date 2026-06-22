//! 参数定义

use serde::{Deserialize, Serialize};

use super::DataType;

/// 参数定义（常量，可被 GP 优化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// 中文名称
    pub name_cn: String,

    /// 英文名称
    #[serde(default)]
    pub name_en: Option<String>,

    /// 数据类型
    #[serde(rename = "type", default)]
    pub dtype: DataType,

    /// 默认值（标量参数）。向量参数（用 `values`）可省略。
    #[serde(default)]
    pub default: f64,

    /// 向量参数的逐元素值（如各 cohort 的开花日 `[40,80,120]`）。
    /// 设置后该参数为**向量**（[`crate::eval::Value::Vector`]）；否则用 `default` 作标量。
    #[serde(default)]
    pub values: Option<Vec<f64>>,

    /// 单位
    #[serde(default)]
    pub unit: Option<String>,

    /// 优化边界 [min, max]
    #[serde(default)]
    pub bounds: Option<(f64, f64)>,

    /// 是否可被 GP 优化
    #[serde(default = "default_true")]
    pub optimizable: bool,

    /// 是否为**管理输入**（园区按处理区设置的、可逐区不同的管理量，如灌溉/施氮/EC）。
    ///
    /// 多处理区编排用：园区视图「本区管理」编辑器据此列出可逐区设置的参数；存入 `<zone>.json`。
    /// （`control` 类**变量**——如 CO₂——已天然是管理决策输入，由 class 识别，无需本标志。）
    #[serde(default)]
    pub management: bool,

    /// 描述
    #[serde(default)]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Parameter {
    /// 检查值是否在边界内
    pub fn is_within_bounds(&self, value: f64) -> bool {
        match self.bounds {
            Some((min, max)) => value >= min && value <= max,
            None => true,
        }
    }

    /// 获取显示标签
    pub fn display_label(&self) -> String {
        if let Some(ref unit) = self.unit {
            format!("{} = {} [{}]", self.name_cn, self.default, unit)
        } else {
            format!("{} = {}", self.name_cn, self.default)
        }
    }
}
