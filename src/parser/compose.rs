//! E3 组合 pass（arc §4.1 步①）。
//!
//! 把**基座** + 选中的**模块 overlay** 合成一个扁平模型，交给下游展开/求解——是温室全保真
//! arc 的模型组合/配置化（SSOT）机制：可选设备（补光/保温幕/遮荫/加热管/侧窗…）=
//! 可加载的 overlay 模块，不加载 = 该设备不存在，**不做减法坍缩**（arc §2 P2）。
//!
//! **必须先于 structure/cohort 展开与破环**（arc §4.1 关键相邻约束）：模块 overlay 可携带
//! 自己的 cohort（如多路管），须先合并再统一展开；故 compose 在 `serde_yaml::Value` 层、
//! 在反序列化/展开**之前**运行（candidate A）。
//!
//! **Phase 2（本档 = 非空 compose 机器·施工 spec §6）**：
//! - **① append**：overlay 的 parameters/variables/cohorts/equations/balance 追加进 base（保序·G1 无重复）。
//! - **② inject + rate 重生成**：overlay 的 `inject: [{stock, source|sink}]` 把设备通量回注进**已有基座态**
//!   的守恒律，并从合并后的 balance 律**重生成**这条被注入态的 rate 方程（`build_rate_expr_value`）——
//!   现役 rate 全是规范折叠形，故重生成对基座项逐位保留、只末尾挂新项（施工 spec §1③）。
//! - **③ meta.balance 重生成**：= ①（模块新态律）+ ②（追加进已有律）后的 `meta.balance`，V3 `--check-balance`
//!   读它、与重生成 rate 逐态机器零（被注入态重言式·模块新态手写 rate+独立 balance 仍真核验）。
//! - **④ 悬挂校验（G6）**：inject 引用的通量须合成后已声明，否则 `ComposeError::DanglingRef`。
//!
//! **override-by-id（拓扑覆盖/遮挡）= Phase 2 后续（施工 spec §9 / 档 1c）**，本档不实现：append_equations
//! 现只做追加+去重；1c 扩展为「带 `override: <id>` 标记则就地替换」。

use serde_yaml::{Mapping, Value};
use std::collections::HashSet;

/// 组合错误。
///
/// 本档（施工 spec §6）四变体覆盖 append/inject/dangling 门禁；`OverrideMissingTarget`（override
/// 目标缺失）留 override-by-id 落地（档 1c）时补。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    /// G1：append 的 equation `id` 或 parameter/variable 键与已有重复（无 override 标记）。
    DuplicateId { module: String, id: String },
    /// inject 的 `stock` 不是 base 的守恒律存量（找不到可回注的 balance 律）。
    InjectMissingStock { module: String, stock: String },
    /// 被注入态无 `rate:` 声明——无从重生成 rate 方程。
    InjectMissingRate { module: String, stock: String },
    /// G6：inject/append 引用的符号在合成后未定义（悬挂）。
    DanglingRef { module: String, name: String },
    /// overlay 结构/语义非法（对抗复审 R1 硬化·走 clean 错误而非 panic/静默）：守恒律字段非列表、
    /// 同一通量重复注入同一存量（防双计）、为已有存量重复声明守恒律（应改用 inject）等。
    InvalidOverlay { module: String, detail: String },
    /// override-by-id（档1c）：`override: <id>` 指向的 base 方程 id 不存在。
    OverrideMissingTarget { module: String, id: String },
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComposeError::DuplicateId { module, id } => {
                write!(f, "模块 {module} 引入了重复的 id/键 `{id}`（无 override 标记）")
            }
            ComposeError::InjectMissingStock { module, stock } => {
                write!(f, "模块 {module} 的 inject 目标存量 `{stock}` 不存在于 base 守恒律")
            }
            ComposeError::InjectMissingRate { module, stock } => {
                write!(f, "被注入态 `{stock}`（模块 {module}）无 rate: 声明，无法重生成速率方程")
            }
            ComposeError::DanglingRef { module, name } => {
                write!(f, "模块 {module} 的 inject/append 引用了未定义符号 `{name}`（悬挂）")
            }
            ComposeError::InvalidOverlay { module, detail } => {
                write!(f, "模块 {module} overlay 非法：{detail}")
            }
            ComposeError::OverrideMissingTarget { module, id } => {
                write!(f, "模块 {module} 的 override 目标方程 id `{id}` 不存在于 base")
            }
        }
    }
}

impl std::error::Error for ComposeError {}

/// 一个模块 overlay（可加载设备的模型片段）。
///
/// 持有**原始 overlay YAML**（`serde_yaml::Value`·展开前）；compose 从中读 `meta.module` /
/// parameters/variables/cohorts/equations/balance / `inject` 各段。**不**单独展开它（可能带半截
/// cohort/structure）——并进 base 后整体统一展开（candidate A·arc §4.1）。
pub struct ModuleOverlay {
    /// 原始 overlay YAML（展开前）。
    pub value: Value,
}

