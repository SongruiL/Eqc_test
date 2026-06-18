//! 轨迹折线图：把 [`SimOutput`] 的若干变量渲染成自包含 SVG 折线图。
//!
//! 与 Forrester/DAG 的 SVG 一样，由 EQC（Rust）**自生成**——零第三方图表库、离线。
//! 前端只负责显示这张 SVG（`<img src="/api/chart.svg?vars=Y,TDM">`），不重画。
//! 交互（hover/缩放）将来如需要再叠加，但默认这张静态图已能看「整季趋势」。

use crate::sim::SimOutput;

/// 折线配色（最多 8 条循环）。
const COLORS: &[&str] = &[
    "#2563eb", "#f97316", "#16a34a", "#dc2626", "#9333ea", "#0891b2", "#ca8a04", "#db2777",
];

/// 把选定变量画成折线图 SVG。`vars` 里不存在的变量被跳过。
pub fn line_chart_svg(out: &SimOutput, vars: &[&str], width: f64, height: f64) -> String {
    let series: Vec<(&str, &[f64])> = vars
        .iter()
        .filter_map(|v| out.series(v).map(|s| (*v, s)))
        .collect();
    let n = out.steps;

    if series.is_empty() || n == 0 {
        return format!(
            "<svg viewBox=\"0 0 {width:.0} {height:.0}\" class=\"chart-svg\" xmlns=\"http://www.w3.org/2000/svg\">\
             <text x=\"{:.0}\" y=\"{:.0}\" font-size=\"13\" fill=\"#6b7280\" text-anchor=\"middle\">（无可绘制数据——选择变量，或用 --drivers 提供驱动量）</text></svg>",
            width / 2.0,
            height / 2.0
        );
    }

    // y 轴范围
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for (_, s) in &series {
        for &y in s.iter() {
            if y.is_finite() {
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
    }
    if !ymin.is_finite() {
        ymin = 0.0;
        ymax = 1.0;
    }
    if (ymax - ymin).abs() < 1e-12 {
        ymax = ymin + 1.0;
    }
    let pad = (ymax - ymin) * 0.05;
    ymin -= pad;
    ymax += pad;

    let (ml, mr, mt, mb) = (58.0, 16.0, 18.0, 38.0);
    let pw = (width - ml - mr).max(1.0);
    let ph = (height - mt - mb).max(1.0);
    let xmap = |i: usize| ml + (i as f64) / ((n - 1).max(1) as f64) * pw;
    let ymap = |y: f64| mt + (ymax - y) / (ymax - ymin) * ph;

    let mut s = format!(
        "<svg viewBox=\"0 0 {width:.0} {height:.0}\" class=\"chart-svg\" xmlns=\"http://www.w3.org/2000/svg\">"
    );

    // y 网格 + 刻度
    for k in 0..=4 {
        let yv = ymin + (ymax - ymin) * (k as f64 / 4.0);
        let yy = ymap(yv);
        s.push_str(&format!(
            "<line x1=\"{ml:.0}\" y1=\"{yy:.1}\" x2=\"{:.1}\" y2=\"{yy:.1}\" stroke=\"#eef2ff\" stroke-width=\"1\"/>",
            ml + pw
        ));
        s.push_str(&format!(
            "<text x=\"{:.0}\" y=\"{:.1}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"end\">{}</text>",
            ml - 6.0,
            yy + 3.0,
            fmt(yv)
        ));
    }
    // x 网格 + 刻度（约 6 个）
    let xticks = 6.min(n);
    for k in 0..xticks {
        let i = if xticks <= 1 { 0 } else { k * (n - 1) / (xticks - 1) };
        let xx = xmap(i);
        s.push_str(&format!(
            "<line x1=\"{xx:.1}\" y1=\"{mt:.0}\" x2=\"{xx:.1}\" y2=\"{:.1}\" stroke=\"#f3f4f6\" stroke-width=\"1\"/>",
            mt + ph
        ));
        s.push_str(&format!(
            "<text x=\"{xx:.1}\" y=\"{:.0}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"middle\">{}</text>",
            mt + ph + 14.0,
            i + 1
        ));
    }
    // 坐标轴线
    s.push_str(&format!(
        "<line x1=\"{ml:.0}\" y1=\"{mt:.0}\" x2=\"{ml:.0}\" y2=\"{:.1}\" stroke=\"#cbd5e1\"/>",
        mt + ph
    ));
    s.push_str(&format!(
        "<line x1=\"{ml:.0}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#cbd5e1\"/>",
        mt + ph,
        ml + pw,
        mt + ph
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.0}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"middle\">DAT (天)</text>",
        ml + pw / 2.0,
        height - 4.0
    ));

    // 折线 + 图例
    for (idx, (name, data)) in series.iter().enumerate() {
        let color = COLORS[idx % COLORS.len()];
        let pts: String = data
            .iter()
            .enumerate()
            .filter(|(_, y)| y.is_finite())
            .map(|(i, y)| format!("{:.1},{:.1}", xmap(i), ymap(*y)))
            .collect::<Vec<_>>()
            .join(" ");
        s.push_str(&format!(
            "<polyline points=\"{pts}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.8\"/>"
        ));
        let lx = ml + 8.0 + idx as f64 * 96.0;
        let ly = mt + 4.0;
        s.push_str(&format!(
            "<rect x=\"{lx:.0}\" y=\"{ly:.0}\" width=\"10\" height=\"10\" fill=\"{color}\"/>\
             <text x=\"{:.0}\" y=\"{:.0}\" font-size=\"10\" fill=\"#374151\">{}</text>",
            lx + 14.0,
            ly + 9.0,
            xml_escape(name)
        ));
    }

    s.push_str("</svg>");
    s
}

fn fmt(v: f64) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 1.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.3}")
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn out(series: &[(&str, Vec<f64>)]) -> SimOutput {
        let mut traj = IndexMap::new();
        let mut steps = 0;
        for (n, v) in series {
            steps = v.len();
            traj.insert(n.to_string(), v.clone());
        }
        SimOutput { steps, trajectories: traj }
    }

    #[test]
    fn test_line_chart_basic() {
        let o = out(&[("Y", vec![0.0, 1.0, 2.0, 3.0]), ("TDM", vec![10.0, 12.0, 14.0, 16.0])]);
        let svg = line_chart_svg(&o, &["Y", "TDM"], 640.0, 320.0);
        assert!(svg.contains("<svg"));
        assert_eq!(svg.matches("<polyline").count(), 2, "两条折线");
        assert!(svg.contains("DAT"));
    }

    #[test]
    fn test_line_chart_empty() {
        let o = out(&[("Y", vec![1.0, 2.0])]);
        let svg = line_chart_svg(&o, &["NotThere"], 640.0, 320.0);
        assert!(svg.contains("无可绘制数据"));
    }
}
