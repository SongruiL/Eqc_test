//! S-expression 验证器
//!
//! 验证带注解的 S-expression 文件是否符合规范

use super::ModuleDef;
use std::collections::{HashMap, HashSet};

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否验证通过
    pub is_valid: bool,
    /// 错误列表（必须修复）
    pub errors: Vec<ValidationError>,
    /// 警告列表（建议修复）
    pub warnings: Vec<ValidationWarning>,
    /// 统计信息
    pub stats: ValidationStats,
}

/// 验证错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// 错误类型
    pub error_type: ErrorType,
    /// 错误消息
    pub message: String,
    /// 相关位置（算子ID、输入名等）
    pub location: Option<String>,
    /// 修复建议
    pub suggestion: Option<String>,
}

/// 验证警告
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// 警告类型
    pub warning_type: WarningType,
    /// 警告消息
    pub message: String,
    /// 相关位置
    pub location: Option<String>,
    /// 修复建议
    pub suggestion: Option<String>,
}

/// 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    /// 缺少必填注解
    MissingRequiredAnnotation,
    /// 无效的注解格式
    InvalidAnnotationFormat,
    /// 重复的标识符
    DuplicateIdentifier,
    /// 未定义的引用
    UndefinedReference,
    /// 输入输出不匹配
    InputOutputMismatch,
    /// 循环依赖
    CircularDependency,
    /// 无效的表达式
    InvalidExpression,
    /// 类型错误
    TypeError,
}

/// 警告类型
#[derive(Debug, Clone, PartialEq)]
pub enum WarningType {
    /// 缺少可选注解
    MissingOptionalAnnotation,
    /// 未使用的输入
    UnusedInput,
    /// 孤立的算子（无连接）
    OrphanOperator,
    /// 缺少描述
    MissingDescription,
    /// 缺少 LaTeX 公式
    MissingLatex,
    /// 默认值格式问题
    DefaultValueFormat,
}

/// 统计信息
#[derive(Debug, Clone, Default)]
pub struct ValidationStats {
    /// 模块数
    pub module_count: usize,
    /// 算子数
    pub operator_count: usize,
    /// 输入参数数
    pub input_count: usize,
    /// 输出参数数
    pub output_count: usize,
    /// 工作流输入数
    pub workflow_input_count: usize,
    /// 工作流输出数
    pub workflow_output_count: usize,
    /// 状态变量数
    pub state_var_count: usize,
    /// 累加器数
    pub accumulator_count: usize,
}

/// S-expression 验证器
pub struct SExprValidator {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationWarning>,
    stats: ValidationStats,
}

