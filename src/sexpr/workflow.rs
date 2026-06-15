//! Workflow定义和注解解析
//!
//! 本模块提供从带注解的S表达式文件解析Workflow定义的功能。
//!
//! # 文件格式
//!
//! 采用注释风格的带注解S表达式文件格式（`.sexpr`）：
//!
//! ```text
//! ;; @module: phenoflex.chill
//! ;; @name: PhenoFlex冷量模型
//! ;; @description: 基于Dynamic Model的冷量累积计算
//!
//! ;; @operator: phenoflex.temp_kelvin
//! ;; @name: 温度转开尔文
//! ;; @category: 物理转换
//! ;; @description: 将摄氏度转换为开尔文温度
//! ;; @input: T, Number, required, 温度(摄氏度)
//! ;; @input: offset, Number, optional, 偏移量, 273
//! ;; @output: TK, Number, 开尔文温度
//! (add T offset)
//! ```

use super::error::{SExprError, SExprResult, Span};
use super::SExpr;
use crate::ast::Expr;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::collections::HashSet;

/// 输入参数定义
#[derive(Debug, Clone)]
pub struct InputDef {
    /// 参数名称
    pub name: String,
    /// 数据类型（Number, String, Boolean, Array）
    pub data_type: String,
    /// 是否必填
    pub required: bool,
    /// 参数描述
    pub description: String,
    /// 默认值（可选，原生 JSON 值）
    pub default_value: Option<serde_json::Value>,
    /// LaTeX 格式的参数名（如 T_K, \xi）
    pub latex_name: Option<String>,
    /// 论文引用说明（参数在论文中的含义）
    pub paper_ref: Option<String>,
}

impl InputDef {
    /// 创建必填输入参数
    pub fn required(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            required: true,
            description: String::new(),
            default_value: None,
            latex_name: None,
            paper_ref: None,
        }
    }

    /// 创建可选输入参数
    pub fn optional(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            required: false,
            description: String::new(),
            default_value: None,
            latex_name: None,
            paper_ref: None,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置默认值
    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self
    }

    /// 设置 LaTeX 格式名称
    pub fn with_latex(mut self, latex: impl Into<String>) -> Self {
        self.latex_name = Some(latex.into());
        self
    }

    /// 设置论文引用说明
    pub fn with_paper_ref(mut self, paper_ref: impl Into<String>) -> Self {
        self.paper_ref = Some(paper_ref.into());
        self
    }
}

/// 输出参数定义
#[derive(Debug, Clone)]
pub struct OutputDef {
    /// 输出名称
    pub name: String,
    /// 数据类型
    pub data_type: String,
    /// 输出描述
    pub description: String,
    /// LaTeX 格式的参数名
    pub latex_name: Option<String>,
}

impl OutputDef {
    /// 创建输出定义
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            description: String::new(),
            latex_name: None,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置 LaTeX 格式名称
    pub fn with_latex(mut self, latex: impl Into<String>) -> Self {
        self.latex_name = Some(latex.into());
        self
    }
}

/// 算子类型（基于数学层次自动检测）
///
/// 分类依据：
/// - Operator: 基本运算（+,-,*,/）或初等函数（sin,cos,exp,sigmoid）
/// - Formula: 复合表达式，多个运算/函数组合
/// - System: 方程组/分段函数/多分支结构
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OperatorType {
    /// 基本运算 + 初等函数
    /// 单个运算符或单个函数调用，操作数为原子值
    Operator,
    /// 复合公式
    /// 多个运算/函数组合，单一输出
    #[default]
    Formula,
    /// 方程组/系统
    /// 分段函数、联立方程、递归定义
    System,
}

impl OperatorType {
    /// 转换为字符串（用于 JSON/SQL 输出）
    /// 注意：System 序列化为 "equation_network" 以保持前端兼容
    pub fn as_str(&self) -> &'static str {
        match self {
            OperatorType::Operator => "operator",
            OperatorType::Formula => "formula",
            OperatorType::System => "equation_network",
        }
    }
}

/// 基于 S 表达式结构自动检测算子类型
///
/// 语义分类规则：
/// - System: 分段函数(piecewise)、多分支结构
/// - Formula: 有业务含义的表达式（默认，因为所有注解定义的算子都有特定含义）
/// - Operator: 仅用于通用工具库中的基础算子（当前场景不适用）
///
/// 注意：通过 @operator 注解定义的算子都有 @name/@description，
/// 说明它们有特定的物理/数学含义，应归类为 Formula 而非 Operator。
pub fn auto_detect_operator_type(sexpr: &SExpr) -> OperatorType {
    match sexpr {
        SExpr::List(items) if !items.is_empty() => {
            if let SExpr::Symbol(op) = &items[0] {
                // 分段函数 → System（多分支结构）
                if op == "piecewise" {
                    return OperatorType::System;
                }
            }
            // 所有其他表达式 → Formula（有业务含义的公式）
            OperatorType::Formula
        }
        // 空列表或单值 → Formula
        _ => OperatorType::Formula,
    }
}

/// 算子定义（从注解解析）
#[derive(Debug, Clone)]
pub struct OperatorDef {
    /// 算子唯一标识符（如 phenoflex.temp_kelvin）
    pub id: String,
    /// 算子显示名称
    pub name: String,
    /// 算子类型（运算符/公式/方程网络架构）
    pub operator_type: OperatorType,
    /// 算子分类
    pub category: String,
    /// 算子描述
    pub description: String,
    /// 公式的 LaTeX 表示
    pub latex_formula: Option<String>,
    /// 输入参数列表
    pub inputs: Vec<InputDef>,
    /// 输出参数列表
    pub outputs: Vec<OutputDef>,
    /// S表达式AST（原始形式）
    pub sexpr: SExpr,
    /// 表达式AST（转换后）
    pub expr: Option<Expr>,
}

impl OperatorDef {
    /// 创建新的算子定义
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            operator_type: OperatorType::default(),
            category: String::new(),
            description: String::new(),
            latex_formula: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            sexpr: SExpr::Number(0.0), // placeholder
            expr: None,
        }
    }

    /// 设置名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 设置算子类型
    pub fn with_type(mut self, op_type: OperatorType) -> Self {
        self.operator_type = op_type;
        self
    }

    /// 设置分类
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置 LaTeX 公式
    pub fn with_latex_formula(mut self, latex: impl Into<String>) -> Self {
        self.latex_formula = Some(latex.into());
        self
    }

    /// 添加输入参数
    pub fn add_input(&mut self, input: InputDef) {
        self.inputs.push(input);
    }

    /// 添加输出参数
    pub fn add_output(&mut self, output: OutputDef) {
        self.outputs.push(output);
    }

    /// 设置S表达式
    pub fn with_sexpr(mut self, sexpr: SExpr) -> Self {
        self.sexpr = sexpr;
        self
    }

    /// 设置表达式AST
    pub fn with_expr(mut self, expr: Expr) -> Self {
        self.expr = Some(expr);
        self
    }
}

/// 模块定义
#[derive(Debug, Clone)]
pub struct ModuleDef {
    /// 模块唯一标识符（如 phenoflex.chill）
    pub id: String,
    /// 模块显示名称
    pub name: String,
    /// 模块描述
    pub description: String,
    /// 算子列表
    pub operators: Vec<OperatorDef>,
    /// 工作流控制流配置（用于迭代执行）
    pub control_flow: WorkflowControlFlowConfig,
    /// 工作流级别输入（显式声明）
    pub workflow_inputs: Vec<WorkflowInputDef>,
    /// 工作流级别输出（显式声明）
    pub workflow_outputs: Vec<WorkflowOutputDef>,
    /// 显式边定义
    pub edges: Vec<EdgeDef>,
    /// 广播定义
    pub broadcasts: Vec<BroadcastDef>,
    /// 概率分布定义
    pub distributions: Vec<DistributionDef>,
    /// Monte Carlo 配置
    pub mc_config: Option<MonteCarloConfigDef>,
    /// 系统边界定义（Forrester 源/汇）
    pub boundaries: Vec<BoundaryDef>,
    /// 模块间耦合接口（Connector）
    pub connectors: Vec<ConnectorDef>,
}

impl ModuleDef {
    /// 创建新的模块定义
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            description: String::new(),
            operators: Vec::new(),
            control_flow: WorkflowControlFlowConfig::default(),
            workflow_inputs: Vec::new(),
            workflow_outputs: Vec::new(),
            edges: Vec::new(),
            broadcasts: Vec::new(),
            distributions: Vec::new(),
            mc_config: None,
            boundaries: Vec::new(),
            connectors: Vec::new(),
        }
    }

    /// 设置名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 添加算子
    pub fn add_operator(&mut self, operator: OperatorDef) {
        self.operators.push(operator);
    }

    /// 设置控制流配置
    pub fn with_control_flow(mut self, config: WorkflowControlFlowConfig) -> Self {
        self.control_flow = config;
        self
    }

    /// 添加工作流输入
    pub fn add_workflow_input(&mut self, input: WorkflowInputDef) {
        self.workflow_inputs.push(input);
    }

    /// 添加工作流输出
    pub fn add_workflow_output(&mut self, output: WorkflowOutputDef) {
        self.workflow_outputs.push(output);
    }

    /// 添加系统边界
    pub fn add_boundary(&mut self, boundary: BoundaryDef) {
        self.boundaries.push(boundary);
    }
}

/// Forrester 系统动力学变量分类（8 类）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarClass {
    /// S - 状态变量：时间积分量，有记忆
    State,
    /// V - 速率变量：直接控制状态变量变化的速率
    Rate,
    /// A - 辅助变量：中间计算量，无记忆
    Auxiliary,
    /// R - 驱动变量：外部环境时间序列输入
    Driving,
    /// D - 模型参数：场景内不变的静态配置
    Parameter,
    /// C - 控制变量：人工管理决策输入
    Control,
    /// M - 半状态变量：有部分记忆但非严格积分
    SemiState,
    /// B - 系统边界：物质进出系统的源/汇
    Boundary,
}

/// Forrester 系统动力学流（边）分类（3 类）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlowType {
    /// 物质流：守恒量的传输（碳、水、能量）
    Material,
    /// 信息流：信号/数据传递，不守恒（默认）
    #[default]
    Info,
    /// 控制流：管理决策的影响路径
    Control,
}

/// 系统边界类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryType {
    /// 物质进入系统
    Source,
    /// 物质离开系统
    Sink,
}

/// 系统边界定义（源/汇）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryDef {
    /// 边界名称
    pub name: String,
    /// 边界类型（源/汇）
    pub boundary_type: BoundaryType,
    /// 描述
    pub description: String,
}

/// Connector 方向（模块间耦合接口）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorDirection {
    /// 输入：从其他模块接收
    In,
    /// 输出：向其他模块暴露
    Out,
}

/// Connector 定义（模块间耦合接口）
///
/// 对应 FMI 标准中的 connector，用于多模块耦合的标准化端口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorDef {
    /// 接口名称
    pub name: String,
    /// 数据类型
    pub data_type: String,
    /// 描述
    pub description: String,
    /// 方向（输入/输出）
    pub direction: ConnectorDirection,
    /// 远程来源（仅 In 方向有效），如 "phenoflex.bloom_date"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_source: Option<String>,
}

