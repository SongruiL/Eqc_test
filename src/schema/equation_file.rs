//! 方程文件结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Equation, Parameter, Variable};

/// 方程文件（对应一个 .eq.yaml 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationFile {
    /// 元数据
    pub meta: Metadata,

    /// 参数定义
    #[serde(default)]
    pub parameters: HashMap<String, Parameter>,

    /// 变量定义
    #[serde(default)]
    pub variables: HashMap<String, Variable>,

    /// 方程定义
    #[serde(default)]
    pub equations: Vec<Equation>,
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// 模块 ID
    pub id: String,

    /// 所属模型名称
    pub model: String,

    /// 中文名称
    pub name_cn: String,

    /// 英文名称
    #[serde(default)]
    pub name_en: Option<String>,

    /// 版本
    #[serde(default = "default_version")]
    pub version: String,

    /// 描述
    #[serde(default)]
    pub description: Option<String>,

    /// 参考文献
    #[serde(default)]
    pub reference: Option<String>,

    /// 源代码文件（参考用）
    #[serde(default)]
    pub source_files: Vec<String>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl EquationFile {
    /// 获取所有参数名称
    pub fn parameter_names(&self) -> Vec<&str> {
        self.parameters.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有变量名称
    pub fn variable_names(&self) -> Vec<&str> {
        self.variables.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有方程 ID
    pub fn equation_ids(&self) -> Vec<&str> {
        self.equations.iter().map(|e| e.id.as_str()).collect()
    }

    /// 获取输出变量列表
    pub fn output_variables(&self) -> Vec<(&str, &Variable)> {
        self.variables
            .iter()
            .filter(|(_, v)| v.var_type == super::VariableType::Output)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// 获取输入变量列表
    pub fn input_variables(&self) -> Vec<(&str, &Variable)> {
        self.variables
            .iter()
            .filter(|(_, v)| v.var_type == super::VariableType::Input)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}
