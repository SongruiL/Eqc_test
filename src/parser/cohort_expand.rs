//! Cohort（同期群）展开：把 `.eq.yaml` 里「按下标的模板」在**加载期**展开成纯标量。
//!
//! 这是 B 路线「cohort 抽象」的落地（A 方案：编译期宏展开）。设计要点：cohort 只是
//! YAML 层的语法糖，展开后产出一个**普通标量** [`EquationFile`]——求值器、校验、DAG、
//! 报告、代码生成全都不用改，仍然只看到标量。整个过程对 `serde_yaml::Value` 做一次重写，
//! 在反序列化成 `EquationFile` 之前完成（与 `reclassify_parameters` 同属加载期处理）。
//!
//! # 语法
//!
//! ```yaml
//! cohorts:
//!   fruit: { size: 3, index: q }        # 定义同期群 fruit，下标 q = 1..3
//!
//! parameters:
//!   anthesis: { cohort: fruit, name_cn: 开花日, values: [55, 95, 130], unit: d }
//!
//! variables:
//!   TF:     { cohort: fruit, class: state, init: 0.0, rate: rateTF }
//!   rateTF: { cohort: fruit, class: rate }
//!
//! equations:
//!   - { output: rateTF, cohort: fruit,
//!       expression: { op: mul, args: [ {ref: T}, {ref: active, at: q} ] } }
//!   - { output: GS,
//!       expression: { op: mul, args: [ {const: 0.24},
//!                     { op: sum_over, over: fruit, body: { ref: DRFG, at: q } } ] } }
//! ```
//!
//! 展开规则：
//! - `cohort: F` 的变量/参数/方程 → 复制 `size` 份，名字后缀 `__i`（i 从 1 起）。
//! - `{ref: X, at: q}` → `{ref: X__i}`（i = 下标 q 的当前值）；`{idx: q}` → `{const: i}`。
//! - `{ref: X, at: q, offset: k}` → 引用同家族**相邻**成员 `{ref: X__(i+k)}`（k 整数，可负）；
//!   越界（i+k < 1 或 > size）→ `{const: 0}`。用于"固定箱车列"等需 j-1/j+1 流的阶段模型
//!   （如番茄果实发育阶段间碳/果数流动：首阶段无前驱、末阶段无后继 → 自动归 0）。
//! - `{op: sum_over, over: F, body: B}` → 把 B 对 F 的每个下标展开，折成 `add` 链；
//!   `prod_over` 折成 `mul` 链。空家族 → `sum_over=0`、`prod_over=1`。
//! - cohort 变量的 `rate`/`prev` 若指向**同家族**成员，同样加 `__i` 后缀。

use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};

/// Cohort 展开错误。
#[derive(Debug, Clone, PartialEq)]
pub enum CohortError {
    /// 顶层结构不是 mapping。
    NotMapping,
    /// `cohorts:` 里某家族定义不合法（缺 size/index）。
    BadFamily(String),
    /// 引用了未声明的同期群家族。
    UnknownCohort(String),
    /// `{at: q}` / `{idx: q}` 引用了当前作用域里没有的下标名。
    UnknownIndex(String),
    /// cohort 参数既没有 `values` 也没有 `default`。
    ParamNoDefault(String),
    /// `values` 列表长度与家族 size 不一致。
    ValuesLenMismatch { name: String, expected: usize, found: usize },
    /// 必需字段缺失。
    MissingField { ctx: String, field: String },
}

impl std::fmt::Display for CohortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CohortError::NotMapping => write!(f, "顶层 YAML 不是 mapping"),
            CohortError::BadFamily(n) => write!(f, "cohort 家族 {n} 定义不合法（需 size 与 index）"),
            CohortError::UnknownCohort(n) => write!(f, "引用了未声明的 cohort 家族: {n}"),
            CohortError::UnknownIndex(n) => write!(f, "未知下标名: {n}"),
            CohortError::ParamNoDefault(n) => {
                write!(f, "cohort 参数 {n} 需要 values 列表或 default 值")
            }
            CohortError::ValuesLenMismatch { name, expected, found } => {
                write!(f, "cohort 参数 {name} 的 values 长度 {found} 与 size {expected} 不一致")
            }
            CohortError::MissingField { ctx, field } => {
                write!(f, "{ctx} 缺少必需字段 {field}")
            }
        }
    }
}

