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
    // 5) 实例化方程。
    if let Some(Value::Sequence(eqs)) = map.get("equations") {
        let new_eqs = expand_eqs(eqs, &entities, &insts, &var_entity)?;
        map.insert(Value::from("equations"), Value::Sequence(new_eqs));
    }
    // 6) 用实例化后的 StructureInfo 替换声明。
    map.insert(Value::from("structure"), build_structure_info(&entities, &insts));

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
) -> Result<Vec<Value>, StructureError> {
    let chain: HashMap<&str, bool> =
        entities.iter().map(|e| (e.name.as_str(), e.topology.as_deref() == Some("chain"))).collect();
    let mut out = Vec::new();
    for eq in eqs {
        let Some(emap) = eq.as_mapping() else {
            out.push(eq.clone());
            continue;
        };
        let Some(ent) = emap.get("for").and_then(Value::as_str) else {
            out.push(eq.clone()); // 无 for: = 整株共享方程，原样
            continue;
        };
        let ent = ent.to_string();
        let list = insts.get(&ent).map(|v| v.as_slice()).unwrap_or(&[]);
        let is_chain = *chain.get(ent.as_str()).unwrap_or(&false);
        // 父实体的实例数（解析 of: prev/next 边界）。
        let n_self = list.len();
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
            // 表达式 ref 解析
            if let Some(expr) = clone.get("expression") {
                let ctx = RefCtx { ent: &ent, inst, idx, n_self, is_chain, var_entity, insts };
                let rewritten = rewrite_refs(expr, &ctx)?;
                clone.insert(Value::from("expression"), rewritten);
            }
            clone.insert(Value::from("instance"), instance_tag(&ent, &inst.id));
            out.push(Value::Mapping(clone));
        }
    }
    Ok(out)
}

/// ref 解析上下文（当前方程实例所在实体/实例 + 邻居/父查找所需）。
struct RefCtx<'a> {
    ent: &'a str,
    inst: &'a Inst,
    idx: usize,
    n_self: usize,
    is_chain: bool,
    var_entity: &'a HashMap<String, String>,
    insts: &'a HashMap<String, Vec<Inst>>,
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

    if let Some(name) = m.get("ref").and_then(Value::as_str) {
        let of = m.get("of").and_then(Value::as_str);
        let var_ent = ctx.var_entity.get(name).map(|s| s.as_str());
        // 整株共享量（非任何实体的 of: 变量）→ 原样（忽略多余 of:）。
        if var_ent.is_none() {
            return Ok(ref_value(name));
        }
        // 结构量：按 of: 解析实例。
        let target_id: Option<String> = match of {
            None | Some("self") => Some(ctx.inst.id.clone()),
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
            Some("parent") => ctx.inst.parent.clone(),
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

/// 构造 StructureInfo 的 YAML（entities + instances + topology）。
fn build_structure_info(entities: &[EntityDef], insts: &HashMap<String, Vec<Inst>>) -> Value {
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
}
