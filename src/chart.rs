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
    // 收集要画的序列：标量变量直接取；向量变量（轨迹里展平成 name[1]/name[2]…）展开成多条分量线。
    let mut series: Vec<(String, &[f64])> = Vec::new();
    for v in vars {
        if let Some(d) = out.series(v) {
            series.push((v.to_string(), d));
        } else {
            let prefix = format!("{v}[");
            for (k, d) in &out.trajectories {
                if k.starts_with(&prefix) {
                    series.push((k.clone(), d.as_slice()));
                }
            }
        }
    }
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

/// 把 DE 收敛轨迹（每代「至今最优代价」）画成 SVG：x=代数、y=代价（越小越好）。
/// 与 [`line_chart_svg`] 同一套自生成 SVG 风格（零图表库、离线）。
pub fn convergence_chart_svg(history: &[f64], width: f64, height: f64) -> String {
    if history.is_empty() {
        return format!(
            "<svg viewBox=\"0 0 {width:.0} {height:.0}\" class=\"chart-svg\" xmlns=\"http://www.w3.org/2000/svg\">\
             <text x=\"{:.0}\" y=\"{:.0}\" font-size=\"13\" fill=\"#6b7280\" text-anchor=\"middle\">（无收敛数据）</text></svg>",
            width / 2.0,
            height / 2.0
        );
    }
    let n = history.len();

    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for &y in history {
        if y.is_finite() {
            ymin = ymin.min(y);
            ymax = ymax.max(y);
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

    let (ml, mr, mt, mb) = (66.0, 16.0, 18.0, 38.0);
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
    // x 刻度（约 6 个，代数从 0 起）
    let xticks = 6.min(n);
    for k in 0..xticks {
        let i = if xticks <= 1 { 0 } else { k * (n - 1) / (xticks - 1) };
        let xx = xmap(i);
        s.push_str(&format!(
            "<line x1=\"{xx:.1}\" y1=\"{mt:.0}\" x2=\"{xx:.1}\" y2=\"{:.1}\" stroke=\"#f3f4f6\" stroke-width=\"1\"/>",
            mt + ph
        ));
        s.push_str(&format!(
            "<text x=\"{xx:.1}\" y=\"{:.0}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"middle\">{i}</text>",
            mt + ph + 14.0
        ));
    }
    // 坐标轴
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
        "<text x=\"{:.1}\" y=\"{:.0}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"middle\">代数 (generation)</text>",
        ml + pw / 2.0,
        height - 4.0
    ));

    // 收敛折线
    let pts: String = history
        .iter()
        .enumerate()
        .filter(|(_, y)| y.is_finite())
        .map(|(i, y)| format!("{:.1},{:.1}", xmap(i), ymap(*y)))
        .collect::<Vec<_>>()
        .join(" ");
    s.push_str(&format!(
        "<polyline points=\"{pts}\" fill=\"none\" stroke=\"#2563eb\" stroke-width=\"1.8\"/>"
    ));
    s.push_str(&format!(
        "<text x=\"{:.0}\" y=\"{:.0}\" font-size=\"10\" fill=\"#374151\">代价（越小越好）</text>",
        ml + 8.0,
        mt + 12.0
    ));

    s.push_str("</svg>");
    s
}

