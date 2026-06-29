//! FSPM `structure:` 段实例化（地基 1c，见 `docs/spec-fspm-foundation.md`）。
//!
//! 与 cohort 同属**加载期**处理（反序列化前对 `serde_yaml::Value` 重写）：把声明的器官结构
//! 实例化成**带身份标签的标量** —— 引擎层照跑标量、不读身份；下游读 `instance:`/`structure:`。
//!
//! # 语法（本期实现 count / chain / per；borne_on/tree/clonal 后续）
//! ```yaml
//! structure:
//!   entities:
//!     metamer: { count: 4, topology: chain }   # 4 节，逐节 succession
//!     fruit:   { per: metamer, count: 2 }       # 每节 2 果（contains 边）
//! variables:
//!   leaf_area: { of: metamer, class: state, init: 0, rate: leaf_growth }
//!   fruit_mass:{ of: fruit, class: state, init: 0, rate: fruit_growth }
//!   assimilate:{ class: state }                 # 无 of: = 整株共享
//! equations:
//!   - { for: fruit, output: fruit_growth,
//!       expression: { op: mul, args: [ {ref: sink, of: self}, {ref: assimilate} ] } }
//! ```
//! 实例化：`of:E` 变量每实例一份（名 `base__<id>`，id 里 `.`→`_`）；`for:E` 方程每实例一份，
//! ref 的 `of: self|parent|prev|next` 解析到对应实例（self 默认；prev/next=chain 邻居，越界→const 0；
//! parent=per 的父实例）；无 `of:` 且非结构量的 ref = 整株共享，原样。

use serde_yaml::{Mapping, Value};
use std::collections::HashMap;

/// 结构实例化错误。
#[derive(Debug, Clone, PartialEq)]
pub enum StructureError {
    /// 顶层不是 mapping。
    NotMapping,
    /// 实体声明不合法（缺 count、或 per 缺 count 等）。
    BadEntity(String),
    /// 引用了未声明的父实体。
    UnknownParent { entity: String, parent: String },
    /// ref 的 `of:` 取值非法（非 self/parent/prev/next）。
    BadRefOf(String),
    /// 必需字段缺失。
    MissingField { ctx: String, field: String },
    /// 聚合声明不合法（缺 over/body、over 取值非法、children 歧义等）。
    BadAggregate(String),
    /// 聚合对空集（mean/min/max 基数 0）—— 加载期拒绝（不设运行时 0/NaN）。
    EmptyAggregate { kind: String, over: String, entity: String },
}

impl std::fmt::Display for StructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructureError::NotMapping => write!(f, "顶层 YAML 不是 mapping"),
            StructureError::BadEntity(n) => write!(f, "结构实体 {n} 声明不合法（需 count，或 per+count）"),
            StructureError::UnknownParent { entity, parent } => {
                write!(f, "实体 {entity} 的父实体 {parent} 未声明")
            }
            StructureError::BadRefOf(s) => write!(f, "ref 的 of: 取值非法: {s}（需 self/parent/prev/next）"),
            StructureError::MissingField { ctx, field } => write!(f, "{ctx} 缺少必需字段 {field}"),
            StructureError::BadAggregate(s) => write!(f, "拓扑聚合声明不合法: {s}"),
            StructureError::EmptyAggregate { kind, over, entity } => {
                write!(f, "{kind} 聚合（over: {over}, 实体 {entity}）的集合为空（基数 0）——请检查实体基数，{kind} 对空集未定义")
            }
        }
    }
}
impl std::error::Error for StructureError {}

/// 一个实体声明。
struct EntityDef {
    name: String,
    /// 顶层绝对实例数（根实体）。
    count: Option<usize>,
    /// 父实体名（per-parent 基数）。
    per: Option<String>,
    /// 每父实例的子实例数。
    per_count: Option<usize>,
    /// 拓扑种类（本期 "chain" 或 None）。
    topology: Option<String>,
}

/// 一个实例（本地 id + 父实例本地 id）。
#[derive(Clone)]
struct Inst {
    id: String,
    parent: Option<String>,
}

/// id 安全化为变量名后缀：`.` → `_`（果 `3.2` → `3_2`）。
fn id_safe(id: &str) -> String {
    id.replace('.', "_")
}

/// 聚合出处（FSPM 风险3·可见性）：某方程输出沿拓扑邻域聚合而来。emit 进 `structure.aggregations`。
/// 聚合在加载期已 lower 成标量 add 链；此处保留语义供分析/前端显示「Σ over children / mean over all」。
struct AggProv {
    output: String,
    kind: String,
    over: String,
    entity: Option<String>,
}

/// 扫描表达式 `Value`，采集其中所有 `{agg: K, over: S, of?: E}` 节点（递归，含 body 内嵌套）。
fn collect_aggs(expr: &Value, output: &str, out: &mut Vec<AggProv>) {
    match expr {
        Value::Mapping(m) => {
            if let Some(kind) = m.get("agg").and_then(Value::as_str) {
                out.push(AggProv {
                    output: output.to_string(),
                    kind: kind.to_string(),
                    over: m.get("over").and_then(Value::as_str).unwrap_or("").to_string(),
                    entity: m.get("of").and_then(Value::as_str).map(|s| s.to_string()),
                });
            }
            for (_, v) in m {
                collect_aggs(v, output, out);
            }
        }
        Value::Sequence(s) => {
            for e in s {
                collect_aggs(e, output, out);
            }
        }
        _ => {}
    }
}

