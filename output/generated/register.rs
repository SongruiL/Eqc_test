//! 自动生成的算子注册代码
//!
//! 由 equation-compiler 自动生成，请勿手动修改。

use crate::lowcode::registry::OperatorRegistry;
use super::phenoflex_chill_operators::*;

/// 注册所有自动生成的算子
///
/// 在 `build_operator_registry` 中调用此函数注册算子。
pub fn register_generated_operators(registry: &mut OperatorRegistry) {
    register_phenoflex_chill_operators(registry);
}

/// 注册 PhenoFlex冷量模型 模块的算子
pub fn register_phenoflex_chill_operators(registry: &mut OperatorRegistry) {
    registry.register(TempKelvinOperator);
    registry.register(ArrheniusOperator);
    registry.register(ChillPortionOperator);
}