/// **E3 步①：组合** base 与模块 overlay → 合成模型（`serde_yaml::Value` 层）。
///
/// **base-only（`overlays` 空）**：原样返回 base（move·不遍历·不 clone·逐位不变·零运行时行为）。
/// **非空**：逐 overlay append + inject → 对被注入态重生成 rate → 悬挂校验。
///
/// 位置：在 `structure_expand`/`cohort_expand` 之**前**（模块 overlay 可带自己的 cohort，
/// 须合并后统一展开·arc §4.1 关键相邻约束）。
pub fn compose(base: Value, overlays: &[ModuleOverlay]) -> Result<Value, ComposeError> {
    if overlays.is_empty() {
        // ── identity 直通：base-only，原样返回（零运行时行为的最强形式：无任何机器）──
        return Ok(base);
    }

    let mut merged = base;
    // 被注入（守恒律改动）的态，保序去重——逐 overlay 处理完再统一重生成其 rate。
    let mut dirty: Vec<String> = Vec::new();
    for ov in overlays {
        apply_overlay(&mut merged, &ov.value, &mut dirty)?;
    }
    // ② rate 重生成：被注入态的 rate 方程从合并后的 balance 律重建。
    for stock in &dirty {
        regenerate_rate(&mut merged, stock)?;
    }
    // ④ 悬挂校验（G6）：inject 引用的通量须合成后已声明。
    dangling_check(&merged, overlays)?;
    Ok(merged)
}

/// 把一个 overlay 并进 merged：append 各段 + 处理 inject（标记 dirty）。
fn apply_overlay(merged: &mut Value, ov: &Value, dirty: &mut Vec<String>) -> Result<(), ComposeError> {
    let module = module_name(ov);

    // ① append mapping 段：parameters / variables / cohorts（candidate A：cohort 合并在展开前）。
    for section in ["parameters", "variables", "cohorts"] {
        if let Some(src) = get_section(ov, section).and_then(Value::as_mapping) {
            append_mapping(merged, section, src, &module)?;
        }
    }
    // ① append equations（含 id 去重 G1；override-by-id 标记 = 档 1c，本档不出现）。
    if let Some(eqs) = get_section(ov, "equations").and_then(Value::as_sequence) {
        append_equations(merged, eqs, &module)?;
    }
    // ① append balance（模块新态的守恒律并入 meta.balance；带 override:true 则替换已有存量的律·档A 双隔间重构）。
    if let Some(laws) = get_section(ov, "balance").and_then(Value::as_sequence) {
        append_balance(merged, laws, &module, dirty)?;
    }
    // ② inject：回注进已有基座守恒律 + 标记 dirty。
    if let Some(injs) = get_section(ov, "inject").and_then(Value::as_sequence) {
        for inj in injs {
            apply_inject(merged, inj, &module, dirty)?;
        }
    }
    Ok(())
}

/// append 一个 mapping 段（parameters/variables/cohorts）：键冲突 = G1 报 DuplicateId。
fn append_mapping(
    merged: &mut Value,
    section: &str,
    src: &Mapping,
    module: &str,
) -> Result<(), ComposeError> {
    let mm = merged.as_mapping_mut().expect("base 顶层须为 mapping");
    if !mm.contains_key(section) {
        mm.insert(vstr(section), Value::Mapping(Mapping::new()));
    }
    let dst = mm
        .get_mut(section)
        .unwrap()
        .as_mapping_mut()
        .expect("section 须为 mapping");
    for (k, v) in src {
        if dst.contains_key(k) {
            let id = k.as_str().unwrap_or("<?>").to_string();
            return Err(ComposeError::DuplicateId { module: module.to_string(), id });
        }
        dst.insert(k.clone(), v.clone());
    }
    Ok(())
}

/// append equations（尾插·id 全局唯一 G1，含同一 overlay 内自查重）+ override-by-id（档1c）。
///
/// 带 `override: <id>` 标记的方程 = **就地替换** base 同 id 方程（不改序·G1 放行·去 override 标记后
/// 落生成物）；目标 id 不存在 → `OverrideMissingTarget`（G1 修正·arc §9）。用于拓扑覆盖/遮挡
/// （如加热管遮挡改 base FIR view factor·保温幕改空气平衡为双隔间）。
fn append_equations(merged: &mut Value, eqs: &[Value], module: &str) -> Result<(), ComposeError> {
    let mut seen = collect_ids(merged);
    let mm = merged.as_mapping_mut().expect("base 顶层须为 mapping");
    if !mm.contains_key("equations") {
        mm.insert(vstr("equations"), Value::Sequence(vec![]));
    }
    let dst = mm.get_mut("equations").unwrap().as_sequence_mut().unwrap();
    for eq in eqs {
        // override-by-id：带 `override: <id>` 标记 → 就地替换 base 同 id 方程（G1 放行）。
        if let Some(target) = get_section(eq, "override").and_then(Value::as_str) {
            let pos = dst.iter().position(|e| {
                e.as_mapping().and_then(|m| m.get("id")).and_then(Value::as_str) == Some(target)
            });
            match pos {
                Some(p) => {
                    let mut replacement = eq.clone();
                    // 去 override 标记；强制 id=目标（对抗复审 R 挖出 MINOR footgun：override:X 但 id:Y
                    // 会让 base id X 静默消失·override 语义=替换目标故结果 id 恒为目标）。
                    if let Some(m) = replacement.as_mapping_mut() {
                        m.remove("override");
                        m.insert(vstr("id"), vstr(target));
                    }
                    dst[p] = replacement;
                }
                None => {
                    return Err(ComposeError::OverrideMissingTarget {
                        module: module.to_string(),
                        id: target.to_string(),
                    })
                }
            }
            continue; // 替换不改 id 集（target 仍在），不落 seen/dup 检查。
        }
        // 普通 append（id 全局唯一 G1·含同一 overlay 内自查重）。
        if let Some(id) = get_section(eq, "id").and_then(Value::as_str) {
            if seen.contains(id) {
                return Err(ComposeError::DuplicateId { module: module.to_string(), id: id.to_string() });
            }
            seen.insert(id.to_string());
        }
        dst.push(eq.clone());
    }
    Ok(())
}

