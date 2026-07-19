//! 档2 V1 双路径 + 隐式冒烟：在 **greenhouse_v2_base d2（冠层活性+水汽）** 上验隐式↔显式一致性。
//!
//! 加载 `greenhouse_v2_base.eq.yaml`（`_prev`-free），跑：
//!   - 隐式 BDF（`simulate_implicit`，smooth_eps=None 原样 min）→ 近精确参照；
//!   - 显式 Euler（`simulate`）逐步细化 dt → 应随 dt→0 收敛到隐式（V1 双路径·同一 formulation）；
//!   - 另跑隐式 smooth_eps=Some(0.05)（档2 生产路径）→ 确认平滑 min 不崩、与原样偏差小（冒烟）。
//! 常数驱动（一个白昼·lai=2 激活冠层+蒸腾+结露）隔离求解器误差。
//!
//! 运行：`cargo run --features implicit --example v2_greenhouse`

use std::collections::HashMap;
use std::path::Path;

use equation_compiler::parse_file;
use equation_compiler::sim::implicit::{simulate_implicit, ImplicitOpts};
use equation_compiler::sim::{simulate, SimInput};

fn const_drivers(names: &[&str], vals: &[f64], steps: usize) -> HashMap<String, Vec<f64>> {
    names.iter().zip(vals).map(|(n, &v)| (n.to_string(), vec![v; steps])).collect()
}

fn soil_init() -> HashMap<String, f64> {
    // GreenLight 土温剖面 + vpAir 默认（1200 已在模型 init）
    [
        ("tSo__1", 16.5), ("tSo__2", 13.364), ("tSo__3", 10.228),
        ("tSo__4", 7.092), ("tSo__5", 3.956),
    ].iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn main() {
    let path = "/root/Maestro/greenhouse_model/greenhouse_v2_base.eq.yaml";
    let file = parse_file(Path::new(path)).expect("解析 d2 温室模型");
    println!("模型: {} v{} ({})  dt={}s", file.meta.id, file.meta.version, file.meta.name_cn, file.meta.dt);

    // 常数驱动：白昼·lai=2·vpOut=1650·co2Out=700（激活冠层短波/蒸腾/结露/CO₂交换）。
    let drv_names = ["iGlob", "tOut", "tSky", "tSoOut", "wind", "lai", "vpOut", "co2Out"];
    let drv_vals = [800.0, 20.0, -10.0, 15.0, 2.0, 2.0, 1650.0, 700.0];
    let states = ["tCan", "tAir", "tCov", "tFlr", "vpAir", "co2Air"];

    let horizon = 1800.0_f64; // 30 分钟

    // —— 隐式参照（smooth_eps=None·原样 min·紧容差）——
    let dt_ref = file.meta.dt; // 10s
    let steps_ref = (horizon / dt_ref).round() as usize;
    let mk = |steps: usize, dt: f64| SimInput {
        steps,
        dt: Some(dt),
        drivers: const_drivers(&drv_names, &drv_vals, steps),
        init_overrides: soil_init(),
        param_overrides: HashMap::new(),
    };
    let impl_ref = simulate_implicit(&file, &mk(steps_ref, dt_ref),
        ImplicitOpts { rtol: 1e-9, atol: 1e-9, smooth_eps: None }).expect("隐式(None)求解");
    let ref_final: Vec<f64> = states.iter().map(|s| impl_ref.final_value(s).unwrap()).collect();

    println!("\n隐式 BDF 参照(smooth_eps=None) t={horizon}s 末值:");
    for (s, v) in states.iter().zip(&ref_final) { println!("  {s:9} = {v:.6}"); }

    // —— 生产路径冒烟：隐式 smooth_eps=Some(0.05) ——
    let impl_sm = simulate_implicit(&file, &mk(steps_ref, dt_ref),
        ImplicitOpts { rtol: 1e-9, atol: 1e-9, smooth_eps: Some(0.05) }).expect("隐式(0.05)求解");
    println!("\n隐式(smooth_eps=0.05·生产路径) vs 原样参照 |Δ|:");
    let mut sm_ok = true;
    for (i, s) in states.iter().enumerate() {
        let d = (impl_sm.final_value(s).unwrap() - ref_final[i]).abs();
        let rel = d / ref_final[i].abs().max(1e-9);
        let ok = rel < 5e-3; sm_ok &= ok;
        println!("  {s:9} |Δ|={d:.3e}  rel={rel:.2e}  {}", if ok { "✅" } else { "❌" });
    }

    // —— 显式 Euler 逐步细化，应收敛到隐式参照 ——
    println!("\n显式 Euler 收敛(各态 |显式−隐式None| 末值误差):");
    println!("  {:>8} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9}", "dt(s)", states[0], states[1], states[2], states[3], states[4], states[5]);
    let mut last_err = vec![f64::INFINITY; states.len()];
    for &dt in &[10.0f64, 5.0, 2.0, 1.0, 0.5] {
        let steps = (horizon / dt).round() as usize;
        let ex = simulate(&file, &mk(steps, dt)).expect("显式求解");
        let errs: Vec<f64> = states.iter().enumerate()
            .map(|(i, s)| (ex.final_value(s).unwrap() - ref_final[i]).abs()).collect();
        println!("  {:>8.3} | {:>9.2e} | {:>9.2e} | {:>9.2e} | {:>9.2e} | {:>9.2e} | {:>9.2e}",
            dt, errs[0], errs[1], errs[2], errs[3], errs[4], errs[5]);
        last_err = errs;
    }

    println!("\n判据(最细 dt=0.5s 相对误差 < 1e-3):");
    let mut all_ok = true;
    for (i, s) in states.iter().enumerate() {
        let scale = ref_final[i].abs().max(1e-9);
        let rel = last_err[i] / scale;
        let ok = rel < 1e-3; all_ok &= ok;
        println!("  {s:9} rel_err = {rel:.3e}  {}", if ok { "✅" } else { "❌" });
    }
    println!("\nV1 双路径(档2) {}   |   生产路径平滑冒烟 {}",
        if all_ok { "PASS ✅" } else { "FAIL ❌" },
        if sm_ok { "PASS ✅" } else { "FAIL ❌" });
    if !all_ok || !sm_ok { std::process::exit(1); }
}
