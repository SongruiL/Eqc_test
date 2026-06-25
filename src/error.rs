//! 错误类型定义

use std::path::PathBuf;
use thiserror::Error;

/// 编译结果类型别名
pub type CompileResult<T> = Result<T, CompileError>;

/// 编译器错误类型
#[derive(Error, Debug)]
pub enum CompileError {
    /// IO 错误
    #[error("IO 错误: {path} - {message}")]
    Io {
        path: PathBuf,
        message: String,
        #[source]
        source: std::io::Error,
    },

    /// YAML 解析错误
    #[error("YAML 解析错误: {path} - {message}")]
    YamlParse {
        path: PathBuf,
        message: String,
    },

    /// 表达式解析错误
    #[error("表达式解析错误: {context} - {message}")]
    ExpressionParse { context: String, message: String },

    /// 验证错误
    #[error("验证错误: {0}")]
    Validation(#[from] ValidationError),

    /// 多个验证错误
    #[error("发现 {} 个验证错误", .0.len())]
    MultipleValidationErrors(Vec<ValidationError>),

    /// 循环依赖
    #[error("检测到循环依赖: {cycle:?}")]
    CyclicDependency { cycle: Vec<String> },

    /// 代码生成错误
    #[error("代码生成错误: {generator} - {message}")]
    CodeGeneration { generator: String, message: String },

    /// 未知错误
    #[error("未知错误: {0}")]
    Unknown(String),
}

/// 验证错误类型
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    /// 未定义的引用
    #[error("未定义的引用: {kind} '{name}' 在 {location}")]
    UndefinedReference {
        kind: String, // "变量" 或 "参数"
        name: String,
        location: String,
    },

    /// 类型不匹配
    #[error("类型不匹配: {expected} vs {actual} 在 {location}")]
    TypeMismatch {
        expected: String,
        actual: String,
        location: String,
    },

    /// 缺少输出方程
    #[error("输出变量 '{variable}' 没有对应的方程定义")]
    MissingOutputEquation { variable: String },

    /// 重复定义
    #[error("重复定义: {kind} '{name}'")]
    DuplicateDefinition { kind: String, name: String },

    /// 无效的源引用
    #[error("无效的源引用: '{0}' - 期望格式 'MODULE.variable'")]
    InvalidSourceReference(String),

    /// 循环依赖
    #[error("循环依赖: {path}")]
    CyclicDependency { path: String },

    /// 结构过定：跨模块多条方程解算同一个变量（耦合折叠后撞在同一节点）。
    /// 单文件内的重复 output 由 DuplicateDefinition 报告；这里专报**跨模块**的系统级过定。
    #[error("结构过定: 变量 '{variable}' 被多条方程解算（{equations}）")]
    StructurallyOverDetermined { variable: String, equations: String },

    /// 类型错误
    #[error("类型错误: {message} 在 {location}")]
    TypeError { message: String, location: String },
}

impl CompileError {
    /// 创建 IO 错误
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        let path = path.into();
        Self::Io {
            message: source.to_string(),
            path,
            source,
        }
    }

    /// 创建 YAML 解析错误
    pub fn yaml_parse(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::YamlParse {
            path: path.into(),
            message: message.into(),
        }
    }

    /// 创建表达式解析错误
    pub fn expr_parse(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExpressionParse {
            context: context.into(),
            message: message.into(),
        }
    }

    /// 创建代码生成错误
    pub fn codegen(generator: impl Into<String>, message: impl Into<String>) -> Self {
        Self::CodeGeneration {
            generator: generator.into(),
            message: message.into(),
        }
    }
}