impl std::error::Error for CohortError {}

/// 一个同期群家族。
struct Family {
    size: usize,
    index: String,
}

/// 下标作用域：下标名 -> 当前具体值。
type IdxEnv = HashMap<String, usize>;

/// 对整份 `.eq.yaml` 的 `Value` 做 cohort 展开。无 `cohorts:` 段时原样返回。
pub fn expand_cohorts(mut root: Value) -> Result<Value, CohortError> {
    let map = root.as_mapping_mut().ok_or(CohortError::NotMapping)?;

    // 1. 解析 cohorts: 段（无则直接返回）
    let families = parse_families(map)?;
    if families.is_empty() {
        return Ok(root);
    }
    map.remove("cohorts");

    // 2. 收集每个家族的成员名（变量 + 参数），用于 rate/prev 后缀判定
    let members = collect_members(map, &families);
    let all_members: HashSet<String> = members.values().flatten().cloned().collect();

    // 3. 展开 parameters / variables / equations
    if let Some(Value::Mapping(params)) = map.get_mut("parameters") {
        *params = expand_decl_map(params, &families, &members, true)?;
    }
    if let Some(Value::Mapping(vars)) = map.get_mut("variables") {
        *vars = expand_decl_map(vars, &families, &members, false)?;
    }
    if let Some(Value::Sequence(eqs)) = map.get_mut("equations") {
        *eqs = expand_equations(eqs, &families, &all_members)?;
    }

    Ok(root)
}

/// 解析 `cohorts:` 段。
fn parse_families(map: &Mapping) -> Result<HashMap<String, Family>, CohortError> {
    let mut out = HashMap::new();
    let Some(Value::Mapping(cohorts)) = map.get("cohorts") else {
        return Ok(out);
    };
    for (k, v) in cohorts {
        let name = k.as_str().unwrap_or_default().to_string();
        let spec = v.as_mapping().ok_or_else(|| CohortError::BadFamily(name.clone()))?;
        let size = spec
            .get("size")
            .and_then(Value::as_u64)
            .ok_or_else(|| CohortError::BadFamily(name.clone()))? as usize;
        let index = spec
            .get("index")
            .and_then(Value::as_str)
            .ok_or_else(|| CohortError::BadFamily(name.clone()))?
            .to_string();
        out.insert(name, Family { size, index });
    }
    Ok(out)
}

/// 收集每个家族的成员名（声明了 `cohort: F` 的变量名与参数名）。
fn collect_members(map: &Mapping, families: &HashMap<String, Family>) -> HashMap<String, HashSet<String>> {
    let mut members: HashMap<String, HashSet<String>> =
        families.keys().map(|k| (k.clone(), HashSet::new())).collect();
    for section in ["variables", "parameters"] {
        if let Some(Value::Mapping(m)) = map.get(section) {
            for (k, v) in m {
                if let (Some(name), Some(fam)) =
                    (k.as_str(), v.as_mapping().and_then(|d| get_str(d, "cohort")))
                {
                    if let Some(set) = members.get_mut(&fam) {
                        set.insert(name.to_string());
                    }
                }
            }
        }
    }
    members
}

