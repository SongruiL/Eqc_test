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

/// Forrester 系统动力学变量分类（8 类）
///
/// 描述变量在过程模型中的「动力学角色」，是画 Forrester（存量-流量）图、
/// 以及逐日仿真器识别状态量/速率/驱动的依据。与 [`VariableType`]（输入/中间/输出，
/// 描述数据流角色）正交：一个变量同时有一个 `VariableType` 和一个 `VarClass`。
///
/// 这是全仓库的单一真相源；`sexpr::workflow`（管线 B）从这里重导出同一枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarClass {
    /// S - 状态变量：时间积分量，有记忆（如累积干物质 TDM）
    State,
    /// V - 速率变量：直接驱动状态量变化的速率（如日干物质生产 DDM）
    Rate,
    /// A - 辅助变量：中间计算量，无跨步记忆（如光截获比例）
    Auxiliary,
    /// R - 驱动变量：外部环境时间序列输入（如气温、辐射）
    Driving,
    /// D - 模型参数：场景内不变的静态配置
    Parameter,
    /// C - 控制变量：人工管理决策输入
    Control,
    /// M - 半状态变量：有部分记忆但非严格积分（如上一步值的延迟寄存器）
    SemiState,
    /// B - 系统边界：物质进出系统的源/汇（如采收果实=汇）
    Boundary,
}

impl VarClass {
    /// 规范名（snake_case，与 serde 序列化一致；用于 JSON 契约）。
    pub fn as_str(&self) -> &'static str {
        match self {
            VarClass::State => "state",
            VarClass::Rate => "rate",
            VarClass::Auxiliary => "auxiliary",
            VarClass::Driving => "driving",
            VarClass::Parameter => "parameter",
            VarClass::Control => "control",
            VarClass::SemiState => "semi_state",
            VarClass::Boundary => "boundary",
        }
    }

    /// 单字母代号（Forrester 图标注用：S/V/A/R/D/C/M/B）
    pub fn code(&self) -> char {
        match self {
            VarClass::State => 'S',
            VarClass::Rate => 'V',
            VarClass::Auxiliary => 'A',
            VarClass::Driving => 'R',
            VarClass::Parameter => 'D',
            VarClass::Control => 'C',
            VarClass::SemiState => 'M',
            VarClass::Boundary => 'B',
        }
    }
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

    /// Forrester 动力学分类（可选；缺省时由 [`Variable::effective_class`] 按结构推断）
    #[serde(default)]
    pub class: Option<VarClass>,

    /// 初值（仅跨步变量需要）：积分状态量 / 延迟寄存器在 n=0（首步之前）的值
    #[serde(default)]
    pub init: Option<f64>,

    /// 积分状态量的速率来源变量名。
    ///
    /// 设置后，仿真器按显式 Euler 积分（日步长 dt=1）：`X[n] = X[n-1] + rate[n]`。
    /// 例如累积干物质 `TDM` 的 `rate: DDM`，累积温度 `CT` 的 `rate: T`。
    /// 状态量本身**不在 `equations:` 里写表达式**——它的值由积分得到。
    #[serde(default)]
    pub rate: Option<String>,

    /// 延迟寄存器的来源变量名（半状态量 SemiState）。
    ///
    /// 设置后，仿真器令本变量取来源变量的**上一步**值：`X[n] = prev_src[n-1]`，
    /// 首步用 `init`。用于需要上一步值的差分（如 `DRFG = 25*(RFG - RFG_prev)`，
    /// 其中 `RFG_prev` 声明 `prev: RFG`）。同样不在 `equations:` 里写表达式。
    #[serde(default)]
    pub prev: Option<String>,
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

    /// 是否为积分状态量（声明了 `rate`）：每步 `X[n] = X[n-1] + rate[n]`。
    pub fn is_integrator(&self) -> bool {
        self.rate.is_some()
    }

    /// 是否为延迟寄存器/半状态量（声明了 `prev`）：每步 `X[n] = prev_src[n-1]`。
    pub fn is_delay(&self) -> bool {
        self.prev.is_some()
    }

    /// 是否为跨步变量（积分状态量或延迟寄存器）——其值不由 `equations:` 计算，
    /// 而由仿真器跨时间步维护，需要 `init`。
    pub fn is_dynamic(&self) -> bool {
        self.is_integrator() || self.is_delay()
    }

    /// 有效 Forrester 分类：优先用显式声明的 `class`，否则按结构推断。
    ///
    /// 推断规则：积分量→State、延迟寄存器→SemiState、输入→Driving、其余→Auxiliary。
    pub fn effective_class(&self) -> VarClass {
        if let Some(c) = self.class {
            return c;
        }
        if self.is_integrator() {
            VarClass::State
        } else if self.is_delay() {
            VarClass::SemiState
        } else if self.var_type == VariableType::Input {
            VarClass::Driving
        } else {
            VarClass::Auxiliary
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Variable {
        Variable {
            var_type: VariableType::Intermediate,
            dtype: DataType::Float,
            unit: None,
            description: None,
            source: None,
            class: None,
            init: None,
            rate: None,
            prev: None,
        }
    }

    #[test]
    fn test_varclass_serde_snake_case() {
        // 与管线 B 既有 JSON 表示保持一致（SemiState -> "semi_state"）
        assert_eq!(
            serde_json::to_string(&VarClass::SemiState).unwrap(),
            "\"semi_state\""
        );
        let v: VarClass = serde_json::from_str("\"state\"").unwrap();
        assert_eq!(v, VarClass::State);
        assert_eq!(VarClass::Rate.code(), 'V');
    }

    #[test]
    fn test_dynamic_classification() {
        // 积分状态量
        let mut tdm = blank();
        tdm.init = Some(19.9);
        tdm.rate = Some("DDM".into());
        assert!(tdm.is_integrator() && tdm.is_dynamic() && !tdm.is_delay());
        assert_eq!(tdm.effective_class(), VarClass::State);

        // 延迟寄存器
        let mut rfg_prev = blank();
        rfg_prev.init = Some(0.000217);
        rfg_prev.prev = Some("RFG".into());
        assert!(rfg_prev.is_delay() && rfg_prev.is_dynamic() && !rfg_prev.is_integrator());
        assert_eq!(rfg_prev.effective_class(), VarClass::SemiState);

        // 驱动（输入）
        let mut driver = blank();
        driver.var_type = VariableType::Input;
        assert!(!driver.is_dynamic());
        assert_eq!(driver.effective_class(), VarClass::Driving);

        // 辅助（普通中间量）
        assert_eq!(blank().effective_class(), VarClass::Auxiliary);

        // 显式 class 优先于推断
        let mut forced = blank();
        forced.var_type = VariableType::Input;
        forced.class = Some(VarClass::Control);
        assert_eq!(forced.effective_class(), VarClass::Control);
    }
}