/// append 模块新态的守恒律进 meta.balance；带 `override: true` 则**替换**已有存量的守恒律
/// （拓扑覆盖·arc §2.2「override 空气平衡为双隔间」）。
///
/// - 新存量的律：追加（#4 硬化：无 override 时不得为已有存量重声明·防死重复）。
/// - 已有存量 + `override: true`：就地替换该律（去标记）+ 标记 dirty → rate 从新律重生成
///   （复用 `build_rate_expr_value`·与 equation override-by-id 同构·保平衡律 SSOT）。
fn append_balance(
    merged: &mut Value,
    laws: &[Value],
    module: &str,
    dirty: &mut Vec<String>,
) -> Result<(), ComposeError> {
    let mut stocks = collect_balance_stocks(merged);
    let mm = merged.as_mapping_mut().expect("base 顶层须为 mapping");
    if !mm.contains_key("meta") {
        mm.insert(vstr("meta"), Value::Mapping(Mapping::new()));
    }
    let meta = mm.get_mut("meta").unwrap().as_mapping_mut().expect("meta 须为 mapping");
    if !meta.contains_key("balance") {
        meta.insert(vstr("balance"), Value::Sequence(vec![]));
    }
    let seq = meta.get_mut("balance").unwrap().as_sequence_mut().unwrap();
    for law in laws {
        let stock = law.as_mapping().and_then(|m| m.get("stock")).and_then(Value::as_str).map(String::from);
        let is_override = law.as_mapping().and_then(|m| m.get("override")).and_then(Value::as_bool).unwrap_or(false);
        match stock {
            Some(stock) if stocks.contains(&stock) => {
                if !is_override {
                    return Err(ComposeError::InvalidOverlay {
                        module: module.to_string(),
                        detail: format!("为已有存量 `{stock}` 重复声明守恒律（应改用 inject 回注·或加 override: true 显式替换）"),
                    });
                }
                // 平衡律 override：就地替换该 stock 的律（去 override 标记）+ dirty 触发 rate 重生成。
                // position 必命中（stocks.contains ⟺ seq 有该律·不变量）；None=内部不变量违反 → 硬报错，
                // 而非静默跳过替换（防未来 refactor 破不变量时从陈旧律重生成·对抗复审建议）。
                let p = seq
                    .iter()
                    .position(|l| l.as_mapping().and_then(|m| m.get("stock")).and_then(Value::as_str) == Some(stock.as_str()))
                    .ok_or_else(|| ComposeError::InvalidOverlay {
                        module: module.to_string(),
                        detail: format!("内部不变量违反：存量 `{stock}` 在集合但无守恒律可替换"),
                    })?;
                let mut replacement = law.clone();
                if let Some(m) = replacement.as_mapping_mut() {
                    m.remove("override");
                }
                seq[p] = replacement;
                if !dirty.iter().any(|d| d == &stock) {
                    dirty.push(stock);
                }
            }
            Some(stock) => {
                if is_override {
                    return Err(ComposeError::InvalidOverlay {
                        module: module.to_string(),
                        detail: format!("balance override 的存量 `{stock}` 无已有守恒律可替换"),
                    });
                }
                stocks.insert(stock);
                seq.push(law.clone());
            }
            None => seq.push(law.clone()),
        }
    }
    Ok(())
}

/// inject 一项：把设备通量追加进 base 中 `stock` 的守恒律（source/sink）+ 标记 dirty。
fn apply_inject(
    merged: &mut Value,
    inj: &Value,
    module: &str,
    dirty: &mut Vec<String>,
) -> Result<(), ComposeError> {
    let stock = get_section(inj, "stock")
        .and_then(Value::as_str)
        .ok_or_else(|| ComposeError::DanglingRef {
            module: module.to_string(),
            name: "inject 缺 stock".to_string(),
        })?
        .to_string();
    let (name, is_source) = if let Some(s) = get_section(inj, "source").and_then(Value::as_str) {
        (s.to_string(), true)
    } else if let Some(s) = get_section(inj, "sink").and_then(Value::as_str) {
        (s.to_string(), false)
    } else {
        return Err(ComposeError::DanglingRef {
            module: module.to_string(),
            name: format!("inject(stock={stock}) 需 source 或 sink"),
        });
    };

    // 先算律的下标（不可变借用），再可变改——避免交错借用。
    let idx = balance_law_index(merged, &stock).ok_or_else(|| ComposeError::InjectMissingStock {
        module: module.to_string(),
        stock: stock.clone(),
    })?;
    let law = merged
        .as_mapping_mut()
        .unwrap()
        .get_mut("meta")
        .unwrap()
        .as_mapping_mut()
        .unwrap()
        .get_mut("balance")
        .unwrap()
        .as_sequence_mut()
        .unwrap()
        .get_mut(idx)
        .unwrap();
    let field = if is_source { "sources" } else { "sinks" };
    let lawm = law.as_mapping_mut().ok_or_else(|| ComposeError::InvalidOverlay {
        module: module.to_string(),
        detail: format!("stock={stock} 的守恒律不是 mapping"),
    })?;
    if !lawm.contains_key(field) {
        lawm.insert(vstr(field), Value::Sequence(vec![]));
    }
    // #1 硬化：字段须为列表（overlay 把 sources/sinks 误写成标量时给 clean 错误而非 panic）。
    let seq = lawm
        .get_mut(field)
        .unwrap()
        .as_sequence_mut()
        .ok_or_else(|| ComposeError::InvalidOverlay {
            module: module.to_string(),
            detail: format!("stock={stock} 守恒律的 {field} 不是列表（应为 [ ... ]）"),
        })?;
    // #2 硬化：拒绝把同一通量重复注入同一存量（否则规范折叠里静默双计）。
    if seq.iter().any(|x| x.as_str() == Some(name.as_str())) {
        return Err(ComposeError::InvalidOverlay {
            module: module.to_string(),
            detail: format!("通量 `{name}` 重复注入 stock={stock}（会双计）"),
        });
    }
    seq.push(vstr(&name));

    if !dirty.iter().any(|d| d == &stock) {
        dirty.push(stock);
    }
    Ok(())
}