/// 对整份 `.eq.yaml` 的 `Value` 做结构实例化。无 `structure:` 段时原样返回。
pub fn expand_structure(mut root: Value) -> Result<Value, StructureError> {
    let map = root.as_mapping_mut().ok_or(StructureError::NotMapping)?;
    if map.get("structure").and_then(Value::as_mapping).is_none() {
        return Ok(root);
    }

    // 1) 解析实体（按依赖排序：父先于子）。
    let entities = parse_entities(map)?;
    // 2) 计算各实体实例。
    let insts = compute_instances(&entities)?;
    // 3) 收集每个变量的所属实体（of:），用于方程 ref 解析。
    let var_entity = collect_var_entities(map);

    // 4) 实例化变量。
    if let Some(Value::Mapping(vars)) = map.get("variables") {
        let new_vars = expand_vars(vars, &insts, &var_entity)?;
        map.insert(Value::from("variables"), Value::Mapping(new_vars));
    }
    // 5) 实例化方程（同时采集聚合出处，供契约/前端「聚合可见性」）。
    let mut aggs: Vec<AggProv> = Vec::new();
    if let Some(Value::Sequence(eqs)) = map.get("equations") {
        let new_eqs = expand_eqs(eqs, &entities, &insts, &var_entity, &mut aggs)?;
        map.insert(Value::from("equations"), Value::Sequence(new_eqs));
    }
    // 6) 用实例化后的 StructureInfo（含聚合出处）替换声明。
    map.insert(Value::from("structure"), build_structure_info(&entities, &insts, &aggs));

    Ok(root)
}

/// 解析 `structure.entities`，按依赖排序（无 per 的根实体在前，per 子实体在后）。
fn parse_entities(map: &Mapping) -> Result<Vec<EntityDef>, StructureError> {
    let s = map.get("structure").and_then(Value::as_mapping).unwrap();
    let ents = s.get("entities").and_then(Value::as_mapping).ok_or(StructureError::MissingField {
        ctx: "structure".into(),
        field: "entities".into(),
    })?;
    let mut defs = Vec::new();
    for (k, v) in ents {
        let name = k.as_str().unwrap_or_default().to_string();
        let d = v.as_mapping().ok_or_else(|| StructureError::BadEntity(name.clone()))?;
        let count = d.get("count").and_then(Value::as_u64).map(|n| n as usize);
        let per = d.get("per").and_then(Value::as_str).map(|s| s.to_string());
        let topology = d.get("topology").and_then(Value::as_str).map(|s| s.to_string());
        let per_count = count; // per 实体的 count = 每父子数
        if per.is_some() && per_count.is_none() {
            return Err(StructureError::BadEntity(name.clone()));
        }
        if per.is_none() && count.is_none() {
            return Err(StructureError::BadEntity(name.clone()));
        }
        let is_per = per.is_some();
        defs.push(EntityDef {
            name,
            count: if is_per { None } else { count },
            per,
            per_count: if is_per { per_count } else { None },
            topology,
        });
    }
    // 排序：根实体（无 per）在前，再按 per 依赖把子实体排在父之后。
    let mut ordered: Vec<EntityDef> = Vec::new();
    let mut remaining = defs;
    while !remaining.is_empty() {
        let before = remaining.len();
        let mut next = Vec::new();
        for d in remaining.into_iter() {
            let ready = match &d.per {
                None => true,
                Some(p) => ordered.iter().any(|e| &e.name == p),
            };
            if ready {
                ordered.push(d);
            } else {
                next.push(d);
            }
        }
        remaining = next;
        if remaining.len() == before {
            // 有未声明的父或循环依赖
            let bad = remaining.remove(0);
            let parent = bad.per.clone().unwrap_or_default();
            return Err(StructureError::UnknownParent { entity: bad.name, parent });
        }
    }
    Ok(ordered)
}

/// 计算各实体的实例列表（本地 id + 父本地 id）。entities 已父先于子排序。
fn compute_instances(entities: &[EntityDef]) -> Result<HashMap<String, Vec<Inst>>, StructureError> {
    let mut out: HashMap<String, Vec<Inst>> = HashMap::new();
    for d in entities {
        let mut list = Vec::new();
        match (&d.per, d.count, d.per_count) {
            // 根实体：1..=count
            (None, Some(n), _) => {
                for i in 1..=n {
                    list.push(Inst { id: i.to_string(), parent: None });
                }
            }
            // per 实体：每个父实例 → per_count 个子（id = "父id.k"）
            (Some(p), _, Some(m)) => {
                let parents = out.get(p).ok_or_else(|| StructureError::UnknownParent {
                    entity: d.name.clone(),
                    parent: p.clone(),
                })?;
                for pinst in parents {
                    for k in 1..=m {
                        list.push(Inst { id: format!("{}.{}", pinst.id, k), parent: Some(pinst.id.clone()) });
                    }
                }
            }
            _ => return Err(StructureError::BadEntity(d.name.clone())),
        }
        out.insert(d.name.clone(), list);
    }
    Ok(out)
}

/// 收集 `of: E` 的变量名 → 实体名。
fn collect_var_entities(map: &Mapping) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(Value::Mapping(vars)) = map.get("variables") {
        for (k, v) in vars {
            if let (Some(name), Some(ent)) =
                (k.as_str(), v.as_mapping().and_then(|d| d.get("of")).and_then(Value::as_str))
            {
                out.insert(name.to_string(), ent.to_string());
            }
        }
    }
    out
}

