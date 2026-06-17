//! 方程文件结构

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Equation, Parameter, Variable};

/// 方程文件（对应一个 .eq.yaml 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationFile {
    /// 元数据
    pub meta: Metadata,

    /// 参数定义（IndexMap：保留 YAML 声明顺序，保证输出可复现）
    #[serde(default)]
    pub parameters: IndexMap<String, Parameter>,

    /// 变量定义（IndexMap：保留 YAML 声明顺序，保证输出可复现）
    #[serde(default)]
    pub variables: IndexMap<String, Variable>,

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

    /// 将方程表达式中引用了「参数名」的 `Var` 节点重分类为 `Param`。
    ///
    /// EQC 解析单个名字时没有上下文（不知道 parameters 列表），所有 `{ref: x}` 先一律
    /// 解析为 `Var`。在整个文件加载、parameters 已知之后调用本方法做修正——这样参数就能
    /// 用任意有意义的名字（如 `Tbase`、`AMAX`），而不必非叫 `p1`、`p2`。
    pub fn reclassify_parameters(&mut self) {
        use crate::ast::Expr;
        let pnames: Vec<String> = self.parameters.keys().cloned().collect();
        for eq in &mut self.equations {
            for pname in &pnames {
                eq.expression = eq
                    .expression
                    .substitute(pname, &Expr::Param(pname.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::schema::DataType;

    #[test]
    fn test_reclassify_parameters() {
        let mut parameters = IndexMap::new();
        parameters.insert(
            "Tbase".to_string(),
            Parameter {
                name_cn: "基点温度".into(),
                name_en: None,
                dtype: DataType::Float,
                default: 3.0,
                unit: Some("degC".into()),
                bounds: None,
                optimizable: true,
                description: None,
            },
        );

        // 表达式引用 Tbase（非 p+数字，故先被解析为 Var）
        let expr = Expr::sub(Expr::var("Tavg"), Expr::var("Tbase"));
        assert!(
            expr.get_parameter_refs().is_empty(),
            "重分类前 Tbase 应是变量"
        );

        let mut file = EquationFile {
            meta: Metadata {
                id: "T".into(),
                model: "T".into(),
                name_cn: "".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
            },
            parameters,
            variables: Default::default(),
            equations: vec![Equation {
                id: "E".into(),
                name: "".into(),
                output: "y".into(),
                expression: expr,
                formula_display: None,
                reference: None,
            }],
        };

        file.reclassify_parameters();
        let pref = file.equations[0].expression.get_parameter_refs();
        let vref = file.equations[0].expression.get_variable_refs();
        assert!(pref.contains(&"Tbase".to_string()), "Tbase 应被重分类为参数: {pref:?}");
        assert!(vref.contains(&"Tavg".to_string()), "Tavg 仍应是变量");
        assert!(!vref.contains(&"Tbase".to_string()), "Tbase 不应再是变量");
    }
}