/// 注解类型
#[derive(Debug, Clone, PartialEq)]
enum AnnotationType {
    Module,
    Operator,
    // 注意: @type 已废弃，算子类型由系统自动检测
    Name,
    Category,
    Description,
    Latex,          // 公式的 LaTeX 表示
    Input,
    InputLatex,     // 输入参数的 LaTeX 名称
    InputPaperRef,  // 输入参数的论文引用说明
    Output,
    OutputLatex,    // 输出参数的 LaTeX 名称
    // 工作流控制流配置
    ExecutionMode,  // 执行模式: single | iterative | monte_carlo
    TimeSeries,     // 时间序列输入
    Accumulator,    // 累加器定义
    StateVar,       // 状态变量定义
    Termination,    // 终止条件
    // 工作流级别输入输出（区别于算子输入输出）
    WorkflowInput,  // 工作流外部输入
    WorkflowOutput, // 工作流最终输出
    // 显式数据流连接
    Edge,           // 显式边: source_node.port -> target_node.port [lag N]
    Broadcast,      // 广播: input_name -> node1.port, node2.port
    // Forrester 系统边界
    Boundary,       // 系统边界: name, source|sink, description
    // 模块间耦合接口
    ConnectorIn,    // 输入接口: name, type, from module.port
    ConnectorOut,   // 输出接口: name, type, description
    // 概率/随机扩展
    Distribution,   // 参数分布: param_name, dist_type, key=value, ...
    McSamples,      // Monte Carlo 采样次数
    McSeed,         // Monte Carlo 随机种子
    McOutput,       // Monte Carlo 输出格式
    McParallel,     // Monte Carlo 并行执行
}

/// 累加器定义（用于迭代执行）
#[derive(Debug, Clone)]
pub struct AccumulatorDef {
    /// 累加器名称
    pub name: String,
    /// 数据来源节点
    pub source_node: String,
    /// 数据来源端口
    pub source_port: String,
    /// 累积操作类型
    pub operation: String,
    /// 初始值
    pub initial_value: f64,
}

/// 状态变量定义（用于跨迭代传递）
#[derive(Debug, Clone)]
pub struct StateVarDef {
    /// 变量名
    pub name: String,
    /// 数据来源节点
    pub source_node: String,
    /// 数据来源端口
    pub source_port: String,
    /// 初始值
    pub initial_value: f64,
    /// 滞后步数
    pub lag: u32,
    /// 动态初始化（如 "dynamic(equilibrium.x_eq)"）
    pub dynamic_init: Option<String>,
}

/// 终止条件定义
#[derive(Debug, Clone)]
pub struct TerminationDef {
    /// 条件类型
    pub condition_type: String,
    /// 累加器名称（用于 accumulator_threshold）
    pub accumulator_name: Option<String>,
    /// 阈值
    pub threshold: Option<f64>,
    /// 比较操作符
    pub op: Option<String>,
}

/// 工作流级别输入定义（区别于算子输入）
#[derive(Debug, Clone)]
pub struct WorkflowInputDef {
    /// 输入名称
    pub name: String,
    /// 数据类型（Number, String, Boolean, Array<Number> 等）
    pub data_type: String,
    /// 是否必填
    pub required: bool,
    /// 描述
    pub description: String,
    /// 默认值（原生 JSON 值）
    pub default_value: Option<serde_json::Value>,
    /// LaTeX 格式名称
    pub latex_name: Option<String>,
    /// 论文引用说明
    pub paper_ref: Option<String>,
    /// 绑定到的节点和端口（自动推断或显式指定）
    pub bind_to_node: Option<String>,
    pub bind_to_port: Option<String>,
    /// Forrester 变量分类（可选，可自动推断）
    pub var_class: Option<VarClass>,
}

/// 工作流级别输出定义（区别于算子输出）
#[derive(Debug, Clone)]
pub struct WorkflowOutputDef {
    /// 输出名称
    pub name: String,
    /// 数据类型
    pub data_type: String,
    /// 描述
    pub description: String,
    /// LaTeX 格式名称
    pub latex_name: Option<String>,
    /// 数据来源：accumulator | node
    pub source_type: String,
    /// 来源名称（累加器名称或节点ID）
    pub source_name: Option<String>,
    /// 来源端口（仅当 source_type 为 node 时使用）
    pub source_port: Option<String>,
}

/// 工作流控制流配置
#[derive(Debug, Clone, Default)]
pub struct WorkflowControlFlowConfig {
    /// 执行模式
    pub execution_mode: String,
    /// 时间序列输入
    pub time_series_inputs: Vec<String>,
    /// 累加器定义
    pub accumulators: Vec<AccumulatorDef>,
    /// 状态变量定义
    pub state_vars: Vec<StateVarDef>,
    /// 终止条件
    pub termination: Option<TerminationDef>,
}

/// 显式边定义（替代隐式名称匹配）
#[derive(Debug, Clone)]
pub struct EdgeDef {
    /// 源节点（或 "@state", "@acc" 表示特殊来源）
    pub source_node: String,
    /// 源端口
    pub source_port: String,
    /// 目标节点
    pub target_node: String,
    /// 目标端口
    pub target_port: String,
    /// 滞后步数（仅用于状态边）
    pub lag: Option<u32>,
    /// 初始值（支持 "dynamic(node.port)" 语法）
    pub init_value: Option<String>,
    /// Forrester 流类型（默认 Info 信息流）
    pub flow_type: FlowType,
}

/// 广播定义（一个工作流输入分发到多个节点）
#[derive(Debug, Clone)]
pub struct BroadcastDef {
    /// 工作流输入名
    pub input_name: String,
    /// 目标列表 (node, port)
    pub targets: Vec<(String, String)>,
}

/// 概率分布定义（用于 Monte Carlo）
#[derive(Debug, Clone)]
pub struct DistributionDef {
    /// 绑定的参数名
    pub param_name: String,
    /// 分布类型
    pub dist_type: String,
    /// 分布参数
    pub params: HashMap<String, f64>,
}

/// Monte Carlo 配置定义
#[derive(Debug, Clone, Default)]
pub struct MonteCarloConfigDef {
    /// 采样次数
    pub samples: usize,
    /// 随机种子
    pub seed: Option<u64>,
    /// 输出百分位数
    pub output_percentiles: Vec<f64>,
    /// 是否并行执行
    pub parallel: bool,
}

/// 解析后的注解
#[derive(Debug, Clone)]
struct Annotation {
    anno_type: AnnotationType,
    value: String,
}

/// 带注解S表达式解析器
pub struct AnnotatedSExprParser<'a> {
    #[allow(dead_code)]
    input: &'a str,
    lines: Vec<&'a str>,
    current_line: usize,
}