/// 实例化变量：`of:E` 每实例一份（名 `base__<id_safe>` + instance 标签 + rate/prev 同实例后缀）。
fn expand_vars(
    vars: &Mapping,
    insts: &HashMap<String, Vec<Inst>>,
    var_entity: &HashMap<String, String>,
) -> Result<Mapping, StructureError> {
    let mut out = Mapping::new();
    for (k, v) in vars {
        let name = k.as_str().unwrap_or_default().to_string();
        let decl = match v.as_mapping() {
            Some(m) => m,
            None => {
                out.insert(k.clone(), v.clone());
                continue;
            }
        };
        let Some(ent) = decl.get("of").and_then(Value::as_str) else {
            out.insert(k.clone(), v.clone()); // 无 of: = 整株共享，原样
            continue;
        };
        let list = insts.get(ent).map(|v| v.as_slice()).unwrap_or(&[]);
        for inst in list {
            let mut clone = decl.clone();
            clone.remove("of");
            let sfx = id_safe(&inst.id);
            // rate/prev 指向同实体成员 → 同实例后缀
            for field in ["rate", "prev"] {
                if let Some(r) = clone.get(field).and_then(Value::as_str).map(|s| s.to_string()) {
                    if var_entity.get(&r).map(|e| e == ent).unwrap_or(false) {
                        clone.insert(Value::from(field), Value::from(format!("{r}__{sfx}")));
                    }
                }
            }
            // 友好名 label 追加实例 id
            if let Some(l) = clone.get("label").and_then(Value::as_str).map(|s| s.to_string()) {
                clone.insert(Value::from("label"), Value::from(format!("{l}[{}]", inst.id)));
            }
            clone.insert(Value::from("instance"), instance_tag(ent, &inst.id));
            out.insert(Value::from(format!("{name}__{sfx}")), Value::Mapping(clone));
        }
    }
    Ok(out)
}

/// 实例化方程：`for:E` 每实例一份（id/output 后缀 + ref of: 解析 + instance 标签）。
fn expand_eqs(
    eqs: &[Value],
    entities: &[EntityDef],
    insts: &HashMap<String, Vec<Inst>>,
    var_entity: &HashMap<String, String>,
    aggs: &mut Vec<AggProv>,
) -> Result<Vec<Value>, StructureError> {
    let entities_chain: HashMap<String, bool> =
        entities.iter().map(|e| (e.name.clone(), e.topology.as_deref() == Some("chain"))).collect();
    let mut out = Vec::new();
    for eq in eqs {
        let Some(emap) = eq.as_mapping() else {
            out.push(eq.clone());
            continue;
        };
        // 采集聚合出处（用原始 output 基名 + 表达式；每方程一次，不随实例重复）。
        if let (Some(output), Some(expr)) =
            (emap.get("output").and_then(Value::as_str), emap.get("expression"))
        {
            collect_aggs(expr, output, aggs);
        }
        match emap.get("for").and_then(Value::as_str).map(|s| s.to_string()) {
            // 整株共享方程（无 for:）：仍要 lower 表达式里的聚合（over: all）；普通共享 ref 原样。
            None => {
                let mut clone = emap.clone();
                if let Some(expr) = clone.get("expression") {
                    let ctx = RefCtx {
                        ent: "", inst: None, idx: 0, n_self: 0, is_chain: false,
                        var_entity, insts, entities_chain: &entities_chain,
                    };
                    let rewritten = rewrite_refs(expr, &ctx)?;
                    clone.insert(Value::from("expression"), rewritten);
                }
                out.push(Value::Mapping(clone));
            }
            // `for: E` 方程：每实例一份。
            Some(ent) => {
                let list = insts.get(&ent).map(|v| v.as_slice()).unwrap_or(&[]);
                let is_chain = *entities_chain.get(&ent).unwrap_or(&false);
                let n_self = list.len(); // 解析 of: prev/next 边界
                for (idx, inst) in list.iter().enumerate() {
                    let mut clone = emap.clone();
                    clone.remove("for");
                    let sfx = id_safe(&inst.id);
                    // id / output 后缀
                    if let Some(id) = clone.get("id").and_then(Value::as_str).map(|s| s.to_string()) {
                        clone.insert(Value::from("id"), Value::from(format!("{id}__{sfx}")));
                    }
                    let out_name = clone.get("output").and_then(Value::as_str).map(|s| s.to_string()).ok_or(
                        StructureError::MissingField { ctx: "equation".into(), field: "output".into() },
                    )?;
                    clone.insert(Value::from("output"), Value::from(format!("{out_name}__{sfx}")));
                    // 表达式 ref / 聚合 解析
                    if let Some(expr) = clone.get("expression") {
                        let ctx = RefCtx {
                            ent: &ent, inst: Some(inst), idx, n_self, is_chain,
                            var_entity, insts, entities_chain: &entities_chain,
                        };
                        let rewritten = rewrite_refs(expr, &ctx)?;
                        clone.insert(Value::from("expression"), rewritten);
                    }
                    clone.insert(Value::from("instance"), instance_tag(&ent, &inst.id));
                    out.push(Value::Mapping(clone));
                }
            }
        }
    }
    Ok(out)
}

/// ref 解析上下文（当前方程实例所在实体/实例 + 邻居/父查找所需）。
/// `inst=None` = 整株共享方程（无 `for:`）：此时只能解析共享 ref 与 `over: all` 聚合，
/// 引用结构量或 `over: children`/`of: self` 会报错（无当前实例上下文）。
struct RefCtx<'a> {
    ent: &'a str,
    inst: Option<&'a Inst>,
    idx: usize,
    n_self: usize,
    is_chain: bool,
    var_entity: &'a HashMap<String, String>,
    insts: &'a HashMap<String, Vec<Inst>>,
    /// 实体名 → 是否 chain 拓扑（聚合 lower 时给目标实体建 sub-ctx 用）。
    entities_chain: &'a HashMap<String, bool>,
}