/// 展开变量或参数声明 mapping。`is_param` 时处理 cohort 参数的 `values`/`default`。
fn expand_decl_map(
    decls: &Mapping,
    families: &HashMap<String, Family>,
    members: &HashMap<String, HashSet<String>>,
    is_param: bool,
) -> Result<Mapping, CohortError> {
    let mut out = Mapping::new();
    for (k, v) in decls {
        let name = k.as_str().unwrap_or_default().to_string();
        let decl = match v.as_mapping() {
            Some(m) => m,
            None => {
                out.insert(k.clone(), v.clone());
                continue;
            }
        };
        let Some(fam_name) = get_str(decl, "cohort") else {
            out.insert(k.clone(), v.clone());
            continue;
        };
        let fam = families.get(&fam_name).ok_or_else(|| CohortError::UnknownCohort(fam_name.clone()))?;

        // cohort 参数：取 values 列表或 default
        let values: Option<Vec<f64>> = if is_param {
            match decl.get("values").and_then(Value::as_sequence) {
                Some(seq) => {
                    if seq.len() != fam.size {
                        return Err(CohortError::ValuesLenMismatch {
                            name: name.clone(),
                            expected: fam.size,
                            found: seq.len(),
                        });
                    }
                    Some(seq.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect())
                }
                None => {
                    if decl.get("default").and_then(Value::as_f64).is_none() {
                        return Err(CohortError::ParamNoDefault(name.clone()));
                    }
                    None // 用 default（已在 decl 里）
                }
            }
        } else {
            None
        };

        for i in 1..=fam.size {
            let mut clone = decl.clone();
            clone.remove("cohort");
            if is_param {
                clone.remove("values");
                if let Some(vals) = &values {
                    clone.insert(Value::from("default"), Value::from(vals[i - 1]));
                }
            } else {
                // 变量：rate/prev 指向同家族成员时加后缀
                for field in ["rate", "prev"] {
                    if let Some(r) = get_str(&clone, field) {
                        if members.get(&fam_name).is_some_and(|s| s.contains(&r)) {
                            clone.insert(Value::from(field), Value::from(suffix(&r, i)));
                        }
                    }
                }
            }
            out.insert(Value::from(suffix(&name, i)), Value::Mapping(clone));
        }
    }
    Ok(out)
}

/// 展开方程序列。
fn expand_equations(
    eqs: &[Value],
    families: &HashMap<String, Family>,
    all_members: &HashSet<String>,
) -> Result<Vec<Value>, CohortError> {
    let mut out = Vec::new();
    for eq in eqs {
        let Some(emap) = eq.as_mapping() else {
            out.push(eq.clone());
            continue;
        };
        match get_str(emap, "cohort") {
            Some(fam_name) => {
                let fam = families
                    .get(&fam_name)
                    .ok_or_else(|| CohortError::UnknownCohort(fam_name.clone()))?;
                for i in 1..=fam.size {
                    let mut clone = emap.clone();
                    clone.remove("cohort");
                    // output 后缀
                    let out_name = get_str(&clone, "output").ok_or_else(|| CohortError::MissingField {
                        ctx: "equation".into(),
                        field: "output".into(),
                    })?;
                    clone.insert(Value::from("output"), Value::from(suffix(&out_name, i)));
                    // id 后缀（保证唯一）
                    if let Some(id) = get_str(&clone, "id") {
                        clone.insert(Value::from("id"), Value::from(format!("{id}_{i}")));
                    }
                    // 表达式重写（绑定本家族下标）
                    let mut env = IdxEnv::new();
                    env.insert(fam.index.clone(), i);
                    if let Some(expr) = clone.get("expression") {
                        let rewritten = rewrite_expr(expr, &env, families, all_members)?;
                        clone.insert(Value::from("expression"), rewritten);
                    }
                    out.push(Value::Mapping(clone));
                }
            }
            None => {
                // 标量方程：仍要重写（可能含 sum_over），下标作用域为空
                let mut clone = emap.clone();
                if let Some(expr) = clone.get("expression") {
                    let env = IdxEnv::new();
                    let rewritten = rewrite_expr(expr, &env, families, all_members)?;
                    clone.insert(Value::from("expression"), rewritten);
                }
                out.push(Value::Mapping(clone));
            }
        }
    }
    Ok(out)
}