impl SExprValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ValidationStats::default(),
        }
    }

    /// 验证模块定义
    pub fn validate(&mut self, module: &ModuleDef) -> ValidationResult {
        self.errors.clear();
        self.warnings.clear();
        self.stats = ValidationStats::default();

        // 收集统计信息
        self.collect_stats(module);

        // 执行各项验证
        self.validate_module_meta(module);
        self.validate_operators(module);
        self.validate_workflow_inputs(module);
        self.validate_workflow_outputs(module);
        self.validate_control_flow(module);
        self.validate_data_flow(module);
        self.validate_explicit_edges(module);
        self.validate_broadcasts(module);
        self.validate_distributions(module);

        ValidationResult {
            is_valid: self.errors.is_empty(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
            stats: self.stats.clone(),
        }
    }

    /// 收集统计信息
    fn collect_stats(&mut self, module: &ModuleDef) {
        self.stats.module_count = 1;
        self.stats.operator_count = module.operators.len();
        self.stats.input_count = module.operators.iter().map(|op| op.inputs.len()).sum();
        self.stats.output_count = module.operators.iter().map(|op| op.outputs.len()).sum();
        self.stats.workflow_input_count = module.workflow_inputs.len();
        self.stats.workflow_output_count = module.workflow_outputs.len();
        self.stats.state_var_count = module.control_flow.state_vars.len();
        self.stats.accumulator_count = module.control_flow.accumulators.len();
    }

    /// 验证模块元信息
    fn validate_module_meta(&mut self, module: &ModuleDef) {
        // 检查模块 ID
        if module.id.is_empty() {
            self.errors.push(ValidationError {
                error_type: ErrorType::MissingRequiredAnnotation,
                message: "模块缺少 @module 注解".to_string(),
                location: None,
                suggestion: Some("添加 ;; @module: your.module.id".to_string()),
            });
        }

        // 检查模块名称
        if module.name.is_empty() {
            self.warnings.push(ValidationWarning {
                warning_type: WarningType::MissingOptionalAnnotation,
                message: "模块缺少 @name 注解".to_string(),
                location: Some(module.id.clone()),
                suggestion: Some("添加 ;; @name: 模块名称".to_string()),
            });
        }

        // 检查模块描述
        if module.description.is_empty() {
            self.warnings.push(ValidationWarning {
                warning_type: WarningType::MissingDescription,
                message: "模块缺少 @description 注解".to_string(),
                location: Some(module.id.clone()),
                suggestion: Some("添加 ;; @description: 模块描述".to_string()),
            });
        }
    }

    /// 验证算子定义
    fn validate_operators(&mut self, module: &ModuleDef) {
        let mut operator_ids: HashSet<String> = HashSet::new();

        for operator in &module.operators {
            // 检查重复 ID
            let short_id = operator.id.split('.').next_back().unwrap_or(&operator.id);
            if operator_ids.contains(short_id) {
                self.errors.push(ValidationError {
                    error_type: ErrorType::DuplicateIdentifier,
                    message: format!("重复的算子 ID: {}", operator.id),
                    location: Some(operator.id.clone()),
                    suggestion: Some("每个算子 ID 必须唯一".to_string()),
                });
            }
            operator_ids.insert(short_id.to_string());

            // 检查算子名称
            if operator.name.is_empty() {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingOptionalAnnotation,
                    message: format!("算子 {} 缺少 @name 注解", operator.id),
                    location: Some(operator.id.clone()),
                    suggestion: Some("添加 ;; @name: 算子名称".to_string()),
                });
            }

            // 检查算子描述
            if operator.description.is_empty() {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingDescription,
                    message: format!("算子 {} 缺少 @description 注解", operator.id),
                    location: Some(operator.id.clone()),
                    suggestion: Some("添加 ;; @description: 算子描述".to_string()),
                });
            }

            // 检查 LaTeX 公式
            if operator.latex_formula.is_none() {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingLatex,
                    message: format!("算子 {} 缺少 @latex 注解", operator.id),
                    location: Some(operator.id.clone()),
                    suggestion: Some("添加 ;; @latex: $公式$".to_string()),
                });
            }

            // 检查输入
            if operator.inputs.is_empty() {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingOptionalAnnotation,
                    message: format!("算子 {} 没有定义输入参数", operator.id),
                    location: Some(operator.id.clone()),
                    suggestion: Some("添加 ;; @input: name, Type, required|optional, description".to_string()),
                });
            }

            // 检查输出
            if operator.outputs.is_empty() {
                self.errors.push(ValidationError {
                    error_type: ErrorType::MissingRequiredAnnotation,
                    message: format!("算子 {} 没有定义输出参数", operator.id),
                    location: Some(operator.id.clone()),
                    suggestion: Some("添加 ;; @output: name, Type, description".to_string()),
                });
            }

            // 检查每个输入参数
            for input in &operator.inputs {
                if input.description.is_empty() {
                    self.warnings.push(ValidationWarning {
                        warning_type: WarningType::MissingDescription,
                        message: format!("算子 {} 的输入 {} 缺少描述", operator.id, input.name),
                        location: Some(format!("{}.{}", operator.id, input.name)),
                        suggestion: Some("在 @input 注解中添加描述".to_string()),
                    });
                }
            }
        }
    }

    /// 验证工作流输入
    fn validate_workflow_inputs(&mut self, module: &ModuleDef) {
        let operator_ids: HashSet<String> = module.operators.iter()
            .map(|op| op.id.split('.').next_back().unwrap_or(&op.id).to_string())
            .collect();

        for wi in &module.workflow_inputs {
            // 检查绑定目标是否存在
            if let Some(ref bind_node) = wi.bind_to_node {
                if !operator_ids.contains(bind_node) {
                    self.errors.push(ValidationError {
                        error_type: ErrorType::UndefinedReference,
                        message: format!(
                            "工作流输入 {} 绑定到不存在的节点: {}",
                            wi.name, bind_node
                        ),
                        location: Some(wi.name.clone()),
                        suggestion: Some(format!(
                            "可用的节点: {:?}",
                            operator_ids.iter().take(5).collect::<Vec<_>>()
                        )),
                    });
                }
            }
        }
    }

    /// 验证工作流输出
    fn validate_workflow_outputs(&mut self, module: &ModuleDef) {
        let operator_ids: HashSet<String> = module.operators.iter()
            .map(|op| op.id.split('.').next_back().unwrap_or(&op.id).to_string())
            .collect();

        let accumulator_names: HashSet<String> = module.control_flow.accumulators.iter()
            .map(|acc| acc.name.clone())
            .collect();

        for wo in &module.workflow_outputs {
            match wo.source_type.as_str() {
                "node" => {
                    if let Some(ref source_node) = wo.source_name {
                        if !operator_ids.contains(source_node) {
                            self.errors.push(ValidationError {
                                error_type: ErrorType::UndefinedReference,
                                message: format!(
                                    "工作流输出 {} 引用不存在的节点: {}",
                                    wo.name, source_node
                                ),
                                location: Some(wo.name.clone()),
                                suggestion: Some(format!(
                                    "可用的节点: {:?}",
                                    operator_ids.iter().take(5).collect::<Vec<_>>()
                                )),
                            });
                        }
                    }
                }
                "accumulator" => {
                    if let Some(ref acc_name) = wo.source_name {
                        if !accumulator_names.contains(acc_name) {
                            self.errors.push(ValidationError {
                                error_type: ErrorType::UndefinedReference,
                                message: format!(
                                    "工作流输出 {} 引用不存在的累加器: {}",
                                    wo.name, acc_name
                                ),
                                location: Some(wo.name.clone()),
                                suggestion: Some(format!(
                                    "可用的累加器: {:?}",
                                    accumulator_names.iter().collect::<Vec<_>>()
                                )),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 验证控制流配置
    fn validate_control_flow(&mut self, module: &ModuleDef) {
        let operator_ids: HashSet<String> = module.operators.iter()
            .map(|op| op.id.split('.').next_back().unwrap_or(&op.id).to_string())
            .collect();

        // 验证累加器
        for acc in &module.control_flow.accumulators {
            if !operator_ids.contains(&acc.source_node) {
                self.errors.push(ValidationError {
                    error_type: ErrorType::UndefinedReference,
                    message: format!(
                        "累加器 {} 引用不存在的节点: {}",
                        acc.name, acc.source_node
                    ),
                    location: Some(acc.name.clone()),
                    suggestion: Some(format!(
                        "可用的节点: {:?}",
                        operator_ids.iter().take(5).collect::<Vec<_>>()
                    )),
                });
            }
        }

        // 验证状态变量
        for sv in &module.control_flow.state_vars {
            if !operator_ids.contains(&sv.source_node) {
                self.errors.push(ValidationError {
                    error_type: ErrorType::UndefinedReference,
                    message: format!(
                        "状态变量 {} 引用不存在的节点: {}",
                        sv.name, sv.source_node
                    ),
                    location: Some(sv.name.clone()),
                    suggestion: Some(format!(
                        "可用的节点: {:?}",
                        operator_ids.iter().take(5).collect::<Vec<_>>()
                    )),
                });
            }
        }

        // 验证迭代模式配置
        if module.control_flow.execution_mode == "iterative" {
            if module.control_flow.time_series_inputs.is_empty() {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingOptionalAnnotation,
                    message: "迭代模式缺少 @time_series 注解".to_string(),
                    location: None,
                    suggestion: Some("添加 ;; @time_series: T 指定时间序列输入".to_string()),
                });
            }

            if module.control_flow.termination.is_none() {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingOptionalAnnotation,
                    message: "迭代模式缺少 @termination 注解".to_string(),
                    location: None,
                    suggestion: Some("添加 ;; @termination: exhaust 或 accumulator y >= 40".to_string()),
                });
            }
        }
    }

    /// 验证数据流（输入输出连接）
    fn validate_data_flow(&mut self, module: &ModuleDef) {
        // 收集所有输出端口
        let mut output_ports: HashMap<String, String> = HashMap::new(); // port_name -> operator_id
        for op in &module.operators {
            for output in &op.outputs {
                let op_short_id = op.id.split('.').next_back().unwrap_or(&op.id);
                output_ports.insert(output.name.clone(), op_short_id.to_string());
            }
        }

        // 收集所有工作流输入名称
        let workflow_input_names: HashSet<String> = module.workflow_inputs.iter()
            .map(|wi| wi.name.clone())
            .collect();

        // 收集状态变量和累加器名称
        let _state_var_names: HashSet<String> = module.control_flow.state_vars.iter()
            .map(|sv| sv.name.clone())
            .collect();
        let _accumulator_names: HashSet<String> = module.control_flow.accumulators.iter()
            .map(|acc| acc.name.clone())
            .collect();

        // 当使用显式边模式时，跳过隐式数据流验证
        if !module.edges.is_empty() || !module.broadcasts.is_empty() {
            return;
        }

        // 检查每个算子的输入是否有来源（仅隐式模式）
        for op in &module.operators {
            for input in &op.inputs {
                let has_source = output_ports.contains_key(&input.name)
                    || workflow_input_names.contains(&input.name)
                    || _state_var_names.contains(&input.name)
                    || _accumulator_names.contains(&input.name);

                if !has_source && input.required {
                    // 不作为错误，因为可能通过工作流输入自动绑定
                }
            }
        }
    }

    /// 验证显式边定义
    fn validate_explicit_edges(&mut self, module: &ModuleDef) {
        if module.edges.is_empty() { return; }

        let operator_ids: HashSet<String> = module.operators.iter()
            .map(|op| op.id.split('.').next_back().unwrap_or(&op.id).to_string())
            .collect();

        // 收集所有算子的输入/输出端口
        let mut op_inputs: HashMap<String, HashSet<String>> = HashMap::new();
        let mut op_outputs: HashMap<String, HashSet<String>> = HashMap::new();
        for op in &module.operators {
            let nid = op.id.split('.').next_back().unwrap_or(&op.id).to_string();
            let ins: HashSet<String> = op.inputs.iter().map(|i| i.name.clone()).collect();
            let outs: HashSet<String> = op.outputs.iter().map(|o| o.name.clone()).collect();
            op_inputs.insert(nid.clone(), ins);
            op_outputs.insert(nid, outs);
        }

        let state_var_names: HashSet<String> = module.control_flow.state_vars.iter()
            .map(|sv| sv.name.clone()).collect();
        let acc_names: HashSet<String> = module.control_flow.accumulators.iter()
            .map(|a| a.name.clone()).collect();

        for edge in &module.edges {
            // 验证源节点
            if edge.source_node.starts_with("@state") {
                // 检查状态变量存在性
                let var_name = edge.source_port.clone();
                if !state_var_names.contains(&var_name) {
                    self.errors.push(ValidationError {
                        error_type: ErrorType::UndefinedReference,
                        message: format!("@edge 引用不存在的状态变量: {}", var_name),
                        location: Some(format!("@edge: {}.{} -> ...", edge.source_node, edge.source_port)),
                        suggestion: Some(format!("已定义的状态变量: {:?}", state_var_names)),
                    });
                }
            } else if edge.source_node.starts_with("@acc") {
                let var_name = edge.source_port.clone();
                if !acc_names.contains(&var_name) {
                    self.errors.push(ValidationError {
                        error_type: ErrorType::UndefinedReference,
                        message: format!("@edge 引用不存在的累加器: {}", var_name),
                        location: Some(format!("@edge: {}.{} -> ...", edge.source_node, edge.source_port)),
                        suggestion: Some(format!("已定义的累加器: {:?}", acc_names)),
                    });
                }
            } else if !operator_ids.contains(&edge.source_node) {
                self.errors.push(ValidationError {
                    error_type: ErrorType::UndefinedReference,
                    message: format!("@edge 源节点不存在: {}", edge.source_node),
                    location: Some(format!("@edge: {}.{} -> {}.{}", edge.source_node, edge.source_port, edge.target_node, edge.target_port)),
                    suggestion: Some(format!("可用节点: {:?}", operator_ids.iter().take(5).collect::<Vec<_>>())),
                });
            } else if let Some(ports) = op_outputs.get(&edge.source_node) {
                if !ports.contains(&edge.source_port) {
                    self.errors.push(ValidationError {
                        error_type: ErrorType::UndefinedReference,
                        message: format!("@edge 源端口不存在: {}.{}", edge.source_node, edge.source_port),
                        location: Some(format!("@edge: {}.{} -> {}.{}", edge.source_node, edge.source_port, edge.target_node, edge.target_port)),
                        suggestion: Some(format!("可用输出端口: {:?}", ports)),
                    });
                }
            }

            // 验证目标节点
            if !operator_ids.contains(&edge.target_node) {
                self.errors.push(ValidationError {
                    error_type: ErrorType::UndefinedReference,
                    message: format!("@edge 目标节点不存在: {}", edge.target_node),
                    location: Some(format!("@edge: ... -> {}.{}", edge.target_node, edge.target_port)),
                    suggestion: Some(format!("可用节点: {:?}", operator_ids.iter().take(5).collect::<Vec<_>>())),
                });
            } else if let Some(ports) = op_inputs.get(&edge.target_node) {
                if !ports.contains(&edge.target_port) {
                    self.errors.push(ValidationError {
                        error_type: ErrorType::UndefinedReference,
                        message: format!("@edge 目标端口不存在: {}.{}", edge.target_node, edge.target_port),
                        location: Some(format!("@edge: ... -> {}.{}", edge.target_node, edge.target_port)),
                        suggestion: Some(format!("可用输入端口: {:?}", ports)),
                    });
                }
            }
        }

        // 检查孤立节点（使用显式边模式时）
        let mut connected: HashSet<String> = HashSet::new();
        for edge in &module.edges {
            if !edge.source_node.starts_with('@') { connected.insert(edge.source_node.clone()); }
            connected.insert(edge.target_node.clone());
        }
        for bc in &module.broadcasts {
            for (node, _) in &bc.targets { connected.insert(node.clone()); }
        }
        // 加上工作流输出/累加器/状态变量引用的节点
        for wo in &module.workflow_outputs {
            if let Some(ref n) = wo.source_name { connected.insert(n.clone()); }
        }
        for acc in &module.control_flow.accumulators { connected.insert(acc.source_node.clone()); }
        for sv in &module.control_flow.state_vars { connected.insert(sv.source_node.clone()); }

        for op in &module.operators {
            let nid = op.id.split('.').next_back().unwrap_or(&op.id).to_string();
            if !connected.contains(&nid) {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::OrphanOperator,
                    message: format!("算子 {} 在显式边模式下没有任何连接", nid),
                    location: Some(op.id.clone()),
                    suggestion: Some("添加 @edge 连接此算子或将其移除".to_string()),
                });
            }
        }
    }

    /// 验证广播定义
    fn validate_broadcasts(&mut self, module: &ModuleDef) {
        if module.broadcasts.is_empty() { return; }

        let operator_ids: HashSet<String> = module.operators.iter()
            .map(|op| op.id.split('.').next_back().unwrap_or(&op.id).to_string())
            .collect();

        let workflow_input_names: HashSet<String> = module.workflow_inputs.iter()
            .map(|wi| wi.name.clone()).collect();

        for bc in &module.broadcasts {
            // 检查广播输入是否存在于工作流输入中
            if !workflow_input_names.contains(&bc.input_name) {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingOptionalAnnotation,
                    message: format!("@broadcast 的输入 {} 未声明为 @workflow_input", bc.input_name),
                    location: Some(format!("@broadcast: {} -> ...", bc.input_name)),
                    suggestion: Some(format!("添加 ;; @workflow_input: {}, Type, required, description", bc.input_name)),
                });
            }

            // 检查广播目标节点是否存在
            for (target_node, _target_port) in &bc.targets {
                if !operator_ids.contains(target_node) {
                    self.errors.push(ValidationError {
                        error_type: ErrorType::UndefinedReference,
                        message: format!("@broadcast 目标节点不存在: {}", target_node),
                        location: Some(format!("@broadcast: {} -> {}.{}", bc.input_name, target_node, _target_port)),
                        suggestion: Some(format!("可用节点: {:?}", operator_ids.iter().take(5).collect::<Vec<_>>())),
                    });
                }
            }
        }
    }

    /// 验证分布定义
    fn validate_distributions(&mut self, module: &ModuleDef) {
        if module.distributions.is_empty() { return; }

        let workflow_input_names: HashSet<String> = module.workflow_inputs.iter()
            .map(|wi| wi.name.clone()).collect();

        let valid_dist_types = ["normal", "lognormal", "uniform", "truncated_normal", "triangular", "beta", "gamma", "fixed"];

        for dist in &module.distributions {
            // 检查参数名是否存在于工作流输入
            if !workflow_input_names.contains(&dist.param_name) {
                self.warnings.push(ValidationWarning {
                    warning_type: WarningType::MissingOptionalAnnotation,
                    message: format!("@distribution 参数 {} 未声明为 @workflow_input", dist.param_name),
                    location: Some(format!("@distribution: {} ...", dist.param_name)),
                    suggestion: None,
                });
            }

            // 检查分布类型
            if !valid_dist_types.contains(&dist.dist_type.as_str()) {
                self.errors.push(ValidationError {
                    error_type: ErrorType::InvalidAnnotationFormat,
                    message: format!("@distribution 未知分布类型: {}", dist.dist_type),
                    location: Some(format!("@distribution: {} ...", dist.param_name)),
                    suggestion: Some(format!("支持的类型: {:?}", valid_dist_types)),
                });
            }

            // 检查必要参数
            match dist.dist_type.as_str() {
                "normal" | "lognormal" | "truncated_normal" => {
                    if !dist.params.contains_key("mean") || !dist.params.contains_key("std") {
                        self.errors.push(ValidationError {
                            error_type: ErrorType::InvalidAnnotationFormat,
                            message: format!("@distribution {} 分布缺少 mean/std 参数", dist.dist_type),
                            location: Some(format!("@distribution: {} ...", dist.param_name)),
                            suggestion: Some("添加 mean=值, std=值".to_string()),
                        });
                    }
                }
                "uniform" => {
                    if !dist.params.contains_key("min") || !dist.params.contains_key("max") {
                        self.errors.push(ValidationError {
                            error_type: ErrorType::InvalidAnnotationFormat,
                            message: "均匀分布缺少 min/max 参数".to_string(),
                            location: Some(format!("@distribution: {} ...", dist.param_name)),
                            suggestion: Some("添加 min=值, max=值".to_string()),
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for SExprValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 格式化验证结果为可读字符串
pub fn format_validation_result(result: &ValidationResult) -> String {
    let mut output = String::new();

    output.push_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    output.push_str("║                    S-Expression 验证报告                             ║\n");
    output.push_str("╚══════════════════════════════════════════════════════════════════════╝\n\n");

    // 统计信息
    output.push_str("📊 统计信息\n");
    output.push_str("───────────────────────────────────────────────────────────────────────\n");
    output.push_str(&format!("   算子数量:       {}\n", result.stats.operator_count));
    output.push_str(&format!("   输入参数数:     {}\n", result.stats.input_count));
    output.push_str(&format!("   输出参数数:     {}\n", result.stats.output_count));
    output.push_str(&format!("   工作流输入数:   {}\n", result.stats.workflow_input_count));
    output.push_str(&format!("   工作流输出数:   {}\n", result.stats.workflow_output_count));
    output.push_str(&format!("   状态变量数:     {}\n", result.stats.state_var_count));
    output.push_str(&format!("   累加器数:       {}\n", result.stats.accumulator_count));
    output.push('\n');

    // 错误
    if !result.errors.is_empty() {
        output.push_str(&format!("❌ 错误 ({} 个)\n", result.errors.len()));
        output.push_str("───────────────────────────────────────────────────────────────────────\n");
        for (i, err) in result.errors.iter().enumerate() {
            output.push_str(&format!("   {}. [{:?}] {}\n", i + 1, err.error_type, err.message));
            if let Some(ref loc) = err.location {
                output.push_str(&format!("      位置: {}\n", loc));
            }
            if let Some(ref sug) = err.suggestion {
                output.push_str(&format!("      建议: {}\n", sug));
            }
        }
        output.push('\n');
    }

    // 警告
    if !result.warnings.is_empty() {
        output.push_str(&format!("⚠️  警告 ({} 个)\n", result.warnings.len()));
        output.push_str("───────────────────────────────────────────────────────────────────────\n");
        for (i, warn) in result.warnings.iter().enumerate() {
            output.push_str(&format!("   {}. [{:?}] {}\n", i + 1, warn.warning_type, warn.message));
            if let Some(ref loc) = warn.location {
                output.push_str(&format!("      位置: {}\n", loc));
            }
            if let Some(ref sug) = warn.suggestion {
                output.push_str(&format!("      建议: {}\n", sug));
            }
        }
        output.push('\n');
    }

    // 结果
    output.push_str("───────────────────────────────────────────────────────────────────────\n");
    if result.is_valid {
        output.push_str("✅ 验证通过\n");
    } else {
        output.push_str("❌ 验证失败\n");
    }

    output
}

/// 生成 S-expression 书写规范文档
pub fn generate_spec_doc() -> String {
    r#"# S-Expression 书写规范

## 1. 文件结构

一个 `.sexpr` 文件包含以下部分：

```
;; ============================================================================
;; 模块声明
;; ============================================================================
;; @module: module.id
;; @name: 模块名称
;; @description: 模块描述

;; ============================================================================
;; 控制流配置（可选，用于迭代执行模式）
;; ============================================================================
;; @execution_mode: iterative
;; @time_series: T
;; @accumulator: y from delta_cp init 0 op add
;; @state_var: S from state_adjustment.S init 1.0 lag 1
;; @termination: exhaust | accumulator y >= 40

;; ============================================================================
;; 工作流输入输出定义
;; ============================================================================
;; @workflow_input: name, Type, required|optional, description[, default][, bind node.port]
;; @workflow_output: name, Type, description, accumulator name | node node.port

;; ============================================================================
;; 算子定义
;; ============================================================================
;; @operator: module.operator_id
;; @name: 算子名称
;; @category: 分类
;; @description: 描述
;; @latex: $公式$
;; @input: name, Type, required|optional, description
;; @input_latex: LaTeX符号
;; @input_paper_ref: 论文引用
;; @output: name, Type, description
;; @output_latex: LaTeX符号
(expression)
```

## 2. 注解格式

### 2.1 模块注解

| 注解 | 必填 | 格式 | 说明 |
|------|------|------|------|
| `@module` | ✅ | `@module: id.path` | 模块唯一标识符 |
| `@name` | ⚪ | `@name: 名称` | 模块显示名称 |
| `@description` | ⚪ | `@description: 描述` | 模块描述 |

### 2.2 算子注解

| 注解 | 必填 | 格式 | 说明 |
|------|------|------|------|
| `@operator` | ✅ | `@operator: module.id` | 算子唯一标识符 |
| `@name` | ⚪ | `@name: 名称` | 算子显示名称 |
| `@category` | ⚪ | `@category: 分类` | 算子分类 |
| `@description` | ⚪ | `@description: 描述` | 算子描述 |
| `@latex` | ⚪ | `@latex: $公式$` | LaTeX 公式 |
| `@input` | ⚪ | `@input: name, Type, required, desc` | 输入参数 |
| `@output` | ✅ | `@output: name, Type, desc` | 输出参数 |

### 2.3 输入注解格式

```
;; @input: 参数名, 类型, required|optional, 描述
;; @input_latex: LaTeX符号（可选）
;; @input_paper_ref: 论文引用（可选）
```

**类型选项**：
- `Number` - 数值
- `Boolean` - 布尔值
- `String` - 字符串
- `Array<Number>` - 数值数组
- `Object` - 对象

### 2.4 控制流注解（迭代模式）

```
;; @execution_mode: iterative
;; @time_series: T
;; @accumulator: y from delta_cp init 0 op add
;; @state_var: S from state_adjustment.S init 1.0 lag 1
;; @termination: exhaust
```

**累加器格式**：
```
@accumulator: 变量名 from 来源节点.端口 init 初始值 op 操作符
```

**状态变量格式**：
```
@state_var: 变量名 from 来源节点.端口 init 初始值 lag 滞后步数
```

**终止条件格式**：
```
@termination: exhaust                    # 耗尽时间序列
@termination: accumulator y >= 40        # 累加器达到阈值
@termination: iterations 100             # 固定迭代次数
```

### 2.5 工作流输入格式

```
;; @workflow_input: name, Type, required|optional, description[, default][, bind node.port]
```

示例：
```
;; @workflow_input: T, Array<Number>, required, 温度序列
;; @workflow_input: yc, Number, optional, 冷量阈值, 40
;; @workflow_input: Tb, Number, optional, 基础温度, 4, bind gdh.Tb
```

### 2.6 工作流输出格式

```
;; @workflow_output: name, Type, description, accumulator name
;; @workflow_output: name, Type, description, node node.port
```

示例：
```
;; @workflow_output: y_final, Number, 最终冷量, accumulator y
;; @workflow_output: result, Number, 计算结果, node add.result
```

## 3. 表达式语法

### 3.1 基础运算

```lisp
(add a b)       ; 加法
(sub a b)       ; 减法
(mul a b)       ; 乘法
(div a b)       ; 除法
(neg a)         ; 取负
(pow a b)       ; 幂运算
```

### 3.2 数学函数

```lisp
(exp x)         ; 指数
(log x)         ; 自然对数
(sqrt x)        ; 平方根
(sin x)         ; 正弦
(cos x)         ; 余弦
(tan x)         ; 正切
(abs x)         ; 绝对值
```

### 3.3 比较运算

```lisp
(gt a b)        ; a > b
(lt a b)        ; a < b
(geq a b)       ; a >= b
(leq a b)       ; a <= b
(eq a b)        ; a == b
```

### 3.4 条件表达式

```lisp
(if condition then_expr else_expr)
```

### 3.5 分段函数

```lisp
(piecewise
  ((condition1) expr1)
  ((condition2) expr2)
  :otherwise default_expr)
```

## 4. 示例

```lisp
;; @module: math.basic
;; @name: 基础数学运算
;; @description: 提供基础数学运算算子

;; @operator: math.add
;; @name: 加法
;; @category: 算术
;; @description: 计算两数之和
;; @latex: $result = a + b$
;; @input: a, Number, required, 第一个加数
;; @input_latex: a
;; @input: b, Number, required, 第二个加数
;; @input_latex: b
;; @output: result, Number, 计算结果
;; @output_latex: result
(add a b)
```

## 5. 最佳实践

1. **使用有意义的标识符**：算子ID应反映其功能
2. **添加完整的注解**：包括 description、latex、paper_ref
3. **显式指定绑定**：工作流输入应指定 `bind node.port`
4. **验证后再生成**：使用 `eqc validate-sexpr` 验证
5. **保持一致的格式**：使用统一的缩进和注释风格
"#.to_string()
}