/// 递归重写表达式里的 `{ref: X [, of: self|parent|prev|next]}`：解析到对应实例的标量名。
fn rewrite_refs(v: &Value, ctx: &RefCtx) -> Result<Value, StructureError> {
    if let Value::Sequence(s) = v {
        let mut out = Vec::with_capacity(s.len());
        for e in s {
            out.push(rewrite_refs(e, ctx)?);
        }
        return Ok(Value::Sequence(out));
    }
    let Value::Mapping(m) = v else { return Ok(v.clone()) };

    // FSPM 风险3：拓扑聚合 { agg: K, over: S, of?: E, body: B } → 加载期展开成标量 add/mul 链。
    if let Some(kind) = m.get("agg").and_then(Value::as_str) {
        return expand_aggregate(m, kind, ctx);
    }

    // FSPM 风险4：实例序号 { rank: self|parent } → 加载期折成常量（实例 id 末段路径分量）。
    // self = 本实例在同胞组里的 1-based 序号（chain 链位 / per 内位）；parent = 父实例序号（上溯一层）。
    // 用于器官「错峰出现」阈值（θ = (节位−1)·phyllochron + (果位−1)·ψ）等。类比 cohort 的 {idx}。
    if let Some(which) = m.get("rank").and_then(Value::as_str) {
        let cur = ctx.inst.ok_or_else(|| {
            StructureError::BadRefOf(format!("rank: {which}（需在 for: 方程或聚合 body 内引用）"))
        })?;
        let id = match which {
            "self" => cur.id.as_str(),
            "parent" => cur.parent.as_deref().ok_or_else(|| {
                StructureError::BadRefOf(format!("rank: parent（实例 {} 无父实例）", cur.id))
            })?,
            other => return Err(StructureError::BadRefOf(format!("rank: {other}（需 self/parent）"))),
        };
        // id 末段路径分量转整数（"3.2"→2、"3"→3；compute_instances 保证各段为整数）
        let last = id.rsplit('.').next().unwrap_or(id);
        let rank: f64 =
            last.parse().map_err(|_| StructureError::BadRefOf(format!("rank: 实例 id 末段 {last} 非整数")))?;
        return Ok(const_value(rank));
    }

    if let Some(name) = m.get("ref").and_then(Value::as_str) {
        let of = m.get("of").and_then(Value::as_str);
        let var_ent = ctx.var_entity.get(name).map(|s| s.as_str());
        // 整株共享量（非任何实体的 of: 变量）→ 原样（忽略多余 of:）。
        if var_ent.is_none() {
            return Ok(ref_value(name));
        }
        // 结构量：需当前实例上下文（for: 方程或聚合 body 内）。
        let cur = ctx.inst.ok_or_else(|| {
            StructureError::BadRefOf(format!("{name}（结构量需在 for: 方程或聚合 body 内引用，整株共享方程不可直接引用单实例量）"))
        })?;
        // 按 of: 解析实例。
        let target_id: Option<String> = match of {
            None | Some("self") => Some(cur.id.clone()),
            Some("prev") => {
                if ctx.is_chain && ctx.idx >= 1 {
                    Some(ctx.insts[ctx.ent][ctx.idx - 1].id.clone())
                } else {
                    None // 越界/非链 → const 0
                }
            }
            Some("next") => {
                if ctx.is_chain && ctx.idx + 1 < ctx.n_self {
                    Some(ctx.insts[ctx.ent][ctx.idx + 1].id.clone())
                } else {
                    None
                }
            }
            Some("parent") => cur.parent.clone(),
            Some(other) => return Err(StructureError::BadRefOf(other.to_string())),
        };
        return Ok(match target_id {
            Some(id) => ref_value(&format!("{name}__{}", id_safe(&id))),
            None => const_value(0.0),
        });
    }

    // 普通结构：逐字段递归。
    let mut out = Mapping::new();
    for (k, val) in m {
        out.insert(k.clone(), rewrite_refs(val, ctx)?);
    }
    Ok(Value::Mapping(out))
}