/// 对被注入态从合并后的 balance 律重生成 rate 方程（整条替换 expression）。
fn regenerate_rate(merged: &mut Value, stock: &str) -> Result<(), ComposeError> {
    // 1) rate 变量名（权威来自 variables[stock].rate·非命名约定）。
    let rate_var = get_section(merged, "variables")
        .and_then(|v| v.as_mapping())
        .and_then(|vars| vars.get(stock))
        .and_then(|sv| sv.as_mapping())
        .and_then(|sm| sm.get("rate"))
        .and_then(Value::as_str)
        .ok_or_else(|| ComposeError::InjectMissingRate {
            module: "<compose>".to_string(),
            stock: stock.to_string(),
        })?
        .to_string();

    // 2) 合并后的 balance 律 → 源/汇/cap。
    let idx = balance_law_index(merged, stock).ok_or_else(|| ComposeError::InjectMissingStock {
        module: "<compose>".to_string(),
        stock: stock.to_string(),
    })?;
    let (sources, sinks, cap) = extract_law_fields(merged, idx);

    // 3) 生成 rate 表达式 Value（规范折叠形·复现现役手写 RATE-*）。
    let expr = build_rate_expr_value(&sources, &sinks, cap.as_deref());

    // 4) 替换 output == rate_var 的方程的 expression。
    let eqs = merged
        .as_mapping_mut()
        .unwrap()
        .get_mut("equations")
        .and_then(|v| v.as_sequence_mut())
        .ok_or_else(|| ComposeError::DanglingRef {
            module: "<compose>".to_string(),
            name: format!("缺 equations 段（重生成 {rate_var}）"),
        })?;
    for eq in eqs.iter_mut() {
        let is_target = eq
            .as_mapping()
            .and_then(|m| m.get("output"))
            .and_then(Value::as_str)
            == Some(rate_var.as_str());
        if is_target {
            eq.as_mapping_mut().unwrap().insert(vstr("expression"), expr);
            return Ok(());
        }
    }
    Err(ComposeError::DanglingRef {
        module: "<compose>".to_string(),
        name: format!("被注入态 {stock} 的 rate 方程 output={rate_var} 未找到"),
    })
}

/// **★核心 helper**：从守恒律 (sources, sinks, cap) 折出 rate 表达式 `(Σsources − Σsinks)/cap`。
///
/// 规范折叠形（复现现役所有手写 RATE-* 的确切树·施工 spec §1③/§6.2 逐位保真铁律）：
/// - 源：`add` 左结合（`[]`→无·`[s]`→`{ref:s}`·`[s1,s2,..]`→`add(add(s1,s2),..)`）。
/// - 汇：`sub` 左结合；**空源边角**（源为空、≥1 汇）→ 首汇 `neg`（复现 RATE-CO2AIR）。
/// - `/cap`（有 cap 时 `div`，无则裸 num）。
fn build_rate_expr_value(sources: &[String], sinks: &[String], cap: Option<&str>) -> Value {
    let mut num: Option<Value> = None;
    for s in sources {
        let r = vref(s);
        num = Some(match num.take() {
            None => r,
            Some(acc) => vop("add", vec![acc, r]),
        });
    }
    for s in sinks {
        let r = vref(s);
        num = Some(match num.take() {
            None => vop("neg", vec![r]), // 空源边角：首汇取负
            Some(acc) => vop("sub", vec![acc, r]),
        });
    }
    let num = num.unwrap_or_else(|| vconst(0.0)); // 源汇皆空（真实存量不应发生）
    match cap {
        Some(c) => vop("div", vec![num, vref(c)]),
        None => num,
    }
}

