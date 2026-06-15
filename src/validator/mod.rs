//! 验证器模块
//!
//! 检查方程定义的正确性：
//! - 引用完整性（reference_checker）
//! - 类型兼容性（type_checker）
//! - 循环依赖检测（cycle_detector）

mod cycle_detector;
mod reference_checker;
mod type_checker;

pub use crate::error::ValidationError;
pub use type_checker::{check_types, infer_type, ExprType};

use crate::error::{CompileError, CompileResult};
use crate::schema::EquationFile;

/// 验证方程文件
///
/// 执行所有验证检查，返回第一个错误或成功。
///
/// 检查顺序：
/// 1. 引用完整性检查
/// 2. 跨模块引用检查
/// 3. 类型检查
/// 4. 循环依赖检测
pub fn validate(files: &[EquationFile]) -> CompileResult<()> {
    let mut all_errors = Vec::new();

    // 1. 引用检查
    for file in files {
        let errors = reference_checker::check_references(file);
        all_errors.extend(errors);
    }

    // 2. 跨模块引用检查
    let cross_errors = reference_checker::check_cross_module_references(files);
    all_errors.extend(cross_errors);

    // 3. 类型检查
    for file in files {
        let errors = type_checker::check_types(file);
        all_errors.extend(errors);
    }

    // 4. 循环依赖检测
    if let Some(cycle) = cycle_detector::detect_cycles(files) {
        all_errors.push(ValidationError::CyclicDependency {
            path: cycle.join(" -> "),
        });
    }

    // 返回结果
    if all_errors.is_empty() {
        Ok(())
    } else if all_errors.len() == 1 {
        Err(CompileError::Validation(all_errors.remove(0)))
    } else {
        Err(CompileError::MultipleValidationErrors(all_errors))
    }
}

/// 仅验证单个文件（不检查跨模块引用）
pub fn validate_file(file: &EquationFile) -> CompileResult<()> {
    let mut errors = reference_checker::check_references(file);
    errors.extend(type_checker::check_types(file));

    if errors.is_empty() {
        Ok(())
    } else if errors.len() == 1 {
        Err(CompileError::Validation(errors.into_iter().next().unwrap()))
    } else {
        Err(CompileError::MultipleValidationErrors(errors))
    }
}