/// 递归重写表达式 `Value`：处理 `at`/`idx`/`sum_over`/`prod_over`，其余结构原样递归。
fn rewrite_expr(
    v: &Value,
    env: &IdxEnv,
    families: &HashMap<String, Family>,
    all_members: &HashSet<String>,
) -> Result<Value, CohortError> {
    let Value::Mapping(m) = v else {
        // 序列：逐元素重写；标量：原样
        if let Value::Sequence(s) = v {
            let mut out = Vec::with_capacity(s.len());
            for e in s {
                out.push(rewrite_expr(e, env, families, all_members)?);
            }
            return Ok(Value::Sequence(out));
        }
        return Ok(v.clone());
    };

    // {ref: X [, at: q [, offset: k]]}
    // `offset`（整数，可负）= 引用同家族的相邻成员（如箱车 j-1/j+1）。
    // 越界（target < 1 或 > size）→ 折成 {const: 0}，天然处理首/末阶段无邻居的边界。
    // 无 offset（或 offset=0）→ 与原行为逐位一致（不需家族 size，老模型不受影响）。
    if let Some(name) = get_str(m, "ref") {
        if let Some(idx_name) = get_str(m, "at") {
            let i = *env.get(&idx_name).ok_or_else(|| CohortError::UnknownIndex(idx_name.clone()))?;
            let offset = m.get("offset").and_then(Value::as_i64).unwrap_or(0);
            if offset == 0 {
                return Ok(ref_value(&suffix(&name, i)));
            }
            // 带偏移：需所属家族的 size 做边界判定（按下标名反查家族）
            let size = families
                .values()
                .find(|f| f.index == idx_name)
                .map(|f| f.size)
                .ok_or_else(|| CohortError::UnknownIndex(idx_name.clone()))?;
            let target = i as i64 + offset;
            if target >= 1 && target <= size as i64 {
                return Ok(ref_value(&suffix(&name, target as usize)));
            } else {
                return Ok(const_value(0.0));
            }
        }
        return Ok(v.clone());
    }

    // {idx: q} -> {const: i}
    if let Some(idx_name) = get_str(m, "idx") {
        let i = *env.get(&idx_name).ok_or_else(|| CohortError::UnknownIndex(idx_name.clone()))?;
        return Ok(const_value(i as f64));
    }

    // {op: sum_over|prod_over, over: F, body: B}
    if let Some(op) = get_str(m, "op") {
        if op == "sum_over" || op == "prod_over" {
            let fam_name = get_str(m, "over").ok_or_else(|| CohortError::MissingField {
                ctx: format!("{op}"),
                field: "over".into(),
            })?;
            let fam = families
                .get(&fam_name)
                .ok_or_else(|| CohortError::UnknownCohort(fam_name.clone()))?;
            let body = m.get("body").ok_or_else(|| CohortError::MissingField {
                ctx: format!("{op}"),
                field: "body".into(),
            })?;
            let combine = if op == "sum_over" { "add" } else { "mul" };
            let ident = if op == "sum_over" { 0.0 } else { 1.0 };
            if fam.size == 0 {
                return Ok(const_value(ident));
            }
            let mut args = Vec::with_capacity(fam.size);
            for i in 1..=fam.size {
                let mut e2 = env.clone();
                e2.insert(fam.index.clone(), i);
                args.push(rewrite_expr(body, &e2, families, all_members)?);
            }
            // 折成二元链，避免对 add/mul 元数做假设
            return Ok(fold_binary(combine, args));
        }
    }

    // 普通结构：逐字段递归重写
    let mut out = Mapping::new();
    for (k, val) in m {
        out.insert(k.clone(), rewrite_expr(val, env, families, all_members)?);
    }
    Ok(Value::Mapping(out))
}

/// 把多个参数折成左结合的二元 `op` 链；单个参数原样返回。
fn fold_binary(op: &str, args: Vec<Value>) -> Value {
    let mut it = args.into_iter();
    let mut acc = it.next().expect("fold_binary 至少一个参数");
    for x in it {
        acc = op_value(op, vec![acc, x]);
    }
    acc
}

// —— 小工具 ——

