//! 动态模型的「独立仿真器」代码生成（Python）。
//!
//! 把 [`crate::sim::build_plan`] 编译出的步进计划（**与 `eqc simulate` 引擎共用**）翻译成
//! 一段可独立运行的 Python `simulate()`：逐日显式 Euler——状态量 `X[n]=X[n-1]+rate[n]`、
//! 延迟寄存器 `X[n]=src[n-1]`、内置 `DAT`。方程体复用 [`crate::ast::Expr::to_python`]。
//!
//! 当前覆盖**标量动态模型**（目标：与 `eqc simulate` 数值一致）。向量模型（cohort/`vsum`）
//! 需 numpy 改造，后置。

use crate::schema::EquationFile;
use crate::sim::{build_plan, PlanStep};

/// f64 → Python 字面量（最短可往返写法，如 `0.0` / `2.75` / `0.016667`）。
fn fnum(x: f64) -> String {
    format!("{:?}", x)
}

/// 为动态模型生成独立 Python 仿真器；非动态（无状态量/延迟寄存器）返回 `None`。
pub fn generate_python_simulator(file: &EquationFile) -> Option<String> {
    let plan = build_plan(file).ok()?;
    let dynamic = !plan.delays.is_empty()
        || plan.steps.iter().any(|s| matches!(s, PlanStep::Integrator { .. }));
    if !dynamic {
        return None;
    }

    let is_param = |name: &str| file.parameters.contains_key(name);
    let title = if file.meta.name_cn.is_empty() {
        file.meta.id.clone()
    } else {
        file.meta.name_cn.clone()
    };
    let id = file.meta.id.to_lowercase();

    let mut l: Vec<String> = Vec::new();
    l.push(format!("# —— 由 EQC 生成的独立仿真器：{title} ——"));
    l.push("# 逐日显式 Euler，与 `eqc simulate` 同语义（同一份步进计划生成）。自动生成，请勿手改。".into());
    l.push("# 用法: simulate(drivers, steps[, params][, init]) -> {变量名: 逐日列表}".into());
    l.push(format!("#       命令行: python {id}_sim.py <驱动量CSV>  → 打印各变量末值"));
    l.push("import numpy as np".into());
    l.push("import math".into());
    l.push("from types import SimpleNamespace".into());
    l.push(String::new());
    // 记录 helper：标量→name；向量→name[1]/name[2]…（与 eqc simulate 的 flatten 一致）
    l.push("def _rec(traj, name, v):".into());
    l.push("    a = np.asarray(v, dtype=float)".into());
    l.push("    if a.ndim == 0:".into());
    l.push("        traj.setdefault(name, []).append(float(a))".into());
    l.push("    else:".into());
    l.push("        f = a.reshape(-1)".into());
    l.push("        for _i in range(f.size):".into());
    l.push("            traj.setdefault(name + '[' + str(_i + 1) + ']', []).append(float(f[_i]))".into());
    l.push(String::new());

    // 参数默认值（标量数字；向量参数为列表，标量生成器暂不支持其覆盖语义）
    l.push("_DEFAULTS = {".into());
    for (name, p) in &file.parameters {
        let val = match &p.values {
            Some(v) => format!("np.array([{}])", v.iter().map(|x| fnum(*x)).collect::<Vec<_>>().join(", ")),
            None => fnum(p.default),
        };
        l.push(format!("    '{name}': {val},"));
    }
    l.push("}".into());

    // 状态量 / 延迟寄存器初值
    l.push("_INIT = {".into());
    for (name, var) in &file.variables {
        if (var.is_integrator() || var.is_delay()) && var.init.is_some() {
            l.push(format!("    '{name}': {},", fnum(var.init.unwrap())));
        }
    }
    l.push("}".into());
    l.push(String::new());

    // simulate()
    l.push("def simulate(drivers, steps, params=None, init=None):".into());
    l.push("    _p = dict(_DEFAULTS); _p.update(params or {})".into());
    l.push("    p = SimpleNamespace(**_p)".into());
    l.push("    _init = dict(_INIT); _init.update(init or {})".into());
    l.push("    traj = {}".into());
    l.push("    prev = {}".into());
    // 时间步长（来自模型 meta.dt；状态量积分 X+=rate·dt，与引擎一致）
    l.push(format!("    _dt = {}", fnum(file.meta.dt)));
    l.push("    for _n in range(1, steps + 1):".into());
    l.push("        DAT = _n".into());

    // 驱动量（逐日序列取本步值）
    for d in &plan.drivers {
        l.push(format!("        {d} = drivers['{d}'][_n - 1]"));
    }
    // 延迟寄存器：首步=init，否则 src 上一步值
    for (name, src, _init) in &plan.delays {
        let src_prev = if is_param(src) {
            format!("p.{src}")
        } else {
            format!("prev['{src}']")
        };
        l.push(format!("        {name} = _init['{name}'] if _n == 1 else {src_prev}"));
    }
    // 步内拓扑序：方程 + 积分状态量
    for step in &plan.steps {
        match step {
            PlanStep::Equation { name, expr } => {
                l.push(format!("        {name} = {}", expr.to_python("p")));
            }
            PlanStep::Integrator { name, rate, .. } => {
                let rate_ref = if is_param(rate) {
                    format!("p.{rate}")
                } else {
                    (*rate).to_string()
                };
                l.push(format!(
                    "        {name} = (_init['{name}'] if _n == 1 else prev['{name}']) + {rate_ref} * _dt"
                ));
            }
        }
    }
    // 首步：把延迟寄存器的标量 init 按其来源的形状广播（向量延迟寄存器记录形状跨步一致；
    // 标量来源时为恒等，不影响数值）。复刻引擎 step-0 reshape。
    if !plan.delays.is_empty() {
        l.push("        if _n == 1:".into());
        for (name, src, _init) in &plan.delays {
            let src_ref = if is_param(src) {
                format!("p.{src}")
            } else {
                (*src).to_string()
            };
            l.push(format!(
                "            {name} = _init['{name}'] + 0.0 * np.asarray({src_ref}, dtype=float)"
            ));
        }
    }
    // 快照本步全部变量（供下一步的积分/延迟读取）+ 记录轨迹（向量自动展平为 name[i]）
    l.push("        prev = {".into());
    for name in file.variables.keys() {
        l.push(format!("            '{name}': {name},"));
    }
    l.push("        }".into());
    l.push("        for _k, _v in prev.items():".into());
    l.push("            _rec(traj, _k, _v)".into());
    l.push("    return traj".into());
    l.push(String::new());

    // __main__：读驱动量 CSV 跑一季、打印各变量末值（便于与 `eqc simulate` 对拍）
    l.push("if __name__ == '__main__':".into());
    l.push("    import sys, csv".into());
    l.push("    cols = {}".into());
    l.push("    with open(sys.argv[1], newline='') as _f:".into());
    l.push("        _r = csv.reader(_f); _hdr = next(_r)".into());
    l.push("        for _h in _hdr: cols[_h] = []".into());
    l.push("        for _row in _r:".into());
    l.push("            for _h, _x in zip(_hdr, _row): cols[_h].append(float(_x))".into());
    l.push("    _steps = len(next(iter(cols.values())))".into());
    l.push("    _traj = simulate(cols, _steps)".into());
    l.push("    for _k in sorted(_traj): print(_k, repr(_traj[_k][-1]))".into());
    l.push(String::new());

    Some(l.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::schema::{DataType, Equation, Metadata, Variable, VarClass, VariableType};
    use indexmap::IndexMap;

    fn meta(id: &str) -> Metadata {
        Metadata {
            id: id.into(),
            model: id.into(),
            name_cn: "测试".into(),
            name_en: None,
            version: "1.0".into(),
            description: None,
            reference: None,
            source_files: vec![],
            dt: 1.0,
            dt_seconds: None,
            calibration: None,
            modules: Default::default(),
        }
    }

    fn var(t: VariableType, class: Option<VarClass>, init: Option<f64>, rate: Option<&str>) -> Variable {
        Variable {
            var_type: t,
            dtype: DataType::Float,
            unit: None,
            description: None,
            label: None,
            measurable: false,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class,
            init,
            rate: rate.map(|s| s.to_string()),
            prev: None,
         instance: None }
    }

    #[test]
    fn test_generates_for_dynamic() {
        // 驱动 T；速率 R=T*2；积分状态 X(init=1, rate=R)
        let mut variables = IndexMap::new();
        variables.insert("T".into(), var(VariableType::Input, Some(VarClass::Driving), None, None));
        variables.insert("R".into(), var(VariableType::Intermediate, Some(VarClass::Rate), None, None));
        variables.insert("X".into(), var(VariableType::Output, Some(VarClass::State), Some(1.0), Some("R")));
        let file = EquationFile {
            meta: meta("M"),
            parameters: Default::default(),
            variables,
            equations: vec![Equation {
                id: "E".into(),
                name: "rate".into(),
                output: "R".into(),
                expression: Expr::mul(Expr::var("T"), Expr::Const(2.0)),
                formula_display: None,
                reference: None, gp_target: None, provenance: None,
             instance: None }],
         structure: None };
        let code = generate_python_simulator(&file).expect("动态模型应生成仿真器");
        assert!(code.contains("def simulate("));
        assert!(code.contains("def _rec(")); // 展平记录 helper（标量/向量通吃）
        assert!(code.contains("_INIT = {"));
        assert!(code.contains("'X': 1.0"));
        assert!(code.contains("T = drivers['T'][_n - 1]")); // 驱动量逐日取值
        // 积分更新：X[n] = (init 或 prev) + rate·dt（meta.dt 缺省 1.0）
        assert!(code.contains("_dt = 1.0"));
        assert!(code.contains("X = (_init['X'] if _n == 1 else prev['X']) + R * _dt"));
    }

    #[test]
    fn test_none_for_static() {
        // 纯静态（无状态/延迟）→ 不生成
        let mut variables = IndexMap::new();
        variables.insert("a".into(), var(VariableType::Input, None, None, None));
        variables.insert("b".into(), var(VariableType::Output, None, None, None));
        let file = EquationFile {
            meta: meta("S"),
            parameters: Default::default(),
            variables,
            equations: vec![Equation {
                id: "E".into(),
                name: "b".into(),
                output: "b".into(),
                expression: Expr::var("a"),
                formula_display: None,
                reference: None, gp_target: None, provenance: None,
             instance: None }],
         structure: None };
        assert!(generate_python_simulator(&file).is_none());
    }
}