/// 悬挂校验（G6）：每个 overlay 的 inject 引用的通量须在合成后已声明（variables/parameters）。
fn dangling_check(merged: &Value, overlays: &[ModuleOverlay]) -> Result<(), ComposeError> {
    let declared = collect_declared(merged);
    for ov in overlays {
        let module = module_name(&ov.value);
        if let Some(injs) = get_section(&ov.value, "inject").and_then(Value::as_sequence) {
            for inj in injs {
                let name = get_section(inj, "source")
                    .and_then(Value::as_str)
                    .or_else(|| get_section(inj, "sink").and_then(Value::as_str));
                if let Some(name) = name {
                    if !declared.contains(name) {
                        return Err(ComposeError::DanglingRef {
                            module: module.clone(),
                            name: name.to_string(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

// ── Value 层小工具 ────────────────────────────────────────────────────────

/// 取 `v`（须 mapping）的某段。
fn get_section<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_mapping().and_then(|m| m.get(key))
}

/// overlay 的模块名（`meta.module`），供错误消息。
fn module_name(ov: &Value) -> String {
    get_section(ov, "meta")
        .and_then(|m| m.as_mapping())
        .and_then(|m| m.get("module"))
        .and_then(Value::as_str)
        .unwrap_or("<未命名模块>")
        .to_string()
}

/// meta.balance 里 `stock==stock` 的律下标。
fn balance_law_index(merged: &Value, stock: &str) -> Option<usize> {
    get_section(merged, "meta")
        .and_then(|m| m.as_mapping())
        .and_then(|m| m.get("balance"))
        .and_then(Value::as_sequence)?
        .iter()
        .position(|law| law.as_mapping().and_then(|m| m.get("stock")).and_then(Value::as_str) == Some(stock))
}

/// 取某 balance 律的 (sources, sinks, cap)。
fn extract_law_fields(merged: &Value, idx: usize) -> (Vec<String>, Vec<String>, Option<String>) {
    let law = &merged["meta"]["balance"][idx];
    let list = |key: &str| -> Vec<String> {
        law.as_mapping()
            .and_then(|m| m.get(key))
            .and_then(Value::as_sequence)
            .map(|s| s.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    let cap = law
        .as_mapping()
        .and_then(|m| m.get("cap"))
        .and_then(Value::as_str)
        .map(String::from);
    (list("sources"), list("sinks"), cap)
}

/// 合成后已声明的符号集（variables + parameters 键 + 方程 output）。
///
/// #3 硬化：一个符号「已定义」= 有 variable/parameter 声明**或**是某方程的 output——只查前者会把
/// 「仅由 equations 定义、未在 variables 声明」的通量误判为悬挂（false-positive）。
fn collect_declared(merged: &Value) -> HashSet<String> {
    let mut set = HashSet::new();
    for section in ["variables", "parameters"] {
        if let Some(m) = get_section(merged, section).and_then(Value::as_mapping) {
            for k in m.keys() {
                if let Some(s) = k.as_str() {
                    set.insert(s.to_string());
                }
            }
        }
    }
    if let Some(seq) = get_section(merged, "equations").and_then(Value::as_sequence) {
        for eq in seq {
            if let Some(out) = eq.as_mapping().and_then(|m| m.get("output")).and_then(Value::as_str) {
                set.insert(out.to_string());
            }
        }
    }
    set
}

/// meta.balance 里所有 stock 名（append_balance 的 #4 重声明守恒律去重用）。
fn collect_balance_stocks(merged: &Value) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Some(seq) = get_section(merged, "meta")
        .and_then(|m| m.as_mapping())
        .and_then(|m| m.get("balance"))
        .and_then(Value::as_sequence)
    {
        for law in seq {
            if let Some(stock) = law.as_mapping().and_then(|m| m.get("stock")).and_then(Value::as_str) {
                set.insert(stock.to_string());
            }
        }
    }
    set
}

/// 已有 equation id 集。
fn collect_ids(merged: &Value) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Some(seq) = get_section(merged, "equations").and_then(Value::as_sequence) {
        for eq in seq {
            if let Some(id) = eq.as_mapping().and_then(|m| m.get("id")).and_then(Value::as_str) {
                set.insert(id.to_string());
            }
        }
    }
    set
}

fn vstr(s: &str) -> Value {
    Value::String(s.to_string())
}
fn vref(name: &str) -> Value {
    let mut m = Mapping::new();
    m.insert(vstr("ref"), vstr(name));
    Value::Mapping(m)
}
fn vconst(x: f64) -> Value {
    let mut m = Mapping::new();
    m.insert(vstr("const"), Value::Number(serde_yaml::Number::from(x)));
    Value::Mapping(m)
}
fn vop(op: &str, args: Vec<Value>) -> Value {
    let mut m = Mapping::new();
    m.insert(vstr("op"), vstr(op));
    m.insert(vstr("args"), Value::Sequence(args));
    Value::Mapping(m)
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
        let keys = |v: &Value| v.as_mapping().unwrap().keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys(&base), keys(&after), "顶层 key 顺序须保持");
    }

    /// 端到端：base-only YAML 经完整 `parse_str` 管线（含新插入的 compose 步）解析成功。
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

    // ── §6.6 非空 compose 机器单元测（toy overlay·引擎独立验证）──────────────

    /// 代表性 toy base（含 T 守恒律 + 规范折叠 RATE-T）。
    const TOY_BASE: &str = r#"
meta:
  id: TOY
  model: Toy
  version: "t1"
  balance:
    - { name: E, stock: T, sources: [q_in], sinks: [q_out], cap: capT, tol: 1.0e-6 }
variables:
  T:      { type: output, class: state, init: 20.0, rate: rate_T, unit: degC }
  rate_T: { class: rate, unit: "K/s" }
  q_in:   { class: auxiliary, unit: "W/m2" }
  q_out:  { class: auxiliary, unit: "W/m2" }
  capT:   { class: auxiliary, unit: "J/m2/K" }
parameters:
  k: { name_cn: 系数, default: 0.5, unit: "-" }
equations:
  - { id: E-QIN,  output: q_in,  expression: { ref: k } }
  - { id: E-QOUT, output: q_out, expression: { const: 1.0 } }
  - { id: E-CAPT, output: capT,  expression: { const: 100.0 } }
  - { id: RATE-T, output: rate_T, expression: { op: div, args: [ { op: sub, args: [ { ref: q_in }, { ref: q_out } ] }, { ref: capT } ] } }
"#;

    fn toy_base() -> Value {
        serde_yaml::from_str(TOY_BASE).unwrap()
    }
    fn overlay(yaml: &str) -> ModuleOverlay {
        ModuleOverlay { value: serde_yaml::from_str(yaml).unwrap() }
    }

    /// ★§6.6.1 逐位保真：build_rate_expr_value 复现现役规范折叠形（三种形状 + 空源 neg 边角）。
    #[test]
    fn build_rate_expr_reproduces_canonical_fold() {
        // 多源多汇（RATE-AIR 形）：((s1+s2) − k1 − k2 − k3)/cap
        let got = build_rate_expr_value(
            &["s1".into(), "s2".into()],
            &["k1".into(), "k2".into(), "k3".into()],
            Some("cap"),
        );
        let want: Value = serde_yaml::from_str(
            "{ op: div, args: [ { op: sub, args: [ { op: sub, args: [ { op: sub, args: [ { op: add, args: [ { ref: s1 }, { ref: s2 } ] }, { ref: k1 } ] }, { ref: k2 } ] }, { ref: k3 } ] }, { ref: cap } ] }",
        ).unwrap();
        assert_eq!(got, want, "多源多汇折叠须逐位匹配手写 RATE-AIR 形");

        // 单源（RATE-VPAIR 形）：(s − k1 − k2)/cap，单源无 add 包裹
        let got = build_rate_expr_value(&["s".into()], &["k1".into(), "k2".into()], Some("cap"));
        let want: Value = serde_yaml::from_str(
            "{ op: div, args: [ { op: sub, args: [ { op: sub, args: [ { ref: s }, { ref: k1 } ] }, { ref: k2 } ] }, { ref: cap } ] }",
        ).unwrap();
        assert_eq!(got, want, "单源折叠须无 add 包裹");

        // ★空源边角（RATE-CO2AIR 形）：(−k)/cap = div(neg(k), cap)
        let got = build_rate_expr_value(&[], &["k".into()], Some("cap"));
        let want: Value =
            serde_yaml::from_str("{ op: div, args: [ { op: neg, args: [ { ref: k } ] }, { ref: cap } ] }").unwrap();
        assert_eq!(got, want, "空源单汇须走 neg（复现 RATE-CO2AIR）");
    }

    /// §6.6.2 append：toy overlay 加参数/变量/方程/新态守恒律 → 合并且 base 原项不变。
    #[test]
    fn compose_appends_overlay_sections() {
        let ov = overlay(
            r#"
meta: { module: toy_dev }
parameters:
  p2: { name_cn: 新参, default: 1.0, unit: "-" }
variables:
  D:      { type: output, class: state, init: 5.0, rate: rate_D, unit: degC }
  rate_D: { class: rate }
  q_dev:  { class: auxiliary }
  capD:   { class: auxiliary }
equations:
  - { id: DEV-QDEV, output: q_dev, expression: { ref: k } }
  - { id: DEV-CAPD, output: capD, expression: { const: 50.0 } }
  - { id: DEV-RATE, output: rate_D, expression: { op: div, args: [ { op: neg, args: [ { ref: q_dev } ] }, { ref: capD } ] } }
balance:
  - { name: Edev, stock: D, sources: [], sinks: [q_dev], cap: capD, tol: 1.0e-6 }
"#,
        );
        let merged = compose(toy_base(), &[ov]).expect("append 应成功");
        // 新增在场
        assert!(merged["variables"].as_mapping().unwrap().contains_key("D"));
        assert!(merged["parameters"].as_mapping().unwrap().contains_key("p2"));
        // base 原项不变
        assert!(merged["variables"].as_mapping().unwrap().contains_key("T"));
        assert_eq!(merged["equations"].as_sequence().unwrap().len(), 4 + 3, "方程 = base4 + overlay3");
        // 新态守恒律并入
        assert_eq!(merged["meta"]["balance"].as_sequence().unwrap().len(), 2);
    }

    /// ★§6.6.3 inject + 重生成：注入源项 → T 律 sources 追加 + RATE-T 从合并律重生成。
    #[test]
    fn compose_inject_regenerates_rate() {
        let ov = overlay(
            r#"
meta: { module: toy_heater }
variables:
  q_dev: { class: auxiliary }
equations:
  - { id: DEV-QDEV, output: q_dev, expression: { ref: k } }
inject:
  - { stock: T, source: q_dev }
"#,
        );
        let merged = compose(toy_base(), &[ov]).expect("inject 应成功");
        // T 律 sources 末尾追加 q_dev
        let law = &merged["meta"]["balance"][0];
        let srcs: Vec<&str> = law["sources"].as_sequence().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(srcs, vec!["q_in", "q_dev"], "sources 末尾追加注入项");
        // RATE-T 重生成 = ((q_in + q_dev) − q_out)/capT（base 项 q_in 仍最左·新项挂上）
        let rate_eq = merged["equations"].as_sequence().unwrap().iter()
            .find(|e| e["output"].as_str() == Some("rate_T")).unwrap();
        let want: Value = serde_yaml::from_str(
            "{ op: div, args: [ { op: sub, args: [ { op: add, args: [ { ref: q_in }, { ref: q_dev } ] }, { ref: q_out } ] }, { ref: capT } ] }",
        ).unwrap();
        assert_eq!(rate_eq["expression"], want, "RATE-T 须从合并律重生成");
    }

    /// ★§6.6.4 悬挂负例：inject 未定义通量 / 不存在的 stock / 重复 id 各自报对错。
    #[test]
    fn compose_negative_cases() {
        // 悬挂：inject 引用未声明 undefined_flux
        let ov = overlay(
            "meta: { module: bad1 }\ninject:\n  - { stock: T, source: undefined_flux }\n",
        );
        assert_eq!(
            compose(toy_base(), &[ov]).unwrap_err(),
            ComposeError::DanglingRef { module: "bad1".into(), name: "undefined_flux".into() }
        );

        // inject 到不存在的 stock
        let ov = overlay(
            "meta: { module: bad2 }\nvariables:\n  q_dev: { class: auxiliary }\nequations:\n  - { id: X, output: q_dev, expression: { ref: k } }\ninject:\n  - { stock: NoSuchStock, source: q_dev }\n",
        );
        assert_eq!(
            compose(toy_base(), &[ov]).unwrap_err(),
            ComposeError::InjectMissingStock { module: "bad2".into(), stock: "NoSuchStock".into() }
        );

        // 重复 id（与 base RATE-T 撞）
        let ov = overlay(
            "meta: { module: bad3 }\nequations:\n  - { id: RATE-T, output: zzz, expression: { const: 0.0 } }\n",
        );
        assert_eq!(
            compose(toy_base(), &[ov]).unwrap_err(),
            ComposeError::DuplicateId { module: "bad3".into(), id: "RATE-T".into() }
        );
    }

    /// ★对抗复审硬化（R1）：malformed overlay 走 clean 错误而非 panic/静默。
    #[test]
    fn compose_hardening_guards() {
        // #4：为已有存量 T 重声明守恒律 → InvalidOverlay（而非留死重复律扰 §8）
        let ov = overlay(
            "meta: { module: dup_law }\nvariables:\n  x: { class: auxiliary }\nequations:\n  - { id: XL, output: x, expression: { const: 1.0 } }\nbalance:\n  - { name: dup, stock: T, sources: [x], sinks: [], cap: capT, tol: 1.0e-6 }\n",
        );
        assert!(
            matches!(compose(toy_base(), &[ov]).unwrap_err(), ComposeError::InvalidOverlay { .. }),
            "为已有存量重声明守恒律须报 InvalidOverlay"
        );

        // #2：同一通量重复注入同一存量 → InvalidOverlay（防静默双计）
        let ov = overlay(
            "meta: { module: dbl }\nvariables:\n  q_dev: { class: auxiliary }\nequations:\n  - { id: XD, output: q_dev, expression: { ref: k } }\ninject:\n  - { stock: T, source: q_dev }\n  - { stock: T, source: q_dev }\n",
        );
        assert!(
            matches!(compose(toy_base(), &[ov]).unwrap_err(), ComposeError::InvalidOverlay { .. }),
            "重复注入同一通量须报 InvalidOverlay"
        );

        // #1：注入目标守恒律字段写成标量（非列表）→ InvalidOverlay 而非 panic
        let ov = overlay(
            "meta: { module: shape }\nvariables:\n  D: { type: output, class: state, init: 0.0, rate: rate_D }\n  rate_D: { class: rate }\n  q_dev: { class: auxiliary }\n  capD: { class: auxiliary }\nequations:\n  - { id: XQ, output: q_dev, expression: { ref: k } }\n  - { id: XC, output: capD, expression: { const: 1.0 } }\n  - { id: XR, output: rate_D, expression: { const: 0.0 } }\nbalance:\n  - { name: bad, stock: D, sources: q_dev, sinks: [], cap: capD, tol: 1.0e-6 }\ninject:\n  - { stock: D, source: q_dev }\n",
        );
        assert!(
            matches!(compose(toy_base(), &[ov]).unwrap_err(), ComposeError::InvalidOverlay { .. }),
            "守恒律字段非列表须报 InvalidOverlay 而非 panic"
        );
    }

    /// ★override-by-id（档1c）：带 `override: <id>` 就地替换 base 同 id 方程·不改序·去标记·目标缺失报错。
    #[test]
    fn compose_override_by_id() {
        let n_base = toy_base()["equations"].as_sequence().unwrap().len();
        // 就地替换 base RATE-T 的表达式（override 目标 = RATE-T）
        let ov = overlay(
            "meta: { module: ovr }\nequations:\n  - { id: RATE-T, override: RATE-T, output: rate_T, expression: { const: 0.0 } }\n",
        );
        let merged = compose(toy_base(), &[ov]).expect("override 应成功");
        let eqs = merged["equations"].as_sequence().unwrap();
        assert_eq!(eqs.len(), n_base, "override 是替换非追加·方程数不变");
        let rate_t = eqs.iter().find(|e| e["id"].as_str() == Some("RATE-T")).unwrap();
        assert_eq!(rate_t["expression"], serde_yaml::from_str::<Value>("{ const: 0.0 }").unwrap(), "RATE-T 表达式已就地替换");
        assert!(rate_t.as_mapping().unwrap().get("override").is_none(), "生成物须去 override 标记");

        // ★强制 id=目标：override:RATE-T 但 id:DECOY → 结果保留 RATE-T（防 base id 静默消失·对抗复审硬化）
        let ov = overlay(
            "meta: { module: ovrid }\nequations:\n  - { id: DECOY, override: RATE-T, output: rate_T, expression: { const: 1.0 } }\n",
        );
        let merged = compose(toy_base(), &[ov]).unwrap();
        let ids: Vec<&str> = merged["equations"].as_sequence().unwrap().iter().filter_map(|e| e["id"].as_str()).collect();
        assert!(ids.contains(&"RATE-T") && !ids.contains(&"DECOY"), "override 结果 id 须强制为目标 RATE-T（非 DECOY）");

        // 目标 id 不存在 → OverrideMissingTarget
        let ov = overlay(
            "meta: { module: ovrbad }\nequations:\n  - { override: NO-SUCH-ID, output: zzz, expression: { const: 0.0 } }\n",
        );
        assert_eq!(
            compose(toy_base(), &[ov]).unwrap_err(),
            ComposeError::OverrideMissingTarget { module: "ovrbad".into(), id: "NO-SUCH-ID".into() }
        );
    }

    /// ★平衡律 override（thermal_screen 双隔间重构）：`override: true` 替换已有存量的律 + rate 从新律重生成。
    #[test]
    fn compose_balance_override() {
        // 替换 base T 律（sources [q_in]→[q_in,q_new]·sinks [q_out]→[q_out2]）+ 重生成 RATE-T
        let ov = overlay(
            "meta: { module: bov }\nvariables:\n  q_new: { class: auxiliary }\n  q_out2: { class: auxiliary }\nequations:\n  - { id: E-QNEW, output: q_new, expression: { ref: k } }\n  - { id: E-QOUT2, output: q_out2, expression: { const: 2.0 } }\nbalance:\n  - { name: E2, stock: T, sources: [q_in, q_new], sinks: [q_out2], cap: capT, tol: 1.0e-6, override: true }\n",
        );
        let merged = compose(toy_base(), &[ov]).expect("balance override 应成功");
        let law = &merged["meta"]["balance"][0];
        let names = |k: &str| law[k].as_sequence().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(names("sources"), vec!["q_in", "q_new"], "T 律 sources 被替换");
        assert_eq!(names("sinks"), vec!["q_out2"], "T 律 sinks 被替换");
        assert!(law.as_mapping().unwrap().get("override").is_none(), "生成物去 override 标记");
        // RATE-T 从新律重生成 = ((q_in+q_new)−q_out2)/capT
        let rate = merged["equations"].as_sequence().unwrap().iter().find(|e| e["output"].as_str() == Some("rate_T")).unwrap();
        let want: Value = serde_yaml::from_str(
            "{ op: div, args: [ { op: sub, args: [ { op: add, args: [ { ref: q_in }, { ref: q_new } ] }, { ref: q_out2 } ] }, { ref: capT } ] }",
        ).unwrap();
        assert_eq!(rate["expression"], want, "RATE-T 须从 override 律重生成");

        // 无 override 重声明已有存量 → InvalidOverlay（#4 保留）
        let ov = overlay("meta: { module: bad }\nbalance:\n  - { name: dup, stock: T, sources: [q_in], sinks: [], cap: capT, tol: 1.0e-6 }\n");
        assert!(matches!(compose(toy_base(), &[ov]).unwrap_err(), ComposeError::InvalidOverlay { .. }));

        // override 不存在的存量 → InvalidOverlay
        let ov = overlay("meta: { module: bad2 }\nbalance:\n  - { name: x, stock: NoStock, sources: [q_in], sinks: [], cap: capT, tol: 1.0e-6, override: true }\n");
        assert!(matches!(compose(toy_base(), &[ov]).unwrap_err(), ComposeError::InvalidOverlay { .. }));
    }

    /// ★§6.6.5 candidate A：overlay 自带 cohort → compose 合并进 base，再 expand_cohorts 正确展开。
    #[test]
    fn compose_merges_cohort_before_expand() {
        let ov = overlay(
            r#"
meta: { module: multipipe }
cohorts: { pipe: { size: 2, index: j } }
variables:
  tp:      { cohort: pipe, type: output, class: state, init: 0.0, rate: rate_tp }
  rate_tp: { cohort: pipe, class: rate }
equations:
  - { id: PIPE-TP, cohort: pipe, output: rate_tp, expression: { const: 0.0 } }
"#,
        );
        let merged = compose(toy_base(), &[ov]).expect("cohort overlay compose 应成功");
        // compose 后 cohorts.pipe 在场（展开前）
        assert!(merged["cohorts"].as_mapping().unwrap().contains_key("pipe"), "cohort 须合并进 base（candidate A）");
        // 再过 cohort 展开 → tp__1 / tp__2
        let expanded = crate::parser::expand_cohorts(merged).expect("展开应成功");
        let vars = expanded["variables"].as_mapping().unwrap();
        assert!(vars.contains_key("tp__1") && vars.contains_key("tp__2"), "cohort 应展成 __1/__2");
    }

    /// §6.6.6 base-only 恒等（非空机器建成后仍零回归）——已由前两个 identity 测覆盖，此处补 toy。
    #[test]
    fn compose_toy_base_only_identity() {
        let base = toy_base();
        assert_eq!(compose(base.clone(), &[]).unwrap(), base, "toy base-only 逐位不变");
    }
}
