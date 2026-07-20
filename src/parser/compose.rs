//! E3 组合 pass（arc §4.1 步①）。
//!
//! 把**基座** + 选中的**模块 overlay** 合成一个扁平模型，交给下游展开/求解——是温室全保真
//! arc 的模型组合/配置化（SSOT）机制：可选设备（补光/保温幕/遮荫/加热管/侧窗…）=
//! 可加载的 overlay 模块，不加载 = 该设备不存在，**不做减法坍缩**（arc §2 P2）。
//!
//! **必须先于 structure/cohort 展开与破环**（arc §4.1 关键相邻约束）：模块 overlay 可携带
//! 自己的 cohort（如多路管），须先合并再统一展开；故 compose 在 `serde_yaml::Value` 层、
//! 在反序列化/展开**之前**运行。
//!
//! **Phase 1 = identity-compose 骨架**：只做 base-only 恒等直通（逐位不变、零运行时行为），
//! 立起 `compose→展开→(E5b/E2)→solve` 管线骨架 + Phase 2 模块插槽；真正的
//! append / override-by-id / `meta.balance` 重生成 / 悬挂校验机器留 Phase 2（见函数体 TODO）。

use serde_yaml::Value;

/// 组合错误。
///
/// **Phase 1 为空枚举**（identity-compose 不可能失败·不可实例化）。Phase 2 填充：
/// 悬挂引用（G6）、override 目标缺失、重复 id（G1）等。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    // TODO(Phase2): DanglingRef { module: String, name: String },   // G6 悬挂符号
    // TODO(Phase2): OverrideMissingTarget { id: String },           // override 指向不存在的 id
    // TODO(Phase2): DuplicateId(String),                            // G1 无隐式重复 id
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 空枚举不可达（Phase 1 无实例）。Phase 2 补各 variant 的人话消息。
        match *self {}
    }
}

impl std::error::Error for ComposeError {}

/// 一个模块 overlay（可加载设备的模型片段）。
///
/// **Phase 1 为占位空结构**（YAGNI：Phase 2 才明确模块 YAML 形态——独立 `.overlay.yaml`？
/// 内嵌段？override 指令语法？——现在填字段大概率返工）。Phase 1 调用点恒传空切片 `&[]`。
/// Phase 2 填充：追加的 parameters/variables/equations/balance 片段 + override-by-id 指令。
pub struct ModuleOverlay {
    // TODO(Phase2): 模块片段内容
}

/// **E3 步①：组合** base 与模块 overlay → 合成模型（`serde_yaml::Value` 层）。
///
/// **Phase 1（identity-compose）**：`overlays` 为空 → **原样返回 base**（move·不遍历·不 clone·
/// 逐位不变·零运行时行为）。非空 overlay 是 Phase 2 才实现的路径。
///
/// 位置：在 `structure_expand`/`cohort_expand` 之**前**（模块 overlay 可带自己的 cohort，
/// 须合并后统一展开·arc §4.1 关键相邻约束）。
pub fn compose(base: Value, overlays: &[ModuleOverlay]) -> Result<Value, ComposeError> {
    if overlays.is_empty() {
        // ── identity 直通：base-only，原样返回（零运行时行为的最强形式：无任何机器）──
        return Ok(base);
    }

    // ── Phase 2 插槽（arc §4.1 步① 顺序·H2/G1/G6 门禁）──
    //   1) append:         overlay 的 parameters/variables/equations/balance 追加进 base 对应 mapping（保序）
    //   2) override-by-id:  按 equation `id` 覆盖 base 同 id 方程（须带 `override:` 标记·就地替换不改序·G1 放行）
    //   3) meta.balance 重生成: 合并 base + overlay 的 BalanceLaw（否则 V3 --check-balance 查陈旧律）
    //   4) 悬挂校验:        append/override 项引用的符号须在合成后存在，否则 ComposeError::DanglingRef（G6）
    unreachable!("Phase 1 identity-compose：非空 overlay 属 Phase 2（当前所有调用点恒传 &[]）")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一段代表性 YAML（含 meta.balance + cohorts + 参数/变量/方程），供 identity 逐位核验。
    const SAMPLE: &str = r#"
meta:
  id: TEST_COMPOSE
  model: TestCompose
  version: "t1"
  balance:
    - { name: 能量, stock: T, sources: [q_in], sinks: [q_out], cap: capT, tol: 1.0e-6 }
cohorts:
  layer: { size: 3, index: i }
parameters:
  k: { name_cn: 系数, default: 0.5, unit: "-" }
variables:
  T: { type: output, class: state, init: 20.0, rate: rate_T, unit: degC }
  rate_T: { class: rate, unit: "K/s" }
equations:
  - { id: E-RATE, output: rate_T, expression: { op: mul, args: [ { ref: k }, { ref: T } ] } }
"#;

    /// ★核心验收：base 经 identity-compose（空 overlay）后 `serde_yaml::Value` 逐位不变。
    #[test]
    fn compose_identity_bit_identical() {
        let base: Value = serde_yaml::from_str(SAMPLE).unwrap();
        let before = base.clone();
        let after = compose(base, &[]).expect("identity-compose 不应失败");
        assert_eq!(before, after, "identity-compose 必须逐位保持 base 不变");
    }

    /// identity 不动 meta.balance / cohorts / 顶层声明顺序（serde_yaml Mapping 保序）。
    #[test]
    fn compose_identity_preserves_balance_and_order() {
        let base: Value = serde_yaml::from_str(SAMPLE).unwrap();
        let after = compose(base.clone(), &[]).unwrap();
        assert_eq!(base["meta"]["balance"], after["meta"]["balance"], "meta.balance 须逐位不变");
        assert_eq!(base["cohorts"], after["cohorts"], "cohorts 段须逐位不变");
        let keys = |v: &Value| {
            v.as_mapping()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(&base), keys(&after), "顶层 key 顺序须保持");
    }

    /// 端到端：base-only YAML 经完整 `parse_str` 管线（含新插入的 compose 步）解析成功、
    /// 结构符合预期——显式锁「compose 插入点生效且对 parse 管线透明」（对抗复审第 5 点建议）。
    #[test]
    fn compose_transparent_in_parse_str_pipeline() {
        let yaml = r#"
meta: { id: E2E_COMPOSE, model: E2eCompose, name_cn: 端到端 }
parameters:
  k: { name_cn: 系数, default: 0.5, unit: "-" }
variables:
  T: { type: output, class: state, init: 20.0, rate: rate_T, unit: degC }
  rate_T: { class: rate, unit: "K/s" }
equations:
  - { id: E-RATE, name: 速率, output: rate_T, expression: { op: mul, args: [ { ref: k }, { ref: T } ] } }
"#;
        let file = crate::parser::parse_str(yaml).expect("parse_str（含 compose 步）应解析成功");
        assert_eq!(file.meta.id, "E2E_COMPOSE");
        assert_eq!(file.equations.len(), 1, "方程数经 compose 透明管线不变");
        assert!(file.variables.contains_key("T"), "状态 T 应保留");
        assert!(file.parameters.contains_key("k"), "参数 k 应保留");
    }
}
