//! 聚合折叠的单一真相源（SSOT）。
//!
//! cohort 的 `sum_over`/`prod_over`（[`super::cohort_expand`]）与 structure 的拓扑聚合
//! `{agg, over, body}`（[`super::structure_expand`]）共用同一套「把若干项折成标量 add/mul 链」
//! 的逻辑——避免两份实现漂移（FSPM 风险3 收编 · 第3步）。
//!
//! 输出与各自原实现**逐位一致**（左折叠二元链、空集→单位元），故 cohort 既有模型仿真不变。

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

/// sum / prod 折叠：非空 → add/mul 链；空集 → 单位元（sum=0 / prod=1）。
/// cohort `sum_over`/`prod_over` 与 structure `sum`/`prod` 聚合共用。
pub fn fold_sum_or_prod(is_sum: bool, terms: Vec<Value>) -> Value {
    if terms.is_empty() {
        const_value(if is_sum { 0.0 } else { 1.0 })
    } else {
        fold_binary(if is_sum { "add" } else { "mul" }, terms)
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