/// 把多目标 Pareto 前沿画成散点图 SVG：x=目标1、y=目标2，点按目标1 连成权衡曲线。
/// 每个点带 `data-i="{原始下标}"` + class `pp`，供前端点选（叠加该点轨迹）。内联 SVG（非 `<img>`）。
pub fn pareto_chart_svg(points: &[(f64, f64)], xlabel: &str, ylabel: &str, width: f64, height: f64) -> String {
    if points.is_empty() {
        return format!(
            "<svg viewBox=\"0 0 {width:.0} {height:.0}\" class=\"chart-svg\" xmlns=\"http://www.w3.org/2000/svg\">\
             <text x=\"{:.0}\" y=\"{:.0}\" font-size=\"13\" fill=\"#6b7280\" text-anchor=\"middle\">（无前沿点）</text></svg>",
            width / 2.0,
            height / 2.0
        );
    }
    let (mut xmin, mut xmax, mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for &(x, y) in points {
        if x.is_finite() {
            xmin = xmin.min(x);
            xmax = xmax.max(x);
        }
        if y.is_finite() {
            ymin = ymin.min(y);
            ymax = ymax.max(y);
        }
    }
    if !xmin.is_finite() {
        xmin = 0.0;
        xmax = 1.0;
    }
    if !ymin.is_finite() {
        ymin = 0.0;
        ymax = 1.0;
    }
    if (xmax - xmin).abs() < 1e-12 {
        xmax = xmin + 1.0;
    }
    if (ymax - ymin).abs() < 1e-12 {
        ymax = ymin + 1.0;
    }
    let (xpad, ypad) = ((xmax - xmin) * 0.05, (ymax - ymin) * 0.05);
    xmin -= xpad;
    xmax += xpad;
    ymin -= ypad;
    ymax += ypad;

    let (ml, mr, mt, mb) = (70.0, 16.0, 18.0, 40.0);
    let pw = (width - ml - mr).max(1.0);
    let ph = (height - mt - mb).max(1.0);
    let xmap = |x: f64| ml + (x - xmin) / (xmax - xmin) * pw;
    let ymap = |y: f64| mt + (ymax - y) / (ymax - ymin) * ph;

    let mut s = format!(
        "<svg viewBox=\"0 0 {width:.0} {height:.0}\" class=\"chart-svg pareto-svg\" xmlns=\"http://www.w3.org/2000/svg\">"
    );
    // 网格 + 刻度（各 4 段）
    for k in 0..=4 {
        let yv = ymin + (ymax - ymin) * (k as f64 / 4.0);
        let yy = ymap(yv);
        s.push_str(&format!(
            "<line x1=\"{ml:.0}\" y1=\"{yy:.1}\" x2=\"{:.1}\" y2=\"{yy:.1}\" stroke=\"#eef2ff\"/>\
             <text x=\"{:.0}\" y=\"{:.1}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"end\">{}</text>",
            ml + pw, ml - 6.0, yy + 3.0, fmt(yv)
        ));
        let xv = xmin + (xmax - xmin) * (k as f64 / 4.0);
        let xx = xmap(xv);
        s.push_str(&format!(
            "<line x1=\"{xx:.1}\" y1=\"{mt:.0}\" x2=\"{xx:.1}\" y2=\"{:.1}\" stroke=\"#f3f4f6\"/>\
             <text x=\"{xx:.1}\" y=\"{:.0}\" font-size=\"10\" fill=\"#6b7280\" text-anchor=\"middle\">{}</text>",
            mt + ph, mt + ph + 14.0, fmt(xv)
        ));
    }
    // 轴 + 标签
    s.push_str(&format!(
        "<line x1=\"{ml:.0}\" y1=\"{mt:.0}\" x2=\"{ml:.0}\" y2=\"{:.1}\" stroke=\"#cbd5e1\"/>\
         <line x1=\"{ml:.0}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#cbd5e1\"/>",
        mt + ph, mt + ph, ml + pw, mt + ph
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.0}\" font-size=\"10\" fill=\"#374151\" text-anchor=\"middle\">{}</text>",
        ml + pw / 2.0, height - 4.0, xml_escape(xlabel)
    ));
    s.push_str(&format!(
        "<text x=\"12\" y=\"{:.1}\" font-size=\"10\" fill=\"#374151\" text-anchor=\"middle\" transform=\"rotate(-90 12 {:.1})\">{}</text>",
        mt + ph / 2.0, mt + ph / 2.0, xml_escape(ylabel)
    ));

    // 连线（按目标1 升序）
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&a, &b| points[a].0.partial_cmp(&points[b].0).unwrap_or(std::cmp::Ordering::Equal));
    let line: String = order
        .iter()
        .filter(|&&i| points[i].0.is_finite() && points[i].1.is_finite())
        .map(|&i| format!("{:.1},{:.1}", xmap(points[i].0), ymap(points[i].1)))
        .collect::<Vec<_>>()
        .join(" ");
    s.push_str(&format!(
        "<polyline points=\"{line}\" fill=\"none\" stroke=\"#93c5fd\" stroke-width=\"1.4\"/>"
    ));
    // 散点（data-i = 原始下标，供前端点选）
    for (i, &(x, y)) in points.iter().enumerate() {
        if x.is_finite() && y.is_finite() {
            s.push_str(&format!(
                "<circle class=\"pp\" data-i=\"{i}\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" fill=\"#2563eb\" stroke=\"#fff\" stroke-width=\"1\"/>",
                xmap(x), ymap(y)
            ));
        }
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

    #[test]
    fn test_convergence_chart() {
        let svg = convergence_chart_svg(&[-8.0, -9.0, -10.0, -10.5], 720.0, 300.0);
        assert!(svg.contains("<svg"));
        assert_eq!(svg.matches("<polyline").count(), 1);
        assert!(svg.contains("代数"));
    }

    #[test]
    fn test_convergence_chart_empty() {
        let svg = convergence_chart_svg(&[], 720.0, 300.0);
        assert!(svg.contains("无收敛数据"));
    }

    #[test]
    fn test_pareto_chart() {
        let pts = vec![(7.5, 96000.0), (9.0, 180000.0), (10.95, 288000.0)];
        let svg = pareto_chart_svg(&pts, "产量 Y", "CO2 用量", 640.0, 360.0);
        assert!(svg.contains("<svg"));
        assert_eq!(svg.matches("class=\"pp\"").count(), 3, "三个可点选前沿点");
        assert!(svg.contains("data-i=\"0\"") && svg.contains("data-i=\"2\""));
        assert_eq!(svg.matches("<polyline").count(), 1, "一条连线");
    }

    #[test]
    fn test_pareto_chart_empty() {
        assert!(pareto_chart_svg(&[], "x", "y", 640.0, 360.0).contains("无前沿点"));
    }
}
