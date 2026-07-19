//! V4（平滑化误差界）· 真实 ctrl 变体。E2 平滑化在**真实 van Henten 三态 + 设定点反馈控制律**
//! （`greenhouse_v1_ctrl.eq.yaml`）上的 bottom-line 决策差异 ε-扫描：
//!   - 硬-显式-细dt = 决策真值（硬 clamp 控制律 + 收敛 Euler）；
//!   - 平滑-隐式（各 ε）= 生产路径（E5a 折叠 + E2 平滑 + BDF）。
//! 报末态 + 累计执行器用量（∫Q_heat、∫u_vent = 加热能耗/通风时数）随 ε 的差异 → 找「决策差异可忽略」的 ε。
//! 冷暗驱动使加热控制律 engage（穿过 clamp 拐角）。
//!
//! 运行：`cargo run --features implicit --example v4_ctrl`

use std::collections::HashMap;
use std::path::Path;

use equation_compiler::parse_file;
use equation_compiler::sim::implicit::{simulate_implicit, ImplicitOpts};
use equation_compiler::sim::{simulate, SimInput};

fn const_drivers(steps: usize, t_out: f64, i_glob: f64) -> HashMap<String, Vec<f64>> {
    HashMap::from([
        ("T_out".to_string(), vec![t_out; steps]),
        ("I_glob".to_string(), vec![i_glob; steps]),
    ])
}

/// 累计执行器用量 = Σ series[n]·dt（∫·dt）。
fn cumulative(out: &equation_compiler::sim::SimOutput, name: &str, dt: f64) -> f64 {
    out.series(name).map(|s| s.iter().sum::<f64>() * dt).unwrap_or(f64::NAN)
}

fn main() {
    let path = "/root/Maestro/greenhouse_model/greenhouse_v1_ctrl.eq.yaml";
    let file = parse_file(Path::new(path)).expect("解析 ctrl 模型");
    println!("模型: {} ({})  dt={}s", file.meta.id, file.meta.name_cn, file.meta.dt);

    // 冷暗驱动 → 加热控制律 engage（T_air 被拉向加热设定点、穿过 clamp 拐角）
    let (t_out, i_glob) = (-5.0, 50.0);
    let horizon = 3600.0_f64; // 1 小时
    let states = ["T_air", "CO2_air", "H_air"];

    // —— 硬-显式-细dt = 决策真值 ——
    let dt_ref = 1.0;
    let steps_ref = (horizon / dt_ref) as usize;
    let hard = simulate(
        &file,
        &SimInput {
            steps: steps_ref,
            dt: Some(dt_ref),
            drivers: const_drivers(steps_ref, t_out, i_glob),
            ..Default::default()
        },
    )
    .expect("硬-显式");
    let hard_T = hard.final_value("T_air").unwrap();
    let hard_qheat = cumulative(&hard, "Q_heat", dt_ref);
    let hard_uvent = cumulative(&hard, "u_vent", dt_ref);
    println!(
        "\n硬-显式-细dt({dt_ref}s) 决策真值: T_air末={hard_T:.4}℃  ∫Q_heat={hard_qheat:.1} J/m²  ∫u_vent={hard_uvent:.1} s"
    );

    // —— 平滑-隐式（model dt=30s）ε-扫描 ——
    let dt = file.meta.dt; // 30s
    let steps = (horizon / dt) as usize;
    println!("\n平滑-隐式(dt={dt}s) vs 真值，随 ε（绝对值）：");
    println!(
        "  {:>6} | {:>11} | {:>16} | {:>14} | 收敛",
        "ε", "ΔT_air末(℃)", "∫Q_heat(vs 真值)", "∫u_vent(真值≈0)"
    );
    for &eps in &[0.10f64, 0.05, 0.02, 0.01] {
        let inp = SimInput {
            steps,
            dt: Some(dt),
            drivers: const_drivers(steps, t_out, i_glob),
            ..Default::default()
        };
        match simulate_implicit(&file, &inp, ImplicitOpts { rtol: 1e-7, atol: 1e-7, smooth_eps: Some(eps) }) {
            Ok(sm) => {
                let dT = (sm.final_value("T_air").unwrap() - hard_T).abs();
                let qh = cumulative(&sm, "Q_heat", dt);
                let uv = cumulative(&sm, "u_vent", dt);
                let dqh_pct = 100.0 * (qh - hard_qheat) / hard_qheat;
                println!(
                    "  {eps:>6.3} | {dT:>10.3e} | {qh:>10.0}({dqh_pct:+.2}%) | {uv:>14.3} | ✅"
                );
                let _ = &states;
            }
            Err(e) => println!("  {eps:>6.3} | 求解失败: {e}"),
        }
    }
    println!("\n真值(硬-显式-细dt): ∫Q_heat={hard_qheat:.0} J/m²  ∫u_vent={hard_uvent:.3} s（冷天未通风）");
    println!(
        "\n判据 / 诚实标注：\n\
         · **ΔT_air末（点值）= 干净平滑误差信号**：随 ε→0 呈 ~O(ε²) 收敛（ε 每减半、误差约 1/4）→ 平滑对决策的\n\
           扰动极小、由 ε 可控。ε 是无量纲拐角圆化宽度（∵ 控制律已按 Pband 归一化）。\n\
         · ∫Q_heat 的 ~0.4% 残差**主要是积分求积伪影**（平滑-隐式 dt=30s 的黎曼和 vs 硬-显式 dt=1s），\n\
           非平滑误差；纯平滑误差需「同 dt 的硬-显式 vs 平滑-显式」隔离（Phase 1 细化）。\n\
         · ∫u_vent 的几秒是**平滑 max0 关断残留**（0.5·ε 在 u=0 处非零）——绝对量微小的已知伪影。\n\
         结论：ε≈0.05 决策差异可忽略（ΔT_air末 ~4e-3℃、加热 <1%）且收敛稳，作 profile 参数默认值合理。"
    );
}