fn suffix(name: &str, i: usize) -> String {
    format!("{name}__{i}")
}

fn get_str(m: &Mapping, key: &str) -> Option<String> {
    m.get(key).and_then(Value::as_str).map(|s| s.to_string())
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

fn op_value(op: &str, args: Vec<Value>) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("op"), Value::from(op));
    m.insert(Value::from("args"), Value::Sequence(args));
    Value::Mapping(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(s: &str) -> Value {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn test_no_cohorts_passthrough() {
        let src = yaml("meta: { id: M }\nvariables:\n  x: { type: input }\n");
        let out = expand_cohorts(src.clone()).unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn test_expand_variables_and_rate_suffix() {
        let src = yaml(
            r#"
cohorts:
  fruit: { size: 3, index: q }
variables:
  TF:     { cohort: fruit, class: state, init: 0.0, rate: rateTF }
  rateTF: { cohort: fruit, class: rate }
  GS:     { type: output }
"#,
        );
        let out = expand_cohorts(src).unwrap();
        let vars = out.get("variables").unwrap().as_mapping().unwrap();
        // 应有 TF__1..3、rateTF__1..3、GS（标量保持）
        assert!(vars.contains_key("TF__1") && vars.contains_key("TF__3"));
        assert!(vars.contains_key("rateTF__2"));
        assert!(vars.contains_key("GS"));
        assert!(!vars.contains_key("TF"));
        // TF__2 的 rate 应后缀为 rateTF__2（同家族成员）
        let tf2 = vars.get("TF__2").unwrap().as_mapping().unwrap();
        assert_eq!(get_str(tf2, "rate").as_deref(), Some("rateTF__2"));
        // cohorts: 段已移除
        assert!(out.get("cohorts").is_none());
    }

    #[test]
    fn test_expand_params_values() {
        let src = yaml(
            r#"
cohorts:
  fruit: { size: 3, index: q }
parameters:
  anthesis: { cohort: fruit, name_cn: 开花日, values: [55, 95, 130] }
"#,
        );
        let out = expand_cohorts(src).unwrap();
        let ps = out.get("parameters").unwrap().as_mapping().unwrap();
        assert_eq!(ps.get("anthesis__1").unwrap().as_mapping().unwrap().get("default").unwrap().as_f64(), Some(55.0));
        assert_eq!(ps.get("anthesis__3").unwrap().as_mapping().unwrap().get("default").unwrap().as_f64(), Some(130.0));
    }

    #[test]
    fn test_expand_equation_at_idx_and_sum_over() {
        let src = yaml(
            r#"
cohorts:
  fruit: { size: 3, index: q }
variables:
  DRFG: { cohort: fruit }
  GS:   { type: output }
equations:
  - id: E1
    output: DRFG
    cohort: fruit
    expression: { op: mul, args: [ { idx: q }, { ref: TF, at: q } ] }
  - id: E2
    output: GS
    expression: { op: mul, args: [ { const: 0.24 }, { op: sum_over, over: fruit, body: { ref: DRFG, at: q } } ] }
"#,
        );
        let out = expand_cohorts(src).unwrap();
        let eqs = out.get("equations").unwrap().as_sequence().unwrap();
        // E1 展开成 3 条 + E2（标量）= 4 条
        assert_eq!(eqs.len(), 4);

        // E1 第二条：output=DRFG__2，expr = mul(const 2, ref TF__2)
        let e1_2 = &eqs[1];
        assert_eq!(get_str(e1_2.as_mapping().unwrap(), "output").as_deref(), Some("DRFG__2"));
        let expr = e1_2.as_mapping().unwrap().get("expression").unwrap().as_mapping().unwrap();
        let args = expr.get("args").unwrap().as_sequence().unwrap();
        assert_eq!(args[0].as_mapping().unwrap().get("const").unwrap().as_f64(), Some(2.0)); // idx q -> const 2
        assert_eq!(get_str(args[1].as_mapping().unwrap(), "ref").as_deref(), Some("TF__2"));

        // E2：sum_over 展开成 add(add(DRFG__1, DRFG__2), DRFG__3)
        let e2 = eqs[3].as_mapping().unwrap();
        let e2_args = e2.get("expression").unwrap().as_mapping().unwrap().get("args").unwrap().as_sequence().unwrap();
        let sum = e2_args[1].as_mapping().unwrap();
        assert_eq!(get_str(sum, "op").as_deref(), Some("add"));
        // 外层 add 的两个参数：内层 add(...) 和 DRFG__3
        let outer = sum.get("args").unwrap().as_sequence().unwrap();
        assert_eq!(get_str(outer[1].as_mapping().unwrap(), "ref").as_deref(), Some("DRFG__3"));
        let inner = outer[0].as_mapping().unwrap();
        assert_eq!(get_str(inner, "op").as_deref(), Some("add"));
    }

    #[test]
    fn test_expand_neighbor_offset_boxcar() {
        // 固定箱车列：每阶段引用前驱(offset -1)与后继(offset +1)；首/末阶段越界→const 0
        let src = yaml(
            r#"
cohorts:
  stage: { size: 3, index: q }
variables:
  C:     { cohort: stage, class: state, init: 0.0, rate: rateC }
  rateC: { cohort: stage, class: rate }
equations:
  - id: FLOW
    output: rateC
    cohort: stage
    expression:
      op: sub
      args:
        - { ref: C, at: q, offset: -1 }
        - { ref: C, at: q, offset: 1 }
"#,
        );
        let out = expand_cohorts(src).unwrap();
        let eqs = out.get("equations").unwrap().as_sequence().unwrap();
        assert_eq!(eqs.len(), 3); // rateC__1..3

        // 取每条方程的 expression.args[0]（前驱 offset -1）与 args[1]（后继 offset +1）
        let arg = |i: usize, a: usize| -> &Value {
            &eqs[i].as_mapping().unwrap()
                .get("expression").unwrap().as_mapping().unwrap()
                .get("args").unwrap().as_sequence().unwrap()[a]
        };
        let is_const0 = |v: &Value| v.as_mapping().unwrap().get("const").and_then(Value::as_f64) == Some(0.0);
        let ref_name = |v: &Value| get_str(v.as_mapping().unwrap(), "ref");

        // q=1: 前驱越界→const 0；后继→C__2
        assert!(is_const0(arg(0, 0)));
        assert_eq!(ref_name(arg(0, 1)).as_deref(), Some("C__2"));
        // q=2: 前驱→C__1；后继→C__3
        assert_eq!(ref_name(arg(1, 0)).as_deref(), Some("C__1"));
        assert_eq!(ref_name(arg(1, 1)).as_deref(), Some("C__3"));
        // q=3: 前驱→C__2；后继越界→const 0
        assert_eq!(ref_name(arg(2, 0)).as_deref(), Some("C__2"));
        assert!(is_const0(arg(2, 1)));
    }

    #[test]
    fn test_offset_zero_equals_no_offset() {
        // offset: 0 与无 offset 逐位一致（向后兼容）
        let src = yaml(
            r#"
cohorts:
  fam: { size: 2, index: q }
variables:
  X: { cohort: fam }
equations:
  - { id: E, output: X, cohort: fam, expression: { ref: X, at: q, offset: 0 } }
"#,
        );
        let out = expand_cohorts(src).unwrap();
        let eqs = out.get("equations").unwrap().as_sequence().unwrap();
        let expr1 = eqs[1].as_mapping().unwrap().get("expression").unwrap();
        assert_eq!(get_str(expr1.as_mapping().unwrap(), "ref").as_deref(), Some("X__2"));
    }

    #[test]
    fn test_unknown_cohort_errors() {
        let src = yaml(
            r#"
cohorts:
  fruit: { size: 2, index: q }
variables:
  X: { cohort: leaf }
"#,
        );
        assert!(matches!(expand_cohorts(src), Err(CohortError::UnknownCohort(_))));
    }
}
