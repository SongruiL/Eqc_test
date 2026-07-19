//! V1 主验收（Tier B）：在**真实 van Henten 三态温室模型**上验「显式↔隐式一致性」。
//!
//! 加载 `greenhouse_v1.eq.yaml`（带手写 `_prev`），跑：
//!   - 隐式 BDF（`simulate_implicit` 自动 E5a 折叠 `_prev`、解真联立）→ 近精确参照；
//!   - 显式 Euler（原模型，`simulate`）在逐步细化的 dt 下 → 应随 dt→0 收敛到隐式。
//! 常数驱动（一个冷晴天）以隔离求解器误差（去掉驱动离散化混淆）。
//!
//! 运行：`cargo run --features implicit --example v1_greenhouse`

use std::collections::HashMap;
use std::path::Path;

use equation_compiler::parse_file;
use equation_compiler::sim::implicit::{simulate_implicit, ImplicitOpts};
use equation_compiler::sim::{simulate, SimInput};

fn const_drivers(names: &[&str], vals: &[f64], steps: usize) -> HashMap<String, Vec<f64>> {
    names
        .iter()
        .zip(vals)
        .map(|(n, &v)| (n.to_string(), vec![v; steps]))
        .collect()
}

fn main() {
    let path = "/root/Maestro/greenhouse_model/greenhouse_v1.eq.yaml";
    let file = parse_file(Path::new(path)).expect("解析温室模型");
    println!("模型: {} ({})  dt={}s", file.meta.id, file.meta.name_cn, file.meta.dt);

    // 常数驱动：冷晴天。驱动名取自模型 driving 变量。
    let drv_names = ["T_out", "I_glob"];
    let drv_vals = [5.0, 300.0]; // 室外 5℃，全球辐射 300 W/m²
    let states = ["T_air", "CO2_air", "H_air"];

    let horizon = 1800.0_f64; // 30 分钟

    // —— 隐式（近精确参照）：模型 dt=10s，紧容差 ——
    let dt_ref = file.meta.dt; // 10s
    let steps_ref = (horizon / dt_ref).round() as usize;
    let mut inp = SimInput {
        steps: steps_ref,
        dt: Some(dt_ref),
        drivers: const_drivers(&drv_names, &drv_vals, steps_ref),
        ..Default::default()
    };
    let impl_out = simulate_implicit(&file, &inp, ImplicitOpts { rtol: 1e-8, atol: 1e-8, smooth_eps: None })
        .expect("隐式求解");
    let ref_final: Vec<f64> = states.iter().map(|s| impl_out.final_value(s).unwrap()).collect();

    println!("\n隐式 BDF 参照（t={horizon}s 末值）:");
    for (s, v) in states.iter().zip(&ref_final) {
        println!("  {s:9} = {v:.6}");
    }

    // —— 显式 Euler 逐步细化，应收敛到隐式 ——
    println!("\n显式 Euler 收敛（各态 |显式−隐式| 末值误差）:");
    println!(
        "  {:>8} | {:>10} | {:>10} | {:>10}",
        "dt(s)", states[0], states[1], states[2]
    );
    let mut last_err = [f64::INFINITY; 3];
    for &dt in &[10.0f64, 5.0, 2.0, 1.0, 0.5] {
        let steps = (horizon / dt).round() as usize;
        inp = SimInput {
            steps,
            dt: Some(dt),
            drivers: const_drivers(&drv_names, &drv_vals, steps),
            ..Default::default()
        };
        let ex = simulate(&file, &inp).expect("显式求解");
        let errs: Vec<f64> = states
            .iter()
            .enumerate()
            .map(|(i, s)| (ex.final_value(s).unwrap() - ref_final[i]).abs())
            .collect();
        println!(
            "  {:>8.3} | {:>10.3e} | {:>10.3e} | {:>10.3e}",
            dt, errs[0], errs[1], errs[2]
        );
        for i in 0..3 {
            last_err[i] = errs[i];
        }
    }

    // 判据：最细 dt 显式与隐式一致（各态相对误差 < 1e-3）
    println!("\n判据（最细 dt=0.5s 相对误差 < 1e-3）:");
    let mut all_ok = true;
    for (i, s) in states.iter().enumerate() {
        let scale = ref_final[i].abs().max(1e-9);
        let rel = last_err[i] / scale;
        let ok = rel < 1e-3;
        all_ok &= ok;
        println!("  {s:9} rel_err = {rel:.3e}  {}", if ok { "✅" } else { "❌" });
    }
    println!("\nV1 (真实三态) {}", if all_ok { "PASS ✅" } else { "FAIL ❌" });
    if !all_ok {
        std::process::exit(1);
    }
}