impl<'a> AnnotatedSExprParser<'a> {
    /// 创建新的解析器
    pub fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            input,
            lines,
            current_line: 0,
        }
    }

    /// 解析带注解的S表达式文件
    pub fn parse(&mut self) -> SExprResult<ModuleDef> {
        let mut module: Option<ModuleDef> = None;
        let mut current_operator: Option<OperatorDef> = None;
        let mut pending_annotations: Vec<Annotation> = Vec::new();

        // 收集所有注解和表达式
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();

            // 跳过空行
            if line.is_empty() {
                self.current_line += 1;
                continue;
            }

            // 处理注解注释 (;; @key: value)
            if line.starts_with(";;") {
                if let Some(annotation) = self.parse_annotation(line) {
                    match annotation.anno_type {
                        AnnotationType::Module => {
                            // 保存之前的算子
                            if let (Some(ref mut m), Some(op)) = (&mut module, current_operator.take()) {
                                m.add_operator(op);
                            }
                            // 创建新模块
                            module = Some(ModuleDef::new(&annotation.value));
                        }
                        AnnotationType::Operator => {
                            // 保存之前的算子
                            if let (Some(ref mut m), Some(op)) = (&mut module, current_operator.take()) {
                                m.add_operator(op);
                            }
                            // 应用待处理的注解到模块
                            if let Some(ref mut m) = module {
                                for anno in pending_annotations.drain(..) {
                                    match anno.anno_type {
                                        AnnotationType::Name => m.name = anno.value,
                                        AnnotationType::Description => m.description = anno.value,
                                        AnnotationType::ExecutionMode => {
                                            m.control_flow.execution_mode = anno.value;
                                        }
                                        AnnotationType::TimeSeries => {
                                            // 支持逗号分隔的多个时间序列输入，如 "T, Rad"
                                            for name in anno.value.split(',') {
                                                let trimmed = name.trim();
                                                if !trimmed.is_empty() {
                                                    m.control_flow.time_series_inputs.push(trimmed.to_string());
                                                }
                                            }
                                        }
                                        AnnotationType::Accumulator => {
                                            if let Some(acc) = self.parse_accumulator_annotation(&anno.value) {
                                                m.control_flow.accumulators.push(acc);
                                            }
                                        }
                                        AnnotationType::StateVar => {
                                            if let Some(sv) = self.parse_state_var_annotation(&anno.value) {
                                                m.control_flow.state_vars.push(sv);
                                            }
                                        }
                                        AnnotationType::Termination => {
                                            if let Some(term) = self.parse_termination_annotation(&anno.value) {
                                                m.control_flow.termination = Some(term);
                                            }
                                        }
                                        AnnotationType::WorkflowInput => {
                                            if let Some(wi) = self.parse_workflow_input_annotation(&anno.value) {
                                                m.workflow_inputs.push(wi);
                                            }
                                        }
                                        AnnotationType::WorkflowOutput => {
                                            if let Some(wo) = self.parse_workflow_output_annotation(&anno.value) {
                                                m.workflow_outputs.push(wo);
                                            }
                                        }
                                        AnnotationType::Edge => {
                                            if let Some(edge) = self.parse_edge_annotation(&anno.value) {
                                                m.edges.push(edge);
                                            }
                                        }
                                        AnnotationType::Broadcast => {
                                            if let Some(bc) = self.parse_broadcast_annotation(&anno.value) {
                                                m.broadcasts.push(bc);
                                            }
                                        }
                                        AnnotationType::Boundary => {
                                            if let Some(b) = self.parse_boundary_annotation(&anno.value) {
                                                m.boundaries.push(b);
                                            }
                                        }
                                        AnnotationType::ConnectorIn => {
                                            if let Some(c) = self.parse_connector_in_annotation(&anno.value) {
                                                m.connectors.push(c);
                                            }
                                        }
                                        AnnotationType::ConnectorOut => {
                                            if let Some(c) = self.parse_connector_out_annotation(&anno.value) {
                                                m.connectors.push(c);
                                            }
                                        }
                                        AnnotationType::Distribution => {
                                            if let Some(dist) = self.parse_distribution_annotation(&anno.value) {
                                                m.distributions.push(dist);
                                            }
                                        }
                                        AnnotationType::McSamples => {
                                            let samples = anno.value.trim().parse().unwrap_or(1000);
                                            let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                                            mc.samples = samples;
                                        }
                                        AnnotationType::McSeed => {
                                            let seed = anno.value.trim().parse().ok();
                                            let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                                            mc.seed = seed;
                                        }
                                        AnnotationType::McOutput => {
                                            let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                                            // 解析 "percentiles 5 25 50 75 95"
                                            let parts: Vec<&str> = anno.value.split_whitespace().collect();
                                            if !parts.is_empty() && parts[0] == "percentiles" {
                                                mc.output_percentiles = parts[1..].iter()
                                                    .filter_map(|s| s.parse().ok())
                                                    .collect();
                                            }
                                        }
                                        AnnotationType::McParallel => {
                                            let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                                            mc.parallel = anno.value.trim() == "true";
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            // 创建新算子
                            current_operator = Some(OperatorDef::new(&annotation.value));
                            pending_annotations.clear();
                        }
                        _ => {
                            pending_annotations.push(annotation);
                        }
                    }
                }
                self.current_line += 1;
                continue;
            }

            // 跳过普通注释（单个分号）
            if line.starts_with(';') {
                self.current_line += 1;
                continue;
            }

            // 解析S表达式
            if line.starts_with('(') || line.chars().next().is_some_and(|c| c.is_alphanumeric() || c == '-') {
                // 收集多行S表达式
                let sexpr_text = self.collect_sexpr()?;
                
                if let Some(ref mut operator) = current_operator {
                    // 应用待处理的注解到算子
                    for anno in pending_annotations.drain(..) {
                        match anno.anno_type {
                            AnnotationType::Name => operator.name = anno.value,
                            AnnotationType::Category => operator.category = anno.value,
                            AnnotationType::Description => operator.description = anno.value,
                            AnnotationType::Latex => operator.latex_formula = Some(anno.value),
                            AnnotationType::Input => {
                                if let Some(input_def) = self.parse_input_annotation(&anno.value) {
                                    operator.add_input(input_def);
                                }
                            }
                            AnnotationType::InputLatex => {
                                // 应用到最后一个输入参数
                                if let Some(last_input) = operator.inputs.last_mut() {
                                    last_input.latex_name = Some(anno.value);
                                }
                            }
                            AnnotationType::InputPaperRef => {
                                // 应用到最后一个输入参数
                                if let Some(last_input) = operator.inputs.last_mut() {
                                    last_input.paper_ref = Some(anno.value);
                                }
                            }
                            AnnotationType::Output => {
                                if let Some(output_def) = self.parse_output_annotation(&anno.value) {
                                    operator.add_output(output_def);
                                }
                            }
                            AnnotationType::OutputLatex => {
                                // 应用到最后一个输出参数
                                if let Some(last_output) = operator.outputs.last_mut() {
                                    last_output.latex_name = Some(anno.value);
                                }
                            }
                            _ => {}
                        }
                    }

                    // 解析S表达式
                    let sexpr = super::parse(&sexpr_text)?;
                    operator.sexpr = sexpr.clone();

                    // 自动检测算子类型（基于 S 表达式结构）
                    operator.operator_type = auto_detect_operator_type(&sexpr);

                    // 尝试转换为Expr
                    if let Ok(expr) = super::convert(&sexpr) {
                        operator.expr = Some(expr);
                    }
                }
            } else {
                self.current_line += 1;
            }
        }

        // 保存最后一个算子
        if let (Some(ref mut m), Some(op)) = (&mut module, current_operator.take()) {
            m.add_operator(op);
        }

        // 应用最后一个算子之后的模块级注解（@workflow_input, @edge, @broadcast 等）
        if let Some(ref mut m) = module {
            for anno in pending_annotations.drain(..) {
                match anno.anno_type {
                    AnnotationType::Name => m.name = anno.value,
                    AnnotationType::Description => m.description = anno.value,
                    AnnotationType::ExecutionMode => {
                        m.control_flow.execution_mode = anno.value;
                    }
                    AnnotationType::TimeSeries => {
                        for name in anno.value.split(',') {
                            let trimmed = name.trim();
                            if !trimmed.is_empty() {
                                m.control_flow.time_series_inputs.push(trimmed.to_string());
                            }
                        }
                    }
                    AnnotationType::Accumulator => {
                        if let Some(acc) = self.parse_accumulator_annotation(&anno.value) {
                            m.control_flow.accumulators.push(acc);
                        }
                    }
                    AnnotationType::StateVar => {
                        if let Some(sv) = self.parse_state_var_annotation(&anno.value) {
                            m.control_flow.state_vars.push(sv);
                        }
                    }
                    AnnotationType::Termination => {
                        if let Some(term) = self.parse_termination_annotation(&anno.value) {
                            m.control_flow.termination = Some(term);
                        }
                    }
                    AnnotationType::WorkflowInput => {
                        if let Some(wi) = self.parse_workflow_input_annotation(&anno.value) {
                            m.workflow_inputs.push(wi);
                        }
                    }
                    AnnotationType::WorkflowOutput => {
                        if let Some(wo) = self.parse_workflow_output_annotation(&anno.value) {
                            m.workflow_outputs.push(wo);
                        }
                    }
                    AnnotationType::Edge => {
                        if let Some(edge) = self.parse_edge_annotation(&anno.value) {
                            m.edges.push(edge);
                        }
                    }
                    AnnotationType::Broadcast => {
                        if let Some(bc) = self.parse_broadcast_annotation(&anno.value) {
                            m.broadcasts.push(bc);
                        }
                    }
                    AnnotationType::Boundary => {
                        if let Some(b) = self.parse_boundary_annotation(&anno.value) {
                            m.boundaries.push(b);
                        }
                    }
                    AnnotationType::ConnectorIn => {
                        if let Some(c) = self.parse_connector_in_annotation(&anno.value) {
                            m.connectors.push(c);
                        }
                    }
                    AnnotationType::ConnectorOut => {
                        if let Some(c) = self.parse_connector_out_annotation(&anno.value) {
                            m.connectors.push(c);
                        }
                    }
                    AnnotationType::Distribution => {
                        if let Some(dist) = self.parse_distribution_annotation(&anno.value) {
                            m.distributions.push(dist);
                        }
                    }
                    AnnotationType::McSamples => {
                        let samples = anno.value.trim().parse().unwrap_or(1000);
                        let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                        mc.samples = samples;
                    }
                    AnnotationType::McSeed => {
                        let seed = anno.value.trim().parse().ok();
                        let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                        mc.seed = seed;
                    }
                    AnnotationType::McOutput => {
                        let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                        let parts: Vec<&str> = anno.value.split_whitespace().collect();
                        if !parts.is_empty() && parts[0] == "percentiles" {
                            mc.output_percentiles = parts[1..].iter()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                        }
                    }
                    AnnotationType::McParallel => {
                        let mc = m.mc_config.get_or_insert_with(MonteCarloConfigDef::default);
                        mc.parallel = anno.value.trim() == "true";
                    }
                    _ => {}
                }
            }
        }

        // 如果没有模块定义，创建默认模块
        module.ok_or_else(|| SExprError::UnexpectedToken {
            expected: "@module 注解".to_string(),
            found: "文件结束".to_string(),
            span: Span::new(1, 1, 0, 0),
        })
    }

    /// 解析注解行
    fn parse_annotation(&self, line: &str) -> Option<Annotation> {
        // 移除 ;; 前缀和空白
        let content = line.strip_prefix(";;")?.trim();
        
        // 解析 @key: value 格式
        if !content.starts_with('@') {
            return None;
        }

        let content = &content[1..]; // 移除 @
        let (key, value) = content.split_once(':')?;
        let key = key.trim().to_lowercase();
        let value = value.trim().to_string();

        let anno_type = match key.as_str() {
            "module" => AnnotationType::Module,
            "operator" => AnnotationType::Operator,
            "type" => panic!("错误: @type 注解已废弃！算子类型由系统根据 S 表达式结构自动检测。请移除 @type 注解。"),
            "name" => AnnotationType::Name,
            "category" => AnnotationType::Category,
            "description" => AnnotationType::Description,
            "latex" => AnnotationType::Latex,
            "input" => AnnotationType::Input,
            "input_latex" => AnnotationType::InputLatex,
            "input_paper_ref" => AnnotationType::InputPaperRef,
            "output" => AnnotationType::Output,
            "output_latex" => AnnotationType::OutputLatex,
            // 工作流控制流配置
            "execution_mode" => AnnotationType::ExecutionMode,
            "time_series" => AnnotationType::TimeSeries,
            "accumulator" => AnnotationType::Accumulator,
            "state_var" => AnnotationType::StateVar,
            "termination" => AnnotationType::Termination,
            // 工作流级别输入输出
            "workflow_input" => AnnotationType::WorkflowInput,
            "workflow_output" => AnnotationType::WorkflowOutput,
            // 显式数据流连接
            "edge" => AnnotationType::Edge,
            "broadcast" => AnnotationType::Broadcast,
            // Forrester 系统边界
            "boundary" => AnnotationType::Boundary,
            // 模块间耦合接口
            "connector_in" => AnnotationType::ConnectorIn,
            "connector_out" => AnnotationType::ConnectorOut,
            // 概率/随机扩展
            "distribution" => AnnotationType::Distribution,
            "mc_samples" => AnnotationType::McSamples,
            "mc_seed" => AnnotationType::McSeed,
            "mc_output" => AnnotationType::McOutput,
            "mc_parallel" => AnnotationType::McParallel,
            _ => return None,
        };

        Some(Annotation { anno_type, value })
    }

    /// 解析输入参数注解
    /// 格式: name, Type, required|optional, description[, default]
    fn parse_input_annotation(&self, value: &str) -> Option<InputDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].clone();
        let data_type = parts[1].clone();
        let required = parts[2].to_lowercase() == "required";
        let description = parts[3].clone();
        let default_value = parts.get(4).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() { return None; }
            Some(serde_json::from_str(trimmed)
                .unwrap_or_else(|_| serde_json::json!(trimmed)))
        });

        Some(InputDef {
            name,
            data_type,
            required,
            description,
            default_value,
            latex_name: None,
            paper_ref: None,
        })
    }

    /// 解析输出参数注解
    /// 格式: name, Type, description
    fn parse_output_annotation(&self, value: &str) -> Option<OutputDef> {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        if parts.len() < 3 {
            return None;
        }

        let name = parts[0].to_string();
        let data_type = parts[1].to_string();
        let description = parts[2].to_string();

        Some(OutputDef {
            name,
            data_type,
            description,
            latex_name: None,
        })
    }

    /// 解析累加器注解
    /// 格式: name from node.port op operation init value
    /// 示例: y from chill_portion.delta_cp op sum init 0
    fn parse_accumulator_annotation(&self, value: &str) -> Option<AccumulatorDef> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        // 最少需要: name from node.port op operation init value
        if parts.len() < 7 {
            return None;
        }

        let name = parts[0].to_string();
        
        // 解析 "from node.port"
        if parts[1] != "from" {
            return None;
        }
        let source_parts: Vec<&str> = parts[2].split('.').collect();
        if source_parts.len() != 2 {
            return None;
        }
        let source_node = source_parts[0].to_string();
        let source_port = source_parts[1].to_string();

        // 解析 "op operation"
        if parts[3] != "op" {
            return None;
        }
        let operation = parts[4].to_string();

        // 解析 "init value"
        if parts[5] != "init" {
            return None;
        }
        let initial_value = parts[6].parse().unwrap_or(0.0);

        Some(AccumulatorDef {
            name,
            source_node,
            source_port,
            operation,
            initial_value,
        })
    }

    /// 解析状态变量注解
    /// 格式: name from node.port init value lag steps
    /// 示例: x_prev from precursor_update.x init 1.0 lag 1
    fn parse_state_var_annotation(&self, value: &str) -> Option<StateVarDef> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        // 最少需要: name from node.port init value
        if parts.len() < 5 {
            return None;
        }

        let name = parts[0].to_string();
        
        // 解析 "from node.port"
        if parts[1] != "from" {
            return None;
        }
        let source_parts: Vec<&str> = parts[2].split('.').collect();
        if source_parts.len() != 2 {
            return None;
        }
        let source_node = source_parts[0].to_string();
        let source_port = source_parts[1].to_string();

        // 解析 "init value" 或 "init dynamic(node.port)"
        if parts[3] != "init" {
            return None;
        }
        let init_str = parts[4];
        let (initial_value, dynamic_init) = if init_str.starts_with("dynamic(") && init_str.ends_with(')') {
            // dynamic(equilibrium.x_eq) -> 动态初始化
            let inner = &init_str[8..init_str.len()-1];
            (0.0, Some(inner.to_string()))
        } else {
            (init_str.parse().unwrap_or(0.0), None)
        };

        // 解析可选的 "lag steps"
        let lag = if parts.len() >= 7 && parts[5] == "lag" {
            parts[6].parse().unwrap_or(1)
        } else {
            1
        };

        Some(StateVarDef {
            name,
            source_node,
            source_port,
            initial_value,
            lag,
            dynamic_init,
        })
    }

    /// 解析终止条件注解
    /// 格式: accumulator name op threshold
    /// 示例: accumulator z >= 6172
    fn parse_termination_annotation(&self, value: &str) -> Option<TerminationDef> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "accumulator" => {
                // 格式: accumulator name op threshold
                if parts.len() < 4 {
                    return None;
                }
                Some(TerminationDef {
                    condition_type: "accumulator_threshold".to_string(),
                    accumulator_name: Some(parts[1].to_string()),
                    op: Some(parts[2].to_string()),
                    threshold: parts[3].parse().ok(),
                })
            }
            "exhaust" => {
                Some(TerminationDef {
                    condition_type: "exhaust_input".to_string(),
                    accumulator_name: None,
                    op: None,
                    threshold: None,
                })
            }
            "iterations" => {
                // 格式: iterations count
                if parts.len() < 2 {
                    return None;
                }
                Some(TerminationDef {
                    condition_type: "fixed_iterations".to_string(),
                    accumulator_name: None,
                    op: None,
                    threshold: parts[1].parse().ok(),
                })
            }
            _ => None,
        }
    }

    /// 解析工作流输入注解
    /// 格式: name, Type, required|optional, description[, default][, bind node.port][, class driving|parameter|control]
    /// 示例: T, Array<Number>, required, 小时温度序列
    /// 示例: yc, Number, optional, 冷量需求阈值, 40, bind pfcn.yc, class parameter
    /// 示例: T, Array<Number>, optional, 温度序列, [5,4,6], bind t.T, class driving
    fn parse_workflow_input_annotation(&self, value: &str) -> Option<WorkflowInputDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].to_string();
        let data_type = parts[1].to_string();
        let required = parts[2].to_lowercase() == "required";
        let description = parts[3].to_string();
        let default_value = parts.get(4).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed.starts_with("bind ") || trimmed.starts_with("class ") {
                None
            } else {
                Some(serde_json::from_str(trimmed)
                    .unwrap_or_else(|_| serde_json::json!(trimmed)))
            }
        });

        // 扫描剩余部分寻找 bind 和 class 关键字
        let mut bind_to_node = None;
        let mut bind_to_port = None;
        let mut var_class = None;

        for part in parts.iter().skip(4) {
            let trimmed = part.trim();
            if let Some(stripped) = trimmed.strip_prefix("bind ") {
                let bind_parts: Vec<&str> = stripped.trim().split('.').collect();
                if bind_parts.len() == 2 {
                    bind_to_node = Some(bind_parts[0].to_string());
                    bind_to_port = Some(bind_parts[1].to_string());
                }
            } else if let Some(cls) = trimmed.strip_prefix("class ") {
                var_class = match cls.trim() {
                    "driving" => Some(VarClass::Driving),
                    "parameter" => Some(VarClass::Parameter),
                    "control" => Some(VarClass::Control),
                    _ => None,
                };
            }
        }

        Some(WorkflowInputDef {
            name,
            data_type,
            required,
            description,
            default_value,
            latex_name: None,
            paper_ref: None,
            bind_to_node,
            bind_to_port,
            var_class,
        })
    }

    /// 解析工作流输出注解
    /// 格式: name, Type, description, source_type source_name[.port]
    /// 示例: y_final, Number, 最终冷量累积, accumulator y
    /// 示例: bloom_date, Date, 初花期预测日期, accumulator z
    /// 示例: reached, Boolean, 是否达到阈值, node stage_check.reached
    fn parse_workflow_output_annotation(&self, value: &str) -> Option<WorkflowOutputDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].clone();
        let data_type = parts[1].clone();
        let description = parts[2].clone();
        
        // 解析来源: "accumulator name" 或 "node name.port"
        let source_str = parts[3].trim();
        let source_parts: Vec<&str> = source_str.split_whitespace().collect();
        
        if source_parts.len() < 2 {
            return None;
        }

        let source_type = source_parts[0].to_string();
        let (source_name, source_port) = if source_type == "node" {
            let node_parts: Vec<&str> = source_parts[1].split('.').collect();
            if node_parts.len() == 2 {
                (Some(node_parts[0].to_string()), Some(node_parts[1].to_string()))
            } else {
                (Some(source_parts[1].to_string()), None)
            }
        } else {
            // accumulator
            (Some(source_parts[1].to_string()), None)
        };

        Some(WorkflowOutputDef {
            name,
            data_type,
            description,
            latex_name: None,
            source_type,
            source_name,
            source_port,
        })
    }

    /// 解析显式边注解
    /// 格式: source_node.port -> target_node.port [lag N] [init VALUE] [flow material|info|control]
    /// 示例: temp_kelvin.TK -> rate_k0.TK
    /// 示例: @state.S -> precursor_update.S
    /// 示例: photosynthesis.carbon -> reserve.input flow material
    fn parse_edge_annotation(&self, value: &str) -> Option<EdgeDef> {
        // 分割 "->"
        let parts: Vec<&str> = value.split("->").collect();
        if parts.len() != 2 {
            return None;
        }

        let source_str = parts[0].trim();
        let rest = parts[1].trim();

        // 解析 source_node.port
        let source_parts: Vec<&str> = source_str.splitn(2, '.').collect();
        if source_parts.len() != 2 {
            return None;
        }
        let source_node = source_parts[0].to_string();
        let source_port = source_parts[1].to_string();

        // 解析 target_node.port [lag N] [init VALUE] [flow TYPE]
        let rest_parts: Vec<&str> = rest.split_whitespace().collect();
        if rest_parts.is_empty() {
            return None;
        }

        let target_str = rest_parts[0];
        let target_parts: Vec<&str> = target_str.splitn(2, '.').collect();
        if target_parts.len() != 2 {
            return None;
        }
        let target_node = target_parts[0].to_string();
        let target_port = target_parts[1].to_string();

        // 解析可选 lag, init, flow
        let mut lag = None;
        let mut init_value = None;
        let mut flow_type = FlowType::Info;
        let mut i = 1;
        while i < rest_parts.len() {
            match rest_parts[i] {
                "lag" => {
                    if i + 1 < rest_parts.len() {
                        lag = rest_parts[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "init" => {
                    if i + 1 < rest_parts.len() {
                        init_value = Some(rest_parts[i + 1].to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "flow" => {
                    if i + 1 < rest_parts.len() {
                        flow_type = match rest_parts[i + 1] {
                            "material" => FlowType::Material,
                            "control" => FlowType::Control,
                            _ => FlowType::Info,
                        };
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => { i += 1; }
            }
        }

        Some(EdgeDef {
            source_node,
            source_port,
            target_node,
            target_port,
            lag,
            init_value,
            flow_type,
        })
    }

    /// 解析广播注解
    /// 格式: input_name -> node1.port, node2.port, ...
    /// 示例: T -> temp_kelvin.T, gdh.T
    fn parse_broadcast_annotation(&self, value: &str) -> Option<BroadcastDef> {
        let parts: Vec<&str> = value.split("->").collect();
        if parts.len() != 2 {
            return None;
        }

        let input_name = parts[0].trim().to_string();
        let targets_str = parts[1].trim();

        let targets: Vec<(String, String)> = targets_str
            .split(',')
            .filter_map(|t| {
                let t = t.trim();
                let tp: Vec<&str> = t.splitn(2, '.').collect();
                if tp.len() == 2 {
                    Some((tp[0].to_string(), tp[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        if targets.is_empty() {
            return None;
        }

        Some(BroadcastDef {
            input_name,
            targets,
        })
    }

    /// 解析系统边界注解
    /// 格式: name, source|sink, description
    /// 示例: atmosphere_co2, source, 大气CO₂（光合碳源）
    /// 示例: respiration_loss, sink, 呼吸碳损失
    fn parse_boundary_annotation(&self, value: &str) -> Option<BoundaryDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 3 {
            return None;
        }
        let boundary_type = match parts[1].to_lowercase().as_str() {
            "source" => BoundaryType::Source,
            "sink" => BoundaryType::Sink,
            _ => return None,
        };
        Some(BoundaryDef {
            name: parts[0].to_string(),
            boundary_type,
            description: parts[2].to_string(),
        })
    }

    /// 解析 connector_out 注解
    /// 格式: name, type, description
    /// 示例: bloom_date, Number, 盛花期日期(JDay)
    fn parse_connector_out_annotation(&self, value: &str) -> Option<ConnectorDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 3 {
            return None;
        }
        Some(ConnectorDef {
            name: parts[0].clone(),
            data_type: parts[1].clone(),
            description: parts[2].clone(),
            direction: ConnectorDirection::Out,
            remote_source: None,
        })
    }

    /// 解析 connector_in 注解
    /// 格式: name, type, from module.port
    /// 示例: bloom_date, Number, from phenoflex.bloom_date
    fn parse_connector_in_annotation(&self, value: &str) -> Option<ConnectorDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 3 {
            return None;
        }
        let description_or_from = parts[2].trim();
        let (description, remote_source) = if let Some(stripped) = description_or_from.strip_prefix("from ") {
            (String::new(), Some(stripped.trim().to_string()))
        } else {
            let remote = parts.get(3).and_then(|p| {
                let t = p.trim();
                t.strip_prefix("from ").map(|s| s.trim().to_string())
            });
            (description_or_from.to_string(), remote)
        };
        Some(ConnectorDef {
            name: parts[0].clone(),
            data_type: parts[1].clone(),
            description,
            direction: ConnectorDirection::In,
            remote_source,
        })
    }

    /// 解析分布注解
    /// 格式: param_name, dist_type, key=value, key=value, ...
    /// 示例: A0, normal, mean=6319.5, std=500
    fn parse_distribution_annotation(&self, value: &str) -> Option<DistributionDef> {
        let parts = split_annotation_fields(value);
        if parts.len() < 3 {
            return None;
        }

        let param_name = parts[0].clone();
        let dist_type = parts[1].clone();

        let mut params = HashMap::new();
        for part in &parts[2..] {
            if let Some((k, v)) = part.split_once('=') {
                if let Ok(val) = v.trim().parse::<f64>() {
                    params.insert(k.trim().to_string(), val);
                }
            }
        }

        Some(DistributionDef {
            param_name,
            dist_type,
            params,
        })
    }

    /// 收集多行S表达式
    fn collect_sexpr(&mut self) -> SExprResult<String> {
        let mut result = String::new();
        let mut paren_count = 0;
        let mut started = false;

        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            // 跳过注释
            if line.starts_with(';') {
                self.current_line += 1;
                continue;
            }

            // 跳过空行（如果还没开始）
            if line.is_empty() && !started {
                self.current_line += 1;
                continue;
            }

            // 空行结束（如果已经开始且括号已匹配）
            if line.is_empty() && started && paren_count == 0 {
                break;
            }

            // 添加到结果
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(line);
            started = true;

            // 计算括号
            for ch in line.chars() {
                match ch {
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    _ => {}
                }
            }

            self.current_line += 1;

            // 如果括号匹配，结束
            if started && paren_count == 0 {
                break;
            }
        }

        Ok(result)
    }
}

/// 便捷函数：解析带注解的S表达式文件
///
/// # 参数
/// - `input`: 带注解的S表达式文件内容
///
/// # 返回
/// - `Ok(ModuleDef)`: 解析成功
/// - `Err(SExprError)`: 解析失败
///
/// # 示例
/// ```ignore
/// let module = parse_annotated_sexpr(r#"
/// ;; @module: math.basic
/// ;; @name: 基础数学运算
/// ;; @description: 提供基础数学运算算子
///
/// ;; @operator: math.add
/// ;; @name: 加法
/// ;; @category: 算术
/// ;; @description: 计算两数之和
/// ;; @input: a, Number, required, 第一个加数
/// ;; @input: b, Number, required, 第二个加数
/// ;; @output: result, Number, 计算结果
/// (add a b)
/// "#)?;
/// ```
/// 智能分割注解字段：按逗号分隔，但尊重 `[]`、`<>`、`{}` 内的逗号
///
/// 例如 `"T, Array<Map<String,Number>>, optional, 温度, [5,4,6], bind t.T, class driving"`
/// 会被正确拆分为 7 个字段，括号内的逗号不作为分隔符。
fn split_annotation_fields(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth: u32 = 0;

    for ch in value.chars() {
        match ch {
            '[' | '<' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ']' | '>' | '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        parts.push(trimmed);
    }
    parts
}

pub fn parse_annotated_sexpr(input: &str) -> SExprResult<ModuleDef> {
    let mut parser = AnnotatedSExprParser::new(input);
    parser.parse()
}

/// Forrester 变量分类自动推断
///
/// 根据 ModuleDef 的结构自动推断每个元素的 VarClass
fn infer_var_classes(module: &ModuleDef) -> HashMap<String, VarClass> {
    let mut classes = HashMap::new();

    // 1. 累加器 → State (S)
    for acc in &module.control_flow.accumulators {
        classes.insert(acc.name.clone(), VarClass::State);
    }

    // 2. 状态变量 → State (S) 或 SemiState (M)
    for sv in &module.control_flow.state_vars {
        if sv.lag > 1 {
            classes.insert(sv.name.clone(), VarClass::SemiState);
        } else {
            classes.insert(sv.name.clone(), VarClass::State);
        }
    }

    // 3. 系统边界 → Boundary (B)
    for b in &module.boundaries {
        classes.insert(b.name.clone(), VarClass::Boundary);
    }

    // 4. 收集速率节点 ID（输出被 accumulator/state_var 引用的算子）
    let rate_nodes: HashSet<String> = module.control_flow.accumulators.iter()
        .map(|a| a.source_node.clone())
        .chain(module.control_flow.state_vars.iter().map(|s| s.source_node.clone()))
        .collect();

    // 5. 算子分类：速率变量 (V) vs 辅助变量 (A)
    for op in &module.operators {
        let short_id = op.id.split('.').next_back().unwrap_or(&op.id).to_string();
        if rate_nodes.contains(&short_id) {
            classes.insert(short_id, VarClass::Rate);
        } else {
            classes.insert(short_id, VarClass::Auxiliary);
        }
    }

    // 6. 工作流输入：显式 class > time_series→Driving > 有默认值的非数组→Parameter
    for input in &module.workflow_inputs {
        if let Some(ref vc) = input.var_class {
            classes.insert(input.name.clone(), vc.clone());
        } else if module.control_flow.time_series_inputs.contains(&input.name) {
            classes.insert(input.name.clone(), VarClass::Driving);
        } else if input.default_value.is_some() && !input.data_type.starts_with("Array") {
            classes.insert(input.name.clone(), VarClass::Parameter);
        }
    }

    classes
}

/// 生成Workflow JSON
///
/// 根据ModuleDef生成符合lowcode-workflows.definition的JSON结构
/// 包含 Forrester 变量分类 (var_class)、流类型 (flow_type)、系统边界 (boundaries)
pub fn generate_workflow_json(module: &ModuleDef) -> serde_json::Value {
    use serde_json::json;

    // ========== Forrester 分类推断 ==========
    let var_classes = infer_var_classes(module);
    let rate_node_ids: HashSet<String> = var_classes.iter()
        .filter(|(_, vc)| **vc == VarClass::Rate)
        .map(|(k, _)| k.clone())
        .collect();

    let mut edges = Vec::new();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut connected_nodes: HashSet<String> = HashSet::new();
    let mut edge_id = 1;

    // 收集边界节点名称，用于在边中将其映射为虚拟节点（_boundary_xxx）
    let boundary_names: HashSet<String> = module.boundaries.iter().map(|b| b.name.clone()).collect();

    // ========== 显式边模式 ==========
    for edge_def in &module.edges {
        let (edge_type, lag) = if edge_def.source_node.starts_with("@state") {
            ("state", edge_def.lag.unwrap_or(1))
        } else if edge_def.source_node.starts_with("@acc") {
            ("accumulator_read", 0u32)
        } else {
            ("data", 0u32)
        };

        let source_node = edge_def.source_node.trim_start_matches("@state.").trim_start_matches("@acc.");
        let (s_node, s_port) = if edge_def.source_node.starts_with('@') {
            ("_virtual".to_string(), source_node.to_string())
        } else if boundary_names.contains(&edge_def.source_node) {
            // 边界节点映射为 _boundary_xxx，调度器会跳过以 _ 开头的虚拟节点
            (format!("_boundary_{}", edge_def.source_node), edge_def.source_port.clone())
        } else {
            (edge_def.source_node.clone(), edge_def.source_port.clone())
        };

        // 如果目标是边界节点，也映射为虚拟节点
        let (t_node, t_port) = if boundary_names.contains(&edge_def.target_node) {
            (format!("_boundary_{}", edge_def.target_node), edge_def.target_port.clone())
        } else {
            (edge_def.target_node.clone(), edge_def.target_port.clone())
        };

        let flow_type_str = match edge_def.flow_type {
            FlowType::Material => "material",
            FlowType::Info => "info",
            FlowType::Control => "control",
        };

        edges.push(json!({
            "id": format!("e{}", edge_id),
            "source": {"node": &s_node, "port": &s_port},
            "target": {"node": &t_node, "port": &t_port},
            "edge_type": edge_type,
            "flow_type": flow_type_str,
            "lag": lag
        }));
        edge_id += 1;

        if !s_node.starts_with('_') {
            connected_nodes.insert(s_node);
        }
        if !t_node.starts_with('_') {
            connected_nodes.insert(t_node);
        }
    }

    // 处理 @broadcast
    for bc in &module.broadcasts {
        for (target_node, target_port) in &bc.targets {
            edges.push(json!({
                "id": format!("e{}", edge_id),
                "source": {"node": "_input", "port": &bc.input_name},
                "target": {"node": target_node, "port": target_port},
                "edge_type": "broadcast",
                "flow_type": "info",
                "lag": 0
            }));
            edge_id += 1;
            connected_nodes.insert(target_node.clone());
        }
    }

    // 收集已连接节点
    for wi in &module.workflow_inputs {
        if let Some(ref node) = wi.bind_to_node {
            connected_nodes.insert(node.clone());
        }
    }
    for wo in &module.workflow_outputs {
        if wo.source_type == "node" {
            if let Some(ref node) = wo.source_name {
                connected_nodes.insert(node.clone());
            }
        }
    }
    for acc in &module.control_flow.accumulators {
        connected_nodes.insert(acc.source_node.clone());
    }
    for sv in &module.control_flow.state_vars {
        connected_nodes.insert(sv.source_node.clone());
    }

    // 如果没有任何连接信息（无边、无输入/输出声明、无累加器/状态变量），
    // 则默认包含所有算子节点
    let include_all_nodes = connected_nodes.is_empty();

    // 生成节点（含 var_class）
    let mut nodes = Vec::new();
    let mut x = 100;
    let y_spacing = 150;
    let mut first_connected_node: Option<String> = None;

    for (idx, operator) in module.operators.iter().enumerate() {
        let node_id = operator.id.split('.').next_back().unwrap_or(&operator.id).to_string();
        if !include_all_nodes && !connected_nodes.contains(&node_id) { continue; }

        let y = 100 + (idx * y_spacing) as i32;
        let mut node_json = json!({
            "id": &node_id,
            "operator_id": &operator.id,
            "position": {"x": x, "y": y},
            "config": {},
            "name": &operator.name,
            "type": operator.operator_type.as_str()
        });
        if let Some(ref latex) = operator.latex_formula {
            node_json["latex_formula"] = json!(latex);
        }
        // 添加 Forrester var_class
        if let Some(vc) = var_classes.get(&node_id) {
            node_json["var_class"] = serde_json::to_value(vc).unwrap_or(json!("auxiliary"));
        }
        let input_params: Vec<serde_json::Value> = operator.inputs.iter().map(|input| {
            let mut param = json!({"name": &input.name, "type": &input.data_type, "required": input.required, "description": &input.description});
            if let Some(ref dv) = input.default_value { param["default"] = dv.clone(); }
            if let Some(ref l) = input.latex_name { param["latex_name"] = json!(l); }
            if let Some(ref p) = input.paper_ref { param["paper_ref"] = json!(p); }
            param
        }).collect();
        let output_params: Vec<serde_json::Value> = operator.outputs.iter().map(|output| {
            let mut param = json!({"name": &output.name, "type": &output.data_type, "description": &output.description});
            if let Some(ref l) = output.latex_name { param["latex_name"] = json!(l); }
            param
        }).collect();
        node_json["input_params"] = json!(input_params);
        node_json["output_params"] = json!(output_params);
        nodes.push(node_json);
        if first_connected_node.is_none() { first_connected_node = Some(node_id.clone()); }
        x += 250;
    }

    // 生成工作流输入（含 var_class）
    if !module.workflow_inputs.is_empty() {
        for wi in &module.workflow_inputs {
            let mut input_json = json!({
                "name": &wi.name, "type": &wi.data_type, "required": wi.required,
                "description": &wi.description
            });
            if let Some(ref default) = wi.default_value {
                input_json["default"] = default.clone();
            }

            let mut bound_node: Option<String> = None;
            let mut bound_port: Option<String> = None;
            if let (Some(ref node), Some(ref port)) = (&wi.bind_to_node, &wi.bind_to_port) {
                input_json["bind_to"] = json!({"node": node, "port": port});
                bound_node = Some(node.clone());
                bound_port = Some(port.clone());
            }
            // 继承 latex/paper_ref
            if let (Some(ref nid), Some(ref pname)) = (&bound_node, &bound_port) {
                for operator in &module.operators {
                    let op_nid = operator.id.split('.').next_back().unwrap_or(&operator.id);
                    if op_nid == nid {
                        for inp in &operator.inputs {
                            if &inp.name == pname {
                                if wi.latex_name.is_none() { if let Some(ref l) = inp.latex_name { input_json["latex_name"] = json!(l); } }
                                if wi.paper_ref.is_none() { if let Some(ref p) = inp.paper_ref { input_json["paper_ref"] = json!(p); } }
                                break;
                            }
                        }
                        break;
                    }
                }
            }
            if let Some(ref l) = wi.latex_name { input_json["latex_name"] = json!(l); }
            if let Some(ref p) = wi.paper_ref { input_json["paper_ref"] = json!(p); }
            // 添加 var_class
            if let Some(vc) = var_classes.get(&wi.name) {
                input_json["var_class"] = serde_json::to_value(vc).unwrap_or(json!("parameter"));
            }
            inputs.push(input_json);
        }
    } else if let Some(ref first_node_id) = first_connected_node {
        let input_providers: HashMap<String, (String, String)> = module.operators.iter().flat_map(|op| {
            let nid = op.id.split('.').next_back().unwrap_or(&op.id).to_string();
            op.outputs.iter().map(move |o| (o.name.clone(), (nid.clone(), o.name.clone())))
        }).collect();
        for operator in &module.operators {
            let node_id = operator.id.split('.').next_back().unwrap_or(&operator.id).to_string();
            if &node_id == first_node_id {
                for input in &operator.inputs {
                    if !input_providers.contains_key(&input.name) || input_providers.get(&input.name).map(|(n, _)| n) == Some(&node_id) {
                        let mut ij = json!({"name": &input.name, "bind_to": {"node": &node_id, "port": &input.name}, "description": &input.description});
                        if let Some(ref l) = input.latex_name { ij["latex_name"] = json!(l); }
                        if let Some(ref p) = input.paper_ref { ij["paper_ref"] = json!(p); }
                        inputs.push(ij);
                    }
                }
                break;
            }
        }
    }

    // 生成工作流输出
    if !module.workflow_outputs.is_empty() {
        for wo in &module.workflow_outputs {
            let mut oj = json!({"name": &wo.name, "type": &wo.data_type, "description": &wo.description, "source_type": &wo.source_type});
            if let Some(ref l) = wo.latex_name { oj["latex_name"] = json!(l); }
            match wo.source_type.as_str() {
                "accumulator" => {
                    if let Some(ref name) = wo.source_name {
                        oj["bind_from"] = json!({"type": "accumulator", "accumulator": name});
                    }
                }
                "node" => {
                    if let (Some(ref node), Some(ref port)) = (&wo.source_name, &wo.source_port) {
                        oj["bind_from"] = json!({"type": "node", "node": node, "port": port});
                    }
                }
                _ => {}
            }
            outputs.push(oj);
        }
    } else {
        for operator in module.operators.iter().rev() {
            let node_id = operator.id.split('.').next_back().unwrap_or(&operator.id).to_string();
            if connected_nodes.contains(&node_id) {
                for output in &operator.outputs {
                    let mut oj = json!({"name": &output.name, "bind_from": {"type": "node", "node": &node_id, "port": &output.name}, "description": &output.description});
                    if let Some(ref l) = output.latex_name { oj["latex_name"] = json!(l); }
                    outputs.push(oj);
                }
                break;
            }
        }
    }

    // 生成 Mermaid 图
    let mermaid = generate_forrester_mermaid(
        &nodes, &edges, &module.workflow_inputs,
        &module.control_flow, &module.boundaries, &module.connectors,
        &var_classes, &rate_node_ids,
    );

    let mut result = json!({
        "nodes": nodes, "edges": edges, "inputs": inputs, "outputs": outputs,
        "visualization": {"mermaid": mermaid}
    });

    // 边界定义
    if !module.boundaries.is_empty() {
        let boundaries_json: Vec<serde_json::Value> = module.boundaries.iter().map(|b| {
            json!({"name": &b.name, "boundary_type": serde_json::to_value(&b.boundary_type).unwrap_or(json!("source")), "description": &b.description})
        }).collect();
        result["boundaries"] = json!(boundaries_json);
    }

    // connectors 定义
    if !module.connectors.is_empty() {
        let connectors_json: Vec<serde_json::Value> = module.connectors.iter().map(|c| {
            let mut cj = json!({
                "name": &c.name,
                "data_type": &c.data_type,
                "description": &c.description,
                "direction": serde_json::to_value(&c.direction).unwrap_or(json!("out")),
            });
            if let Some(ref src) = c.remote_source {
                cj["remote_source"] = json!(src);
            }
            cj
        }).collect();
        result["connectors"] = json!(connectors_json);
    }

    // var_classes 汇总
    let vc_json: serde_json::Map<String, serde_json::Value> = var_classes.iter()
        .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or(json!("auxiliary"))))
        .collect();
    result["var_classes"] = json!(vc_json);

    // 守恒律验证
    let conservation_warnings = verify_conservation_laws(module);
    if !conservation_warnings.is_empty() {
        let warnings_json: Vec<serde_json::Value> = conservation_warnings.iter().map(|w| {
            json!({"level": serde_json::to_value(&w.level).unwrap_or(json!("warning")), "node": &w.node, "message": &w.message})
        }).collect();
        result["conservation_warnings"] = json!(warnings_json);
    }

    // 控制流配置
    let exec_mode = if module.mc_config.is_some() && module.control_flow.execution_mode.is_empty() {
        "monte_carlo"
    } else if module.control_flow.execution_mode.is_empty() {
        "single"
    } else {
        &module.control_flow.execution_mode
    };

    if exec_mode != "single" {
        let mut control_flow = json!({"execution_mode": exec_mode});

        let mut iteration = json!({});
        if !module.control_flow.time_series_inputs.is_empty() {
            iteration["time_series_inputs"] = json!(module.control_flow.time_series_inputs);
        }
        if !module.control_flow.accumulators.is_empty() {
            let accumulators: Vec<serde_json::Value> = module.control_flow.accumulators.iter().map(|acc| {
                json!({"name": &acc.name, "source_node": &acc.source_node, "source_port": &acc.source_port, "operation": &acc.operation, "initial_value": acc.initial_value})
            }).collect();
            iteration["accumulators"] = json!(accumulators);
        }
        if !module.control_flow.state_vars.is_empty() {
            let state_vars: Vec<serde_json::Value> = module.control_flow.state_vars.iter().map(|sv| {
                let mut svj = json!({"name": &sv.name, "source_node": &sv.source_node, "source_port": &sv.source_port, "initial_value": sv.initial_value, "lag": sv.lag});
                if let Some(ref di) = sv.dynamic_init {
                    let parts: Vec<&str> = di.splitn(2, '.').collect();
                    if parts.len() == 2 {
                        svj["dynamic_init"] = json!({"source_node": parts[0], "source_port": parts[1]});
                    }
                }
                svj
            }).collect();
            iteration["state_vars"] = json!(state_vars);
        }
        if let Some(ref term) = module.control_flow.termination {
            let mut termination = json!({"condition": {"type": &term.condition_type}});
            if let Some(ref name) = term.accumulator_name { termination["condition"]["name"] = json!(name); }
            if let Some(ref op) = term.op {
                let op_str = match op.as_str() { ">=" => "gte", ">" => "gt", "<=" => "lte", "<" => "lt", "==" => "eq", "!=" => "neq", _ => op.as_str() };
                termination["condition"]["op"] = json!(op_str);
            }
            if let Some(threshold) = term.threshold { termination["condition"]["threshold"] = json!(threshold); }
            iteration["termination"] = termination;
        }
        control_flow["iteration"] = iteration;

        if let Some(ref mc) = module.mc_config {
            let mut mc_json = json!({"samples": mc.samples, "parallel": mc.parallel});
            if let Some(seed) = mc.seed { mc_json["seed"] = json!(seed); }
            if !mc.output_percentiles.is_empty() {
                mc_json["output_format"] = json!({"percentiles": mc.output_percentiles});
            }
            control_flow["monte_carlo"] = mc_json;
        }

        if !module.distributions.is_empty() {
            let dists: Vec<serde_json::Value> = module.distributions.iter().map(|d| {
                json!({"name": &d.param_name, "distribution": &d.dist_type, "params": &d.params})
            }).collect();
            if let Some(mc_obj) = control_flow.get_mut("monte_carlo") {
                mc_obj["distributions"] = json!(dists);
            } else {
                control_flow["monte_carlo"] = json!({"samples": 1000, "distributions": dists});
            }
        }

        result["control_flow"] = control_flow;
    }

    result
}

/// 生成 Forrester 风格 Mermaid DAG 图
///
/// 包含子图分组、节点形状区分、边样式区分、classDef 着色
fn generate_forrester_mermaid(
    nodes: &[serde_json::Value],
    edges: &[serde_json::Value],
    workflow_inputs: &[WorkflowInputDef],
    control_flow: &WorkflowControlFlowConfig,
    boundaries: &[BoundaryDef],
    connectors: &[ConnectorDef],
    var_classes: &HashMap<String, VarClass>,
    rate_node_ids: &HashSet<String>,
) -> String {
    let mut m = String::from("flowchart TB\n\n");

    // ========== 驱动变量子图 (R) ==========
    let driving_inputs: Vec<&WorkflowInputDef> = workflow_inputs.iter()
        .filter(|i| var_classes.get(&i.name) == Some(&VarClass::Driving))
        .collect();
    if !driving_inputs.is_empty() {
        m.push_str("    subgraph R_group [\"R Driving Variables\"]\n");
        m.push_str("        direction LR\n");
        for input in &driving_inputs {
            m.push_str(&format!("        {}[/\"{}\"/]\n", input.name, input.description));
        }
        m.push_str("    end\n\n");
    }

    // ========== 模型参数子图 (D) ==========
    let param_inputs: Vec<&WorkflowInputDef> = workflow_inputs.iter()
        .filter(|i| var_classes.get(&i.name) == Some(&VarClass::Parameter))
        .collect();
    if !param_inputs.is_empty() {
        m.push_str("    subgraph D_group [\"D Parameters\"]\n");
        m.push_str("        direction LR\n");
        for input in &param_inputs {
            let label = match &input.default_value {
                Some(d) if !d.is_null() => format!("{}={}", input.name, d),
                _ => input.name.clone(),
            };
            m.push_str(&format!("        {}(\"{}\")\n", input.name, label));
        }
        m.push_str("    end\n\n");
    }

    // ========== 控制变量子图 (C) ==========
    let control_inputs: Vec<&WorkflowInputDef> = workflow_inputs.iter()
        .filter(|i| var_classes.get(&i.name) == Some(&VarClass::Control))
        .collect();
    if !control_inputs.is_empty() {
        m.push_str("    subgraph C_group [\"C Control Variables\"]\n");
        m.push_str("        direction LR\n");
        for input in &control_inputs {
            m.push_str(&format!("        {}{{\"{}\"}} \n", input.name, input.description));
        }
        m.push_str("    end\n\n");
    }

    // ========== 系统边界子图 (B) ==========
    if !boundaries.is_empty() {
        m.push_str("    subgraph B_group [\"B Boundaries\"]\n");
        m.push_str("        direction LR\n");
        for b in boundaries {
            let suffix = if b.boundary_type == BoundaryType::Source { "src" } else { "snk" };
            m.push_str(&format!("        {}>\"{}:{}\"]\n", b.name, suffix, b.description));
        }
        m.push_str("    end\n\n");
    }

    // ========== Connector 子图 (模块间耦合接口) ==========
    if !connectors.is_empty() {
        let out_connectors: Vec<&ConnectorDef> = connectors.iter()
            .filter(|c| c.direction == ConnectorDirection::Out)
            .collect();
        let in_connectors: Vec<&ConnectorDef> = connectors.iter()
            .filter(|c| c.direction == ConnectorDirection::In)
            .collect();
        if !out_connectors.is_empty() {
            m.push_str("    subgraph CONN_OUT [\"Connector Out\"]\n");
            m.push_str("        direction LR\n");
            for c in &out_connectors {
                m.push_str(&format!("        conn_out_{}{{{{\"{}:{}\"}}}}\n", c.name, c.name, c.data_type));
            }
            m.push_str("    end\n\n");
        }
        if !in_connectors.is_empty() {
            m.push_str("    subgraph CONN_IN [\"Connector In\"]\n");
            m.push_str("        direction LR\n");
            for c in &in_connectors {
                let label = if let Some(ref src) = c.remote_source {
                    format!("{}:from {}", c.name, src)
                } else {
                    format!("{}:{}", c.name, c.data_type)
                };
                m.push_str(&format!("        conn_in_{}{{{{\"{}\"}}}} \n", c.name, label));
            }
            m.push_str("    end\n\n");
        }
    }

    // ========== 速率+辅助变量子图 (V/A) ==========
    m.push_str("    subgraph VA_group [\"V/A Computation Nodes\"]\n");
    for node in nodes {
        let id = node["id"].as_str().unwrap_or("unknown");
        let name = node["name"].as_str().unwrap_or(id);
        if rate_node_ids.contains(id) {
            // 速率变量 → 梯形 [/"text"\]
            m.push_str(&format!("        {}[/\"V: {}\"\\]\n", id, name));
        } else {
            // 辅助变量 → 矩形
            m.push_str(&format!("        {}[\"{}\"]\n", id, name));
        }
    }
    m.push_str("    end\n\n");

    // ========== 状态变量子图 (S) ==========
    let has_state = !control_flow.accumulators.is_empty() || !control_flow.state_vars.is_empty();
    if has_state {
        m.push_str("    subgraph S_group [\"S State Variables\"]\n");
        for acc in &control_flow.accumulators {
            m.push_str(&format!("        {}([\"S: {} ({})\"])\n", acc.name, acc.name, acc.operation));
        }
        for sv in &control_flow.state_vars {
            if sv.lag > 1 {
                m.push_str(&format!("        {}([\"M: {} (lag={})\"])\n", sv.name, sv.name, sv.lag));
            } else {
                m.push_str(&format!("        {}([\"S: {} (lag={})\"])\n", sv.name, sv.name, sv.lag));
            }
        }
        m.push_str("    end\n\n");
    }

    // ========== 边渲染 ==========

    // R → V/A（驱动变量到计算节点）
    for input in &driving_inputs {
        if let (Some(ref node), Some(ref port)) = (&input.bind_to_node, &input.bind_to_port) {
            m.push_str(&format!("    {} -->|{}| {}\n", input.name, port, node));
        }
    }

    // D → V/A（参数到计算节点）
    for input in &param_inputs {
        if let (Some(ref node), Some(ref port)) = (&input.bind_to_node, &input.bind_to_port) {
            m.push_str(&format!("    {} -.->|{}| {}\n", input.name, port, node));
        }
    }

    // C → V/A（控制到计算节点）
    for input in &control_inputs {
        if let (Some(ref node), Some(ref port)) = (&input.bind_to_node, &input.bind_to_port) {
            m.push_str(&format!("    {} -. {} .-> {}\n", input.name, port, node));
        }
    }

    // V/A → V/A 数据流边
    for edge in edges {
        let source = edge["source"]["node"].as_str().unwrap_or("?");
        let target = edge["target"]["node"].as_str().unwrap_or("?");
        let port = edge["target"]["port"].as_str().unwrap_or("");
        let edge_type = edge["edge_type"].as_str().unwrap_or("data");
        let flow = edge["flow_type"].as_str().unwrap_or("info");

        // 跳过虚拟来源（state/acc 读取边由下面单独处理）
        if source == "_virtual" || source == "_input" { continue; }
        // 跳过 state/accumulator_read 边（由下面 S→V/A 渲染）
        if edge_type == "state" || edge_type == "accumulator_read" { continue; }

        match flow {
            "material" => m.push_str(&format!("    {} ==>|{}| {}\n", source, port, target)),
            "control" => m.push_str(&format!("    {} -. {} .-> {}\n", source, port, target)),
            _ => m.push_str(&format!("    {} -->|{}| {}\n", source, port, target)),
        }
    }

    // Broadcast 边
    for edge in edges {
        let source = edge["source"]["node"].as_str().unwrap_or("?");
        let target = edge["target"]["node"].as_str().unwrap_or("?");
        let port = edge["target"]["port"].as_str().unwrap_or("");
        let edge_type = edge["edge_type"].as_str().unwrap_or("data");
        if edge_type == "broadcast" && source == "_input" {
            // broadcast 的实际来源是工作流输入名
            let input_name = edge["source"]["port"].as_str().unwrap_or("?");
            m.push_str(&format!("    {} -->|{}| {}\n", input_name, port, target));
        }
    }

    // V → S（速率变量到状态变量，物质流粗线）
    for acc in &control_flow.accumulators {
        m.push_str(&format!("    {} ==>|{}| {}\n", acc.source_node, acc.source_port, acc.name));
    }
    for sv in &control_flow.state_vars {
        m.push_str(&format!("    {} ==>|{}| {}\n", sv.source_node, sv.source_port, sv.name));
    }

    // S → V/A（状态变量反馈到计算节点，虚线）
    for edge in edges {
        let edge_type = edge["edge_type"].as_str().unwrap_or("data");
        let target = edge["target"]["node"].as_str().unwrap_or("?");
        let port = edge["source"]["port"].as_str().unwrap_or("");
        if edge_type == "state" || edge_type == "accumulator_read" {
            m.push_str(&format!("    {} -.->|{}| {}\n", port, port, target));
        }
    }

    m.push('\n');

    // ========== classDef 样式 ==========
    m.push_str("    classDef driving fill:#e1f5fe,stroke:#01579b,stroke-width:2px,color:#01579b\n");
    m.push_str("    classDef param fill:#f3e5f5,stroke:#7b1fa2,stroke-dasharray:5 5\n");
    m.push_str("    classDef control fill:#fff9c4,stroke:#f57f17,stroke-width:2px\n");
    m.push_str("    classDef state fill:#fff3e0,stroke:#e65100,stroke-width:3px\n");
    m.push_str("    classDef semistate fill:#fff3e0,stroke:#e65100,stroke-dasharray:5 5\n");
    m.push_str("    classDef rate fill:#c8e6c9,stroke:#2e7d32,stroke-width:2px\n");
    m.push_str("    classDef aux fill:#e8f5e9,stroke:#4caf50\n");
    m.push_str("    classDef boundary fill:#eceff1,stroke:#607d8b,stroke-width:2px\n");
    m.push_str("    classDef connector fill:#e0f7fa,stroke:#006064,stroke-width:2px\n");
    m.push('\n');

    // ========== 应用样式 ==========
    for input in &driving_inputs { m.push_str(&format!("    class {} driving\n", input.name)); }
    for input in &param_inputs { m.push_str(&format!("    class {} param\n", input.name)); }
    for input in &control_inputs { m.push_str(&format!("    class {} control\n", input.name)); }
    for acc in &control_flow.accumulators { m.push_str(&format!("    class {} state\n", acc.name)); }
    for sv in &control_flow.state_vars {
        if sv.lag > 1 {
            m.push_str(&format!("    class {} semistate\n", sv.name));
        } else {
            m.push_str(&format!("    class {} state\n", sv.name));
        }
    }
    for node in nodes {
        let id = node["id"].as_str().unwrap_or("unknown");
        if rate_node_ids.contains(id) {
            m.push_str(&format!("    class {} rate\n", id));
        } else {
            m.push_str(&format!("    class {} aux\n", id));
        }
    }
    for b in boundaries { m.push_str(&format!("    class {} boundary\n", b.name)); }
    for c in connectors {
        let prefix = if c.direction == ConnectorDirection::Out { "conn_out_" } else { "conn_in_" };
        m.push_str(&format!("    class {}{} connector\n", prefix, c.name));
    }

    m
}

/// 守恒律警告级别
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningLevel {
    Warning,
    Error,
}

/// 守恒律警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationWarning {
    pub level: WarningLevel,
    pub node: String,
    pub message: String,
}

/// 验证守恒律
///
/// 检查规则：
/// - 每个状态变量(S)至少有 1 条 material 入流
/// - 每个 source boundary 至少有 1 条 material 出边
/// - 每个 sink boundary 至少有 1 条 material 入边
pub fn verify_conservation_laws(module: &ModuleDef) -> Vec<ConservationWarning> {
    let mut warnings = Vec::new();

    // 收集所有 material 流的边
    let material_edges: Vec<&EdgeDef> = module.edges.iter()
        .filter(|e| e.flow_type == FlowType::Material)
        .collect();

    // 检查状态变量（accumulator）是否有 material 入流
    for acc in &module.control_flow.accumulators {
        let has_material_in = material_edges.iter().any(|e| {
            e.source_node == acc.source_node && e.source_port == acc.source_port
        }) || material_edges.iter().any(|e| {
            e.target_node == acc.name || e.target_port == acc.name
        });

        // 只在有 material 边的模型中检查（纯信息流模型不需要）
        if !material_edges.is_empty() && !has_material_in {
            warnings.push(ConservationWarning {
                level: WarningLevel::Warning,
                node: acc.name.clone(),
                message: format!(
                    "状态变量 '{}' 没有 material 类型的入流，可能缺少物质守恒标注",
                    acc.name
                ),
            });
        }
    }

    // 检查 source boundary 是否有 material 出边
    for b in &module.boundaries {
        if b.boundary_type == BoundaryType::Source {
            let has_material_out = material_edges.iter().any(|e| e.source_node == b.name);
            if !has_material_out {
                warnings.push(ConservationWarning {
                    level: WarningLevel::Warning,
                    node: b.name.clone(),
                    message: format!(
                        "Source boundary '{}' 没有 material 出边，物质来源不明确",
                        b.name
                    ),
                });
            }
        }
    }

    // 检查 sink boundary 是否有 material 入边
    for b in &module.boundaries {
        if b.boundary_type == BoundaryType::Sink {
            let has_material_in = material_edges.iter().any(|e| e.target_node == b.name);
            if !has_material_in {
                warnings.push(ConservationWarning {
                    level: WarningLevel::Warning,
                    node: b.name.clone(),
                    message: format!(
                        "Sink boundary '{}' 没有 material 入边，物质去向不明确",
                        b.name
                    ),
                });
            }
        }
    }

    warnings
}

/// 生成 L2 级多模块 Mermaid DAG
///
/// 每个模块为一个 subgraph，Connector 连线跨 subgraph
pub fn generate_l2_mermaid(modules: &[ModuleDef]) -> String {
    let mut m = String::from("flowchart TB\n\n");

    // 收集所有 connector_out 供匹配
    let mut out_connectors: HashMap<String, (String, String)> = HashMap::new(); // remote_key -> (module_id, connector_name)
    for module in modules {
        for c in &module.connectors {
            if c.direction == ConnectorDirection::Out {
                let key = format!("{}.{}", module.id, c.name);
                out_connectors.insert(key, (module.id.clone(), c.name.clone()));
            }
        }
    }

    // 渲染每个模块为 subgraph
    for module in modules {
        let safe_id = module.id.replace('.', "_");
        m.push_str(&format!("    subgraph {} [\"{}\"]\n", safe_id, module.name));

        // 简化显示：只显示模块的 connector 端口
        for c in &module.connectors {
            let prefix = if c.direction == ConnectorDirection::Out { "out" } else { "in" };
            let node_id = format!("{}_{}_{}",safe_id, prefix, c.name);
            m.push_str(&format!("        {}{{{{\"{}:{}\"}}}}\n", node_id, c.name, c.data_type));
        }

        // 如果模块没有 connector，显示模块名
        if module.connectors.is_empty() {
            m.push_str(&format!("        {}_core[\"{}\"]\n", safe_id, module.id));
        }

        m.push_str("    end\n\n");
    }

    // 渲染跨模块连线
    for module in modules {
        let safe_id = module.id.replace('.', "_");
        for c in &module.connectors {
            if c.direction == ConnectorDirection::In {
                if let Some(ref src) = c.remote_source {
                    if let Some((src_module_id, src_name)) = out_connectors.get(src) {
                        let src_safe_id = src_module_id.replace('.', "_");
                        let src_node_id = format!("{}_out_{}", src_safe_id, src_name);
                        let tgt_node_id = format!("{}_in_{}", safe_id, c.name);
                        m.push_str(&format!("    {} ==>|coupling| {}\n", src_node_id, tgt_node_id));
                    }
                }
            }
        }
    }

    // 样式
    m.push_str("\n    classDef connector fill:#e0f7fa,stroke:#006064,stroke-width:2px\n");

    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SAMPLE_SEXPR: &str = r#"
;; @module: phenoflex.chill
;; @name: PhenoFlex冷量模型
;; @description: 基于Dynamic Model的冷量累积计算

;; @operator: phenoflex.temp_kelvin
;; @name: 温度转开尔文
;; @category: 物理转换
;; @description: 将摄氏度转换为开尔文温度
;; @input: T, Number, required, 温度(摄氏度)
;; @input: offset, Number, optional, 偏移量, 273
;; @output: TK, Number, 开尔文温度
(add T offset)

;; @operator: phenoflex.arrhenius
;; @name: Arrhenius速率常数
;; @category: 化学动力学
;; @description: 计算Arrhenius方程的速率常数
;; @input: A, Number, required, 频率因子
;; @input: E, Number, required, 活化能(cal/mol)
;; @input: R, Number, optional, 气体常数, 1.987
;; @input: TK, Number, required, 温度(K)
;; @output: k, Number, 速率常数
(mul A (exp (neg (div E (mul R TK)))))
"#;

    #[test]
    fn test_parse_annotated_sexpr() {
        let module = parse_annotated_sexpr(SAMPLE_SEXPR).unwrap();
        
        assert_eq!(module.id, "phenoflex.chill");
        assert_eq!(module.name, "PhenoFlex冷量模型");
        assert_eq!(module.operators.len(), 2);

        let first_op = &module.operators[0];
        assert_eq!(first_op.id, "phenoflex.temp_kelvin");
        assert_eq!(first_op.name, "温度转开尔文");
        assert_eq!(first_op.category, "物理转换");
        assert_eq!(first_op.inputs.len(), 2);
        assert_eq!(first_op.outputs.len(), 1);

        let second_op = &module.operators[1];
        assert_eq!(second_op.id, "phenoflex.arrhenius");
        assert_eq!(second_op.inputs.len(), 4);
    }

    #[test]
    fn test_input_parsing() {
        let module = parse_annotated_sexpr(SAMPLE_SEXPR).unwrap();
        let first_op = &module.operators[0];
        
        let t_input = &first_op.inputs[0];
        assert_eq!(t_input.name, "T");
        assert_eq!(t_input.data_type, "Number");
        assert!(t_input.required);
        assert_eq!(t_input.description, "温度(摄氏度)");
        assert!(t_input.default_value.is_none());

        let offset_input = &first_op.inputs[1];
        assert_eq!(offset_input.name, "offset");
        assert!(!offset_input.required);
        assert_eq!(offset_input.default_value, Some(json!(273)));
    }

    #[test]
    fn test_generate_workflow_json() {
        let module = parse_annotated_sexpr(SAMPLE_SEXPR).unwrap();
        let json = generate_workflow_json(&module);
        
        let nodes = json["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        
        let first_node = &nodes[0];
        assert_eq!(first_node["id"], "temp_kelvin");
        assert_eq!(first_node["operator_id"], "phenoflex.temp_kelvin");
    }

    #[test]
    fn test_iteration_config_parsing() {
        const ITER_SEXPR: &str = r#"
;; @module: test.iter
;; @name: 迭代测试模块
;; @description: 测试迭代配置
;; @execution_mode: iterative
;; @time_series: T
;; @accumulator: y from chill.delta_cp op sum init 0
;; @accumulator: z from heat.delta_z op sum init 0
;; @state_var: S from adjust.S init 1.0 lag 1
;; @termination: accumulator z >= 6172

;; @operator: test.chill
;; @name: 测试算子
;; @category: 测试
;; @description: 测试
;; @input: T, Number, required, 温度
;; @output: delta_cp, Number, 输出
(mul T 0.1)
"#;

        let module = parse_annotated_sexpr(ITER_SEXPR).unwrap();
        
        // 验证控制流配置
        assert_eq!(module.control_flow.execution_mode, "iterative");
        assert_eq!(module.control_flow.time_series_inputs, vec!["T"]);
        
        // 验证累加器
        assert_eq!(module.control_flow.accumulators.len(), 2);
        assert_eq!(module.control_flow.accumulators[0].name, "y");
        assert_eq!(module.control_flow.accumulators[0].source_node, "chill");
        assert_eq!(module.control_flow.accumulators[0].source_port, "delta_cp");
        assert_eq!(module.control_flow.accumulators[0].operation, "sum");
        assert_eq!(module.control_flow.accumulators[0].initial_value, 0.0);
        
        // 验证状态变量
        assert_eq!(module.control_flow.state_vars.len(), 1);
        assert_eq!(module.control_flow.state_vars[0].name, "S");
        assert_eq!(module.control_flow.state_vars[0].lag, 1);
        
        // 验证终止条件
        assert!(module.control_flow.termination.is_some());
        let term = module.control_flow.termination.as_ref().unwrap();
        assert_eq!(term.condition_type, "accumulator_threshold");
        assert_eq!(term.accumulator_name, Some("z".to_string()));
        assert_eq!(term.op, Some(">=".to_string()));
        assert_eq!(term.threshold, Some(6172.0));
    }

    #[test]
    fn test_generate_workflow_json_with_iteration() {
        const ITER_SEXPR: &str = r#"
;; @module: test.iter2
;; @name: 迭代JSON测试
;; @description: 测试迭代JSON生成
;; @execution_mode: iterative
;; @time_series: T
;; @accumulator: y from node1.output op sum init 0
;; @termination: accumulator y >= 100

;; @operator: test.node1
;; @name: 节点1
;; @category: 测试
;; @description: 测试
;; @input: T, Number, required, 输入
;; @output: output, Number, 输出
(mul T 2)

;; @operator: test.node2
;; @name: 节点2
;; @category: 测试
;; @description: 测试
;; @input: output, Number, required, 输入
;; @output: result, Number, 结果
(add output 1)
"#;

        let module = parse_annotated_sexpr(ITER_SEXPR).unwrap();
        let json = generate_workflow_json(&module);
        
        // 验证 control_flow 存在
        assert!(json.get("control_flow").is_some());
        
        let control_flow = &json["control_flow"];
        assert_eq!(control_flow["execution_mode"], "iterative");
        
        let iteration = &control_flow["iteration"];
        assert!(iteration["time_series_inputs"].as_array().is_some());
        assert!(iteration["accumulators"].as_array().is_some());
        assert!(iteration["termination"].is_object());
    }

    #[test]
    fn test_workflow_input_output_parsing() {
        const IO_SEXPR: &str = r#"
;; @module: test.io
;; @name: 输入输出测试
;; @description: 测试工作流输入输出
;; @workflow_input: T, Array<Number>, required, 温度序列
;; @workflow_input: yc, Number, optional, 冷量阈值, 40
;; @workflow_output: y_final, Number, 累积冷量, accumulator y
;; @workflow_output: reached, Boolean, 是否达到, node check.result

;; @operator: test.calc
;; @name: 计算
;; @category: 测试
;; @description: 测试
;; @input: T, Number, required, 输入
;; @output: delta, Number, 输出
(mul T 0.1)
"#;

        let module = parse_annotated_sexpr(IO_SEXPR).unwrap();
        
        // 验证工作流输入
        assert_eq!(module.workflow_inputs.len(), 2);
        
        let t_input = &module.workflow_inputs[0];
        assert_eq!(t_input.name, "T");
        assert_eq!(t_input.data_type, "Array<Number>");
        assert!(t_input.required);
        assert_eq!(t_input.description, "温度序列");
        
        let yc_input = &module.workflow_inputs[1];
        assert_eq!(yc_input.name, "yc");
        assert!(!yc_input.required);
        assert_eq!(yc_input.default_value, Some(json!(40)));
        
        // 验证工作流输出
        assert_eq!(module.workflow_outputs.len(), 2);
        
        let y_output = &module.workflow_outputs[0];
        assert_eq!(y_output.name, "y_final");
        assert_eq!(y_output.source_type, "accumulator");
        assert_eq!(y_output.source_name, Some("y".to_string()));
        
        let reached_output = &module.workflow_outputs[1];
        assert_eq!(reached_output.name, "reached");
        assert_eq!(reached_output.source_type, "node");
        assert_eq!(reached_output.source_name, Some("check".to_string()));
        assert_eq!(reached_output.source_port, Some("result".to_string()));
    }

    #[test]
    fn test_generate_workflow_json_with_explicit_io() {
        const IO_SEXPR: &str = r#"
;; @module: test.io2
;; @name: JSON输入输出测试
;; @description: 测试JSON生成
;; @workflow_input: T, Array<Number>, required, 温度序列
;; @workflow_input: threshold, Number, optional, 阈值, 100
;; @workflow_output: result, Number, 计算结果, accumulator sum

;; @operator: test.node1
;; @name: 节点1
;; @category: 测试
;; @description: 测试
;; @input: T, Number, required, 输入
;; @output: output, Number, 输出
(mul T 2)

;; @operator: test.node2
;; @name: 节点2
;; @category: 测试
;; @description: 测试
;; @input: output, Number, required, 输入
;; @output: result, Number, 结果
(add output 1)
"#;

        let module = parse_annotated_sexpr(IO_SEXPR).unwrap();
        let json = generate_workflow_json(&module);
        
        // 验证输入使用显式声明
        let inputs = json["inputs"].as_array().unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0]["name"], "T");
        assert_eq!(inputs[0]["type"], "Array<Number>");
        assert_eq!(inputs[1]["name"], "threshold");
        assert_eq!(inputs[1]["default"], json!(100));
        
        // 验证输出使用显式声明
        let outputs = json["outputs"].as_array().unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0]["name"], "result");
        assert_eq!(outputs[0]["source_type"], "accumulator");
    }
}