/// FSPM 风险3：把拓扑聚合 `{ agg, over, of?, body }` lower 成标量 add/mul 链（L1，加载期）。
/// `over: children` 取当前父实例的子实例集；`over: all` 取某实体全部实例。每个目标以自身为
/// `self` 实例化 body，再折叠（sum→add 链 / prod→mul 链 / mean→链÷count / min·max→{op,args}）。
fn expand_aggregate(m: &Mapping, kind: &str, ctx: &RefCtx) -> Result<Value, StructureError> {
    let over = m
        .get("over")
        .and_then(Value::as_str)
        .ok_or(StructureError::MissingField { ctx: "agg".into(), field: "over".into() })?;
    let body = m
        .get("body")
        .ok_or(StructureError::MissingField { ctx: "agg".into(), field: "body".into() })?;
    let of_ent = m.get("of").and_then(Value::as_str);

    // 目标实体 + 被聚合的实例集
    let (target_entity, targets): (String, Vec<Inst>) = match over {
        "children" => {
            let cur = ctx.inst.ok_or_else(|| {
                StructureError::BadAggregate("over: children 需在 for: 方程内（无当前父实例）".into())
            })?;
            let pid = cur.id.as_str();
            if let Some(e) = of_ent {
                let l = ctx
                    .insts
                    .get(e)
                    .map(|v| v.iter().filter(|i| i.parent.as_deref() == Some(pid)).cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                (e.to_string(), l)
            } else {
                // 推断唯一子实体（命中多子实体 → 要求 of: 指定）
                let mut found: Option<(String, Vec<Inst>)> = None;
                for (en, l) in ctx.insts.iter() {
                    let sub: Vec<Inst> =
                        l.iter().filter(|i| i.parent.as_deref() == Some(pid)).cloned().collect();
                    if !sub.is_empty() {
                        if found.is_some() {
                            return Err(StructureError::BadAggregate(format!(
                                "over: children 命中多个子实体，请用 of: 指定（如 {en}）"
                            )));
                        }
                        found = Some((en.clone(), sub));
                    }
                }
                found.unwrap_or_else(|| (String::new(), Vec::new()))
            }
        }
        "all" => {
            let e = of_ent.ok_or(StructureError::MissingField {
                ctx: "agg over: all".into(),
                field: "of".into(),
            })?;
            (e.to_string(), ctx.insts.get(e).cloned().unwrap_or_default())
        }
        other => {
            return Err(StructureError::BadAggregate(format!("over: {other}（本轮支持 children/all）")))
        }
    };

    // 对每个目标以自身为 self 实例化 body
    let full = ctx.insts.get(&target_entity).map(|v| v.as_slice()).unwrap_or(&[]);
    let tchain = *ctx.entities_chain.get(&target_entity).unwrap_or(&false);
    let mut terms: Vec<Value> = Vec::with_capacity(targets.len());
    for t in &targets {
        let idx = full.iter().position(|i| i.id == t.id).unwrap_or(0);
        let sub = RefCtx {
            ent: &target_entity,
            inst: Some(t),
            idx,
            n_self: full.len(),
            is_chain: tchain,
            var_entity: ctx.var_entity,
            insts: ctx.insts,
            entities_chain: ctx.entities_chain,
        };
        terms.push(rewrite_refs(body, &sub)?);
    }

    fold_aggregate(kind, over, &target_entity, terms)
}

/// 折叠聚合项成标量表达式。sum 空集→0、prod 空集→1；mean/min/max 空集→加载期报错。
/// sum/prod 折叠走 `agg_fold` 单一折叠源（与 cohort sum_over/prod_over 同源、逐位一致）。
fn fold_aggregate(kind: &str, over: &str, entity: &str, terms: Vec<Value>) -> Result<Value, StructureError> {
    use super::agg_fold;
    let n = terms.len();
    let empty = || StructureError::EmptyAggregate {
        kind: kind.to_string(),
        over: over.to_string(),
        entity: entity.to_string(),
    };
    match kind {
        "sum" => Ok(agg_fold::fold_sum_or_prod(true, terms)),
        "prod" | "product" => Ok(agg_fold::fold_sum_or_prod(false, terms)),
        "mean" => {
            if n == 0 {
                return Err(empty());
            }
            Ok(agg_fold::op_args("div", vec![agg_fold::fold_sum_or_prod(true, terms), const_value(n as f64)]))
        }
        "min" | "max" => {
            if n == 0 {
                return Err(empty());
            }
            Ok(agg_fold::op_args(kind, terms))
        }
        other => Err(StructureError::BadAggregate(format!("kind: {other}"))),
    }
}

/// 构造 StructureInfo 的 YAML（entities + instances + topology）。
fn build_structure_info(entities: &[EntityDef], insts: &HashMap<String, Vec<Inst>>, aggs: &[AggProv]) -> Value {
    let (mut e_seq, mut i_seq, mut t_seq) = (Vec::new(), Vec::new(), Vec::new());
    for d in entities {
        let list = &insts[&d.name];
        let mut e = Mapping::new();
        e.insert(Value::from("name"), Value::from(d.name.as_str()));
        e.insert(Value::from("count"), Value::from(list.len() as u64));
        e.insert(Value::from("topology"), Value::from(d.topology.as_deref().unwrap_or("set")));
        e_seq.push(Value::Mapping(e));
        for inst in list {
            let mut im = Mapping::new();
            im.insert(Value::from("id"), Value::from(inst.id.as_str()));
            im.insert(Value::from("entity"), Value::from(d.name.as_str()));
            if let Some(p) = &inst.parent {
                im.insert(Value::from("parent"), Value::from(global_ref(d.per.as_deref().unwrap_or(""), p)));
                // per/borne → contains 边（父 → 子）
                t_seq.push(edge(&global_ref(d.per.as_deref().unwrap_or(""), p), &global_ref(&d.name, &inst.id), "contains"));
            }
            i_seq.push(Value::Mapping(im));
        }
        // chain → succession 边（相邻实例）
        if d.topology.as_deref() == Some("chain") {
            for w in list.windows(2) {
                t_seq.push(edge(&global_ref(&d.name, &w[0].id), &global_ref(&d.name, &w[1].id), "succession"));
            }
        }
    }
    let mut s = Mapping::new();
    s.insert(Value::from("entities"), Value::Sequence(e_seq));
    s.insert(Value::from("instances"), Value::Sequence(i_seq));
    s.insert(Value::from("topology"), Value::Sequence(t_seq));
    // 聚合出处（风险3·可见性）：非空才写，旧结构模型 structure 块逐字节不变。
    if !aggs.is_empty() {
        let mut a_seq = Vec::with_capacity(aggs.len());
        for a in aggs {
            let mut m = Mapping::new();
            m.insert(Value::from("output"), Value::from(a.output.as_str()));
            m.insert(Value::from("kind"), Value::from(a.kind.as_str()));
            m.insert(Value::from("over"), Value::from(a.over.as_str()));
            if let Some(e) = &a.entity {
                m.insert(Value::from("entity"), Value::from(e.as_str()));
            }
            a_seq.push(Value::Mapping(m));
        }
        s.insert(Value::from("aggregations"), Value::Sequence(a_seq));
    }
    Value::Mapping(s)
}

// —— 小工具 ——

/// 全局实例引用 `<entity>#<id>`（拓扑边/父引用用，跨实体不歧义）。
fn global_ref(entity: &str, id: &str) -> String {
    format!("{entity}#{id}")
}

fn instance_tag(entity: &str, id: &str) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("entity"), Value::from(entity));
    m.insert(Value::from("id"), Value::from(id));
    Value::Mapping(m)
}

