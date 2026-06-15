//! 变量定义

use serde::{Deserialize, Serialize};

/// 变量类型
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    /// 输入变量（来自其他模块或外部）
    Input,
    /// 中间变量（本模块内部计算）
    #[default]
    Intermediate,
    /// 输出变量（供其他模块使用）
    Output,
}

/// 数据类型
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    /// 浮点数
    #[default]
    Float,
    /// 整数
    Int,
    /// 布尔值
    Bool,
    /// 字符串
    String,
    /// 数组（带元素类型）
    #[serde(rename = "array")]
    Array(Box<DataType>),
}

impl DataType {
    /// 转换为 Python 类型字符串
    pub fn to_python(&self) -> String {
        match self {
            Self::Float => "float".to_string(),
            Self::Int => "int".to_string(),
            Self::Bool => "bool".to_string(),
            Self::String => "str".to_string(),
            Self::Array(elem) => format!("np.ndarray[{}]", elem.to_python()),
        }
    }

    /// 转换为 Rust 类型字符串
    pub fn to_rust(&self) -> String {
        match self {
            Self::Float => "f64".to_string(),
            Self::Int => "i64".to_string(),
            Self::Bool => "bool".to_string(),
            Self::String => "String".to_string(),
            Self::Array(elem) => format!("Vec<{}>", elem.to_rust()),
        }
    }

    /// 检查是否是数值类型
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Float | Self::Int)
    }

    /// 检查是否是布尔类型
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool)
    }
}

/// 变量定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// 变量类型
    #[serde(rename = "type", default)]
    pub var_type: VariableType,

    /// 数据类型
    #[serde(default)]
    pub dtype: DataType,

    /// 单位
    #[serde(default)]
    pub unit: Option<String>,

    /// 描述
    #[serde(default)]
    pub description: Option<String>,

    /// 来源（仅 input 类型）：格式 "MODULE.variable"
    #[serde(default)]
    pub source: Option<String>,
}

impl Variable {
    /// 解析来源引用
    ///
    /// 返回 (模块ID, 变量名)
    pub fn parse_source(&self) -> Option<(&str, &str)> {
        self.source.as_ref().and_then(|s| {
            let parts: Vec<&str> = s.splitn(2, '.').collect();
            if parts.len() == 2 {
                Some((parts[0], parts[1]))
            } else {
                None
            }
        })
    }

    /// 获取显示标签
    pub fn display_label(&self) -> String {
        let type_str = match self.var_type {
            VariableType::Input => "输入",
            VariableType::Intermediate => "中间",
            VariableType::Output => "输出",
        };

        if let Some(ref desc) = self.description {
            format!("[{}] {}", type_str, desc)
        } else {
            format!("[{}]", type_str)
        }
    }
}
