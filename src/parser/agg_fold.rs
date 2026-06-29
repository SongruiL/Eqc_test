//! 聚合折叠的单一真相源（SSOT）。
//!
//! cohort 的 `sum_over`/`prod_over`（[`super::cohort_expand`]）与 structure 的拓扑聚合
//! `{agg, over, body}`（[`super::structure_expand`]）共用同一套「把若干项折成标量 add/mul 链」
//! 的逻辑——避免两份实现漂移（FSPM 风险3 收编 · 第3步）。
//!
//! 输出**逐位一致**于旧左折叠二元链（≥2 项折成扁平 `vsum`/`vprod` over `vector`、单项原样、
//! 空集→单位元；见 [`fold_sum_or_prod`]），故 cohort 既有模型仿真不变；且**深度恒为 1**、不随项数加深栈。

use serde_yaml::{Mapping, Value};

/// 构造 `{op: <op>, args: [...]}` 节点。
pub fn op_args(op: &str, args: Vec<Value>) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("op"), Value::from(op));
    m.insert(Value::from("args"), Value::Sequence(args));
    Value::Mapping(m)
}

/// 构造 `{const: x}` 节点。
pub fn const_value(x: f64) -> Value {
    let mut m = Mapping::new();
    m.insert(Value::from("const"), Value::from(x));
    Value::Mapping(m)
}

/// 左折叠成二元 `op` 链：`[a,b,c]` → `op(op(a,b),c)`。`args` 必须非空。
pub fn fold_binary(op: &str, args: Vec<Value>) -> Value {
    let mut it = args.into_iter();
    let mut acc = it.next().expect("fold_binary 至少一个参数");
    for x in it {
        acc = op_args(op, vec![acc, x]);
    }
    acc
}

/// sum / prod 折叠：≥2 项 → 扁平 n 元 `vsum`/`vprod` over `vector` 字面量；单项 → 原样；空集 → 单位元。
/// cohort `sum_over`/`prod_over` 与 structure `sum`/`prod` 聚合共用。
///
/// **扁平而非左嵌套链**：N 项是 `vector` 的兄弟、深度恒为 1，故反序列化/求值/遍历**不随项数 N 加深栈**
/// （FSPM 多器官 Σ 不再栈溢出）。**逐位等于旧 `add`/`mul` 左嵌套链**：`Reduce(Sum)`=`iter().sum()`
/// 从 0.0 左到右累加（`0+t1` 精确）、`Reduce(Prod)`=`iter().product()` 从 1.0（`1*t1` 精确）
/// → cohort 既有模型仿真**逐位不变**。
pub fn fold_sum_or_prod(is_sum: bool, terms: Vec<Value>) -> Value {
    match terms.len() {
        0 => const_value(if is_sum { 0.0 } else { 1.0 }),
        1 => terms.into_iter().next().expect("len==1"),
        _ => op_args(if is_sum { "vsum" } else { "vprod" }, vec![op_args("vector", terms)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_binary_left_assoc() {
        // [a,b,c] → add(add(a,b),c)
        let r = fold_binary("add", vec![const_value(1.0), const_value(2.0), const_value(3.0)]);
        let m = r.as_mapping().unwrap();
        assert_eq!(m.get("op").and_then(Value::as_str), Some("add"));
        let args = m.get("args").unwrap().as_sequence().unwrap();
        assert_eq!(args.len(), 2);
        // 左项还是 add（嵌套），右项是 const 3
        assert_eq!(args[0].as_mapping().unwrap().get("op").and_then(Value::as_str), Some("add"));
        assert_eq!(args[1].as_mapping().unwrap().get("const").and_then(Value::as_f64), Some(3.0));
    }

    #[test]
    fn test_fold_sum_or_prod_empty() {
        // 空集：sum→0 / prod→1
        assert_eq!(fold_sum_or_prod(true, vec![]).as_mapping().unwrap().get("const").and_then(Value::as_f64), Some(0.0));
        assert_eq!(fold_sum_or_prod(false, vec![]).as_mapping().unwrap().get("const").and_then(Value::as_f64), Some(1.0));
        // 单项：原样返回（无包裹）
        let one = fold_sum_or_prod(true, vec![const_value(5.0)]);
        assert_eq!(one.as_mapping().unwrap().get("const").and_then(Value::as_f64), Some(5.0));
    }
}