fn edge(from: &str, to: &str, kind: &str) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("from"), Value::from(from));
    m.insert(Value::from("to"), Value::from(to));
    m.insert(Value::from("kind"), Value::from(kind));
    Value::Mapping(m)
}

fn ref_value(name: &str) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("ref"), Value::from(name));
    Value::Mapping(m)
}

fn const_value(x: f64) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("const"), Value::from(x));
    Value::Mapping(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(s: &str) -> Value {
        serde_yaml::from_str(s).unwrap()
    }
    fn s(m: &Mapping, k: &str) -> Option<String> {
        m.get(k).and_then(Value::as_str).map(|x| x.to_string())
    }

    #[test]
    fn test_no_structure_passthrough() {
        let src = yaml("meta: { id: M }\nvariables:\n  x: { type: input }\n");
        assert_eq!(expand_structure(src.clone()).unwrap(), src);
    }

    #[test]
    fn test_chain_per_instantiation_with_identity() {
        // metamer 链(3) + fruit per metamer(2)；方程 ref of: self/prev/parent。
        let src = yaml(
            r#"
structure:
  entities:
    metamer: { count: 3, topology: chain }
    fruit:   { per: metamer, count: 2 }
variables:
  leaf:   { of: metamer, class: state, init: 0, rate: leaf_g, label: 叶 }
  leaf_g: { of: metamer, class: rate }
  fmass:  { of: fruit, class: state, init: 0, rate: fg }
  fg:     { of: fruit, class: rate }
  assim:  { class: state }
equations:
  - { id: LG, for: metamer, output: leaf_g, expression: { op: add, args: [ {ref: leaf, of: self}, {ref: leaf, of: prev} ] } }
  - { id: FG, for: fruit,   output: fg,     expression: { op: mul, args: [ {ref: leaf, of: parent}, {ref: assim} ] } }
"#,
        );
        let out = expand_structure(src).unwrap();
        let vars = out.get("variables").unwrap().as_mapping().unwrap();

        // metamer 变量 3 份 + fruit 变量 6 份（2×3）；共享量 assim 原样
        assert!(vars.contains_key("leaf__1") && vars.contains_key("leaf__3"));
        assert!(vars.contains_key("fmass__2_1") && vars.contains_key("fmass__3_2"));
        assert!(vars.contains_key("assim") && !vars.contains_key("leaf"));
        // leaf__2：rate 同实例后缀、instance 标签、label 带 id
        let leaf2 = vars.get("leaf__2").unwrap().as_mapping().unwrap();
        assert_eq!(s(leaf2, "rate").as_deref(), Some("leaf_g__2"));
        assert_eq!(s(leaf2, "label").as_deref(), Some("叶[2]"));
        let inst = leaf2.get("instance").unwrap().as_mapping().unwrap();
        assert_eq!((s(inst, "entity").as_deref(), s(inst, "id").as_deref()), (Some("metamer"), Some("2")));
        // fmass__2_1：fruit 实例 "2.1"，rate fg__2_1
        let f21 = vars.get("fmass__2_1").unwrap().as_mapping().unwrap();
        assert_eq!(s(f21, "rate").as_deref(), Some("fg__2_1"));
        let finst = f21.get("instance").unwrap().as_mapping().unwrap();
        assert_eq!((s(finst, "entity").as_deref(), s(finst, "id").as_deref()), (Some("fruit"), Some("2.1")));

        // 方程实例化 + ref 解析
        let eqs = out.get("equations").unwrap().as_sequence().unwrap();
        let by_id = |id: &str| {
            eqs.iter().map(|e| e.as_mapping().unwrap()).find(|m| s(m, "id").as_deref() == Some(id)).unwrap().clone()
        };
        // LG__1（metamer 1）：output leaf_g__1；of:prev 越界 → const 0
        let lg1 = by_id("LG__1");
        assert_eq!(s(&lg1, "output").as_deref(), Some("leaf_g__1"));
        let lg1_args = lg1.get("expression").unwrap().as_mapping().unwrap().get("args").unwrap().as_sequence().unwrap();
        assert_eq!(s(lg1_args[0].as_mapping().unwrap(), "ref").as_deref(), Some("leaf__1")); // self
        assert!(lg1_args[1].as_mapping().unwrap().contains_key("const")); // prev 越界 → 0
        // LG__2：of:prev → leaf__1
        let lg2 = by_id("LG__2");
        let lg2_args = lg2.get("expression").unwrap().as_mapping().unwrap().get("args").unwrap().as_sequence().unwrap();
        assert_eq!(s(lg2_args[1].as_mapping().unwrap(), "ref").as_deref(), Some("leaf__1")); // prev of metamer2 = metamer1
        // FG__2_1（fruit 2.1）：of:parent → metamer 2 的 leaf__2；assim 共享原样
        let fg21 = by_id("FG__2_1");
        assert_eq!(s(&fg21, "output").as_deref(), Some("fg__2_1"));
        let fg21_args = fg21.get("expression").unwrap().as_mapping().unwrap().get("args").unwrap().as_sequence().unwrap();
        assert_eq!(s(fg21_args[0].as_mapping().unwrap(), "ref").as_deref(), Some("leaf__2")); // parent
        assert_eq!(s(fg21_args[1].as_mapping().unwrap(), "ref").as_deref(), Some("assim")); // 共享

        // structure 段：2 实体、3+6=9 实例、链 2 + contains 6 = 8 边
        let st = out.get("structure").unwrap().as_mapping().unwrap();
        assert_eq!(st.get("entities").unwrap().as_sequence().unwrap().len(), 2);
        assert_eq!(st.get("instances").unwrap().as_sequence().unwrap().len(), 9);
        let topo = st.get("topology").unwrap().as_sequence().unwrap();
        let kinds: Vec<String> = topo.iter().filter_map(|e| s(e.as_mapping().unwrap(), "kind")).collect();
        assert_eq!(kinds.iter().filter(|k| *k == "succession").count(), 2);
        assert_eq!(kinds.iter().filter(|k| *k == "contains").count(), 6);
    }

    #[test]
    fn test_rank_accessor() {
        // FSPM 风险4：{rank: self} = 实例序号、{rank: parent} = 父序号，加载期折成常量。
        let src = yaml(
            r#"
structure:
  entities:
    metamer: { count: 3, topology: chain }
    fruit:   { per: metamer, count: 2 }
variables:
  theta:  { of: fruit, class: auxiliary }
  m_rank: { of: metamer, class: auxiliary }
equations:
  - { id: TH, for: fruit, output: theta,
      expression: { op: add, args: [ {rank: parent}, {rank: self} ] } }
  - { id: MR, for: metamer, output: m_rank, expression: { rank: self } }
"#,
        );
        let out = expand_structure(src).unwrap();
        let eqs = out.get("equations").unwrap().as_sequence().unwrap();
        let by_id = |id: &str| {
            eqs.iter().map(|e| e.as_mapping().unwrap()).find(|m| s(m, "id").as_deref() == Some(id)).unwrap().clone()
        };
        let cst = |m: &Mapping| m.get("const").and_then(Value::as_f64);
        // fruit "2.1"：{rank: parent}=metamer 2、{rank: self}=果位 1
        let th21 = by_id("TH__2_1");
        let a = th21.get("expression").unwrap().as_mapping().unwrap().get("args").unwrap().as_sequence().unwrap();
        assert_eq!(cst(a[0].as_mapping().unwrap()), Some(2.0));
        assert_eq!(cst(a[1].as_mapping().unwrap()), Some(1.0));
        // fruit "3.2"：parent=3、self=2
        let th32 = by_id("TH__3_2");
        let a2 = th32.get("expression").unwrap().as_mapping().unwrap().get("args").unwrap().as_sequence().unwrap();
        assert_eq!(cst(a2[0].as_mapping().unwrap()), Some(3.0));
        assert_eq!(cst(a2[1].as_mapping().unwrap()), Some(2.0));
        // metamer "3"：整条表达式即 {rank: self} → {const: 3}
        let mr3 = by_id("MR__3");
        assert_eq!(cst(mr3.get("expression").unwrap().as_mapping().unwrap()), Some(3.0));
        // 无父实例报错（metamer 用 {rank: parent}）
        let bad = yaml(
            "structure:\n  entities:\n    metamer: { count: 2, topology: chain }\nvariables:\n  z: { of: metamer, class: auxiliary }\nequations:\n  - { id: Z, for: metamer, output: z, expression: { rank: parent } }\n",
        );
        assert!(matches!(expand_structure(bad), Err(StructureError::BadRefOf(_))));
    }

    #[test]
    fn test_organ_groups_after_full_parse() {
        // 地基风险2：结构模型经完整加载（实例化 + 反序列化）→ 图层 organ_groups 按器官折叠节点。
        let file = crate::parser::parse_str(
            r#"
meta: { id: M, model: M, name_cn: t }
structure:
  entities:
    metamer: { count: 3, topology: chain }
    fruit:   { per: metamer, count: 2 }
variables:
  leaf: { of: metamer, class: state, init: 0, rate: lg }
  lg:   { of: metamer, class: rate }
  fm:   { of: fruit, class: state, init: 0, rate: fg }
  fg:   { of: fruit, class: rate }
  T:    { type: input }
equations:
  - { id: LG, name: 叶, for: metamer, output: lg, expression: { ref: T } }
  - { id: FG, name: 果, for: fruit,   output: fg, expression: { ref: leaf, of: parent } }
"#,
        )
        .unwrap();
        let g = crate::graph::organ_groups(std::slice::from_ref(&file));
        assert_eq!(g.len(), 2);
        assert_eq!(g["metamer"].len(), 3); // 3 节
        assert_eq!(g["fruit"].len(), 6); // 2×3 果
        // metamer#2 的节点含 leaf__2 与 lg__2（同实体两变量归一实例）
        let m2: Vec<&str> = g["metamer"]["2"].iter().map(|s| s.as_str()).collect();
        assert!(m2.iter().any(|n| n.ends_with(".leaf__2")) && m2.iter().any(|n| n.ends_with(".lg__2")), "{m2:?}");
        // T（共享、无 of:）不归入任何实体
        assert!(!g.values().any(|insts| insts.values().any(|ns| ns.iter().any(|n| n.ends_with(".T")))));
    }

    /// 收集表达式 Value 里所有 `ref` 名（递归）。
    fn collect_refs(v: &Value, acc: &mut Vec<String>) {
        match v {
            Value::Mapping(m) => {
                if let Some(r) = m.get("ref").and_then(Value::as_str) {
                    acc.push(r.to_string());
                }
                for (_, val) in m {
                    collect_refs(val, acc);
                }
            }
            Value::Sequence(s) => {
                for e in s {
                    collect_refs(e, acc);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_aggregate_lowering_children_and_all() {
        // FSPM 风险3：children 聚合（节→Σ果）+ all 聚合（整株共享方程→Σ全果）
        let src = yaml(
            r#"
structure:
  entities:
    metamer: { count: 2, topology: chain }
    fruit:   { per: metamer, count: 3 }
variables:
  fmass:       { of: fruit, class: state, init: 0, rate: fg }
  fg:          { of: fruit, class: rate }
  node_fruit:  { of: metamer, class: state }
  plant_fruit: { class: state }
equations:
  - { id: NF, for: metamer, output: node_fruit,
      expression: { agg: sum, over: children, body: { ref: fmass } } }
  - { id: PF, output: plant_fruit,
      expression: { agg: sum, over: all, of: fruit, body: { ref: fmass } } }
"#,
        );
        let out = expand_structure(src).unwrap();
        let eqs = out.get("equations").unwrap().as_sequence().unwrap();
        let find = |key: &str| {
            eqs.iter()
                .map(|e| e.as_mapping().unwrap())
                .find(|m| s(m, "id").as_deref() == Some(key))
                .unwrap_or_else(|| panic!("找不到方程 {key}"))
                .clone()
        };

        // NF__1（metamer 1）：children = fruit 1.1/1.2/1.3 → vsum(vector(...))（扁平 n 元）
        let nf1 = find("NF__1");
        assert_eq!(s(&nf1, "output").as_deref(), Some("node_fruit__1"));
        let mut r1 = Vec::new();
        collect_refs(nf1.get("expression").unwrap(), &mut r1);
        r1.sort();
        r1.dedup();
        assert_eq!(r1, vec!["fmass__1_1", "fmass__1_2", "fmass__1_3"]);
        // 折叠成 vsum over vector（扁平 n 元；lower 后无 agg 残留）
        assert_eq!(
            nf1.get("expression").unwrap().as_mapping().unwrap().get("op").and_then(Value::as_str),
            Some("vsum")
        );

        // NF__2（metamer 2）：children = fruit 2.1/2.2/2.3
        let nf2 = find("NF__2");
        let mut r2 = Vec::new();
        collect_refs(nf2.get("expression").unwrap(), &mut r2);
        r2.sort();
        r2.dedup();
        assert_eq!(r2, vec!["fmass__2_1", "fmass__2_2", "fmass__2_3"]);

        // PF（整株共享方程，over: all fruit）：全 6 果；output 无后缀
        let pf = find("PF");
        assert_eq!(s(&pf, "output").as_deref(), Some("plant_fruit"));
        let mut rp = Vec::new();
        collect_refs(pf.get("expression").unwrap(), &mut rp);
        rp.sort();
        rp.dedup();
        assert_eq!(rp.len(), 6, "{rp:?}");
        assert!(rp.contains(&"fmass__1_1".to_string()) && rp.contains(&"fmass__2_3".to_string()));

        // 4a：聚合出处写进 structure.aggregations（供契约/前端「聚合可见性」）
        let st = out.get("structure").unwrap().as_mapping().unwrap();
        let aggsm = st.get("aggregations").unwrap().as_sequence().unwrap();
        assert!(aggsm.iter().any(|a| {
            let m = a.as_mapping().unwrap();
            s(m, "output").as_deref() == Some("node_fruit")
                && s(m, "over").as_deref() == Some("children")
                && s(m, "kind").as_deref() == Some("sum")
        }), "缺 node_fruit children 聚合出处");
        assert!(aggsm.iter().any(|a| {
            let m = a.as_mapping().unwrap();
            s(m, "output").as_deref() == Some("plant_fruit")
                && s(m, "over").as_deref() == Some("all")
                && s(m, "entity").as_deref() == Some("fruit")
        }), "缺 plant_fruit all 聚合出处");
    }

    #[test]
    fn test_mean_empty_set_rejected() {
        // mean over 空集（count 0）→ 加载期报错（不设运行时 0/NaN）
        let src = yaml(
            r#"
structure:
  entities:
    leaf: { count: 0 }
variables:
  m: { class: state }
equations:
  - { id: M, output: m, expression: { agg: mean, over: all, of: leaf, body: { ref: x } } }
"#,
        );
        let err = expand_structure(src);
        assert!(matches!(err, Err(StructureError::EmptyAggregate { .. })), "应为 EmptyAggregate，实为 {err:?}");

        // sum over 空集 → 0（不报错）
        let src2 = yaml(
            r#"
structure:
  entities:
    leaf: { count: 0 }
variables:
  m: { class: state }
equations:
  - { id: M, output: m, expression: { agg: sum, over: all, of: leaf, body: { ref: x } } }
"#,
        );
        let out = expand_structure(src2).unwrap();
        let eq = out.get("equations").unwrap().as_sequence().unwrap()[0].as_mapping().unwrap();
        assert!(eq.get("expression").unwrap().as_mapping().unwrap().contains_key("const"));
    }
}
