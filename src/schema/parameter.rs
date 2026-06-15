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

    /// 默认值
    pub default: f64,

    /// 单位
    #[serde(default)]
    pub unit: Option<String>,

    /// 优化边界 [min, max]
    #[serde(default)]
    pub bounds: Option<(f64, f64)>,

    /// 是否可被 GP 优化
    #[serde(default = "default_true")]
    pub optimizable: bool,

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
