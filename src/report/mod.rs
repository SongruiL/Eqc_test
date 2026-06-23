//! 自包含 HTML 模型报告：DAG（EQC 自生成 SVG）+ 每个方程的二维公式（MathML，
//! 浏览器原生渲染）。零第三方 JS、完全离线。
//!
//! 入口：[`generate_report`] / [`generate_report_with`]。CLI 子命令 `eqc report`。

mod layout;
pub use layout::LayoutKind;
use layout::{compute as compute_layout, Geom};

use crate::ast::Expr;
use crate::dag::{Dag, DagLevel};
use crate::schema::{EquationFile, VarClass};
use std::collections::{HashMap, HashSet};

/// XML/HTML 文本转义。
fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// 数字格式化：整数不带小数点。
fn fmt_num(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

// ============================================
// MathML 生成
// ============================================

/// 变量/参数名 -> MathML 标识符（含 `_` 下标支持，如 `Pmax_l`）。
fn ident(name: &str) -> String {
    if let Some((base, sub)) = name.split_once('_') {
        format!("<msub><mi>{}</mi><mi>{}</mi></msub>", xml(base), xml(sub))
    } else {
        format!("<mi>{}</mi>", xml(name))
    }
}

/// 作为「因子」渲染：若是加减则加括号（避免歧义）。
fn factor(e: &Expr) -> String {
    let needs = matches!(crate::ops::as_operator(e), Some((n, _)) if matches!(n, "add" | "sub"));
    if needs {
        format!("<mrow><mo>(</mo>{}<mo>)</mo></mrow>", mml(e))
    } else {
        mml(e)
    }
}

fn func1(name: &str, a: &Expr) -> String {
    format!("<mrow><mi>{name}</mi><mo>(</mo>{}<mo>)</mo></mrow>", mml(a))
}
fn func2(name: &str, a: &Expr, b: &Expr) -> String {
    format!("<mrow><mi>{name}</mi><mo>(</mo>{}<mo>,</mo>{}<mo>)</mo></mrow>", mml(a), mml(b))
}
fn funcn(name: &str, args: &[&Expr]) -> String {
    let inner: Vec<String> = args.iter().map(|a| mml(a)).collect();
    format!("<mrow><mi>{name}</mi><mo>(</mo>{}<mo>)</mo></mrow>", inner.join("<mo>,</mo>"))
}

/// 中缀二元运算。
fn infix(args: &[&Expr], op: &str) -> String {
    format!("<mrow>{}<mo>{op}</mo>{}</mrow>", mml(args[0]), mml(args[1]))
}

/// 注册表算子（52 个）的 MathML。
fn op_mml(name: &str, args: &[&Expr]) -> String {
    match name {
        "add" => infix(args, "+"),
        "sub" => infix(args, "&#8722;"),
        "mul" => format!("<mrow>{}<mo>&#183;</mo>{}</mrow>", factor(args[0]), factor(args[1])),
        "div" => format!("<mfrac><mrow>{}</mrow><mrow>{}</mrow></mfrac>", mml(args[0]), mml(args[1])),
        "neg" => format!("<mrow><mo>&#8722;</mo>{}</mrow>", factor(args[0])),
        "abs" => format!("<mrow><mo>|</mo>{}<mo>|</mo></mrow>", mml(args[0])),
        "pow" => format!("<msup>{}<mrow>{}</mrow></msup>", factor(args[0]), mml(args[1])),
        "sqrt" => format!("<msqrt>{}</msqrt>", mml(args[0])),
        "mod" => format!("<mrow>{}<mo>&#8201;mod&#8201;</mo>{}</mrow>", mml(args[0]), mml(args[1])),
        "eq" => infix(args, "="),
        "lt" => infix(args, "&lt;"),
        "gt" => infix(args, "&gt;"),
        "leq" => infix(args, "&#8804;"),
        "geq" => infix(args, "&#8805;"),
        "neq" => infix(args, "&#8800;"),
        "and" => infix(args, "&#8743;"),
        "or" => infix(args, "&#8744;"),
        "not" => format!("<mrow><mo>&#172;</mo>{}</mrow>", mml(args[0])),
        // 其余（三角/双曲/exp/log…）-> 函数形式
        _ => funcn(name, args),
    }
}

/// 求和/连乘的 MathML（带上下限）。
fn bigop(sym: &str, index: &str, lower: &Expr, upper: &Expr, body: &Expr) -> String {
    format!(
        "<mrow><munderover><mo>{sym}</mo><mrow><mi>{}</mi><mo>=</mo>{}</mrow><mrow>{}</mrow></munderover>{}</mrow>",
        xml(index),
        mml(lower),
        mml(upper),
        factor(body)
    )
}

/// 分段函数 -> cases 表。
fn piecewise_mml(pieces: &[(Expr, Expr)], otherwise: &Expr) -> String {
    let mut rows = String::new();
    for (cond, val) in pieces {
        rows.push_str(&format!(
            "<mtr><mtd>{}</mtd><mtd><mtext>&#8201;若&#8201;</mtext>{}</mtd></mtr>",
            mml(val),
            mml(cond)
        ));
    }
    rows.push_str(&format!(
        "<mtr><mtd>{}</mtd><mtd><mtext>&#8201;其他</mtext></mtd></mtr>",
        mml(otherwise)
    ));
    format!("<mrow><mo>{{</mo><mtable columnalign=\"left\">{rows}</mtable></mrow>")
}

/// 把表达式渲染成 MathML 字符串（含外层 `<math>`）。供 JSON 契约 / 前端复用。
pub fn expr_mathml(e: &Expr) -> String {
    format!("<math display=\"block\"><mrow>{}</mrow></math>", mml(e))
}

/// Expr -> MathML（不含外层 `<math>`）。
fn mml(e: &Expr) -> String {
    match e {
        Expr::Const(v) => format!("<mn>{}</mn>", fmt_num(*v)),
        Expr::Pi => "<mi>&#960;</mi>".to_string(),
        Expr::E => "<mi>e</mi>".to_string(),
        Expr::Var(n) | Expr::Param(n) => ident(n),

        Expr::Sum { index, lower, upper, body } => bigop("&#8721;", index, lower, upper, body),
        Expr::Product { index, lower, upper, body } => bigop("&#8719;", index, lower, upper, body),
        Expr::Piecewise { pieces, otherwise } => piecewise_mml(pieces, otherwise),
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            // 渲染成两分支的 cases
            piecewise_mml(
                std::slice::from_ref(&((**cond).clone(), (**then_branch).clone())),
                else_branch,
            )
        }
        Expr::Max(xs) => funcn("max", &xs.iter().collect::<Vec<_>>()),
        Expr::Min(xs) => funcn("min", &xs.iter().collect::<Vec<_>>()),

        // 常见特殊函数（二维传统记号）
        Expr::Gamma(x) => func1("&#915;", x),         // Γ
        Expr::Lgamma(x) => func1("ln&#915;", x),
        Expr::Digamma(x) => func1("&#968;", x),       // ψ
        Expr::Beta(a, b) => func2("B", a, b),
        Expr::Lbeta(a, b) => func2("lnB", a, b),
        Expr::Erf(x) => func1("erf", x),
        Expr::Erfc(x) => func1("erfc", x),
        Expr::Erfinv(x) => func1("erf&#8315;&#185;", x),
        Expr::Factorial(n) => format!("<mrow>{}<mo>!</mo></mrow>", factor(n)),
        Expr::Logit(x) => func1("logit", x),
        Expr::Expit(x) => func1("&#963;", x),         // σ
        Expr::NormPdf(x, m, s) => funcn("&#966;", &[x, m, s]), // φ
        Expr::NormCdf(x, m, s) => funcn("&#934;", &[x, m, s]), // Φ
        Expr::NormPpf(p, m, s) => funcn("&#934;&#8315;&#185;", &[p, m, s]),

        other => {
            if let Some((name, args)) = crate::ops::as_operator(other) {
                op_mml(name, &args)
            } else {
                format!("<mtext>{}</mtext>", xml(&variant_name(other)))
            }
        }
    }
}

/// 取变体名（用于未支持算子的占位）。
fn variant_name(e: &Expr) -> String {
    let dbg = format!("{e:?}");
    dbg.split(|c| c == '(' || c == '{' || c == ' ')
        .next()
        .unwrap_or("?")
        .to_string()
}

// ============================================
// DAG -> SVG（分层布局）
// ============================================

/// 一条边的 SVG path `d`：分层布局用上下贝塞尔（流向朝下）；自由布局（力导向/Forrester）
/// 用「框边到框边」的微弯曲线，端点裁剪到各自节点框的边界（箭头正好落在框上，不被遮）。
fn edge_path(x1: f64, y1: f64, x2: f64, y2: f64, bw: f64, bh: f64, kind: LayoutKind) -> String {
    if !kind.free_edges() {
        // 分层：从 from 底边到 to 顶边
        let (sx, sy) = (x1 + bw / 2.0, y1 + bh);
        let (tx, ty) = (x2 + bw / 2.0, y2);
        return format!(
            "M{sx:.0},{sy:.0} C{sx:.0},{:.0} {tx:.0},{:.0} {tx:.0},{ty:.0}",
            sy + 32.0,
            ty - 32.0
        );
    }
    // 自由方向：中心到中心，端点裁到框边，中点沿法向外凸做微弯
    let (hw, hh) = (bw / 2.0, bh / 2.0);
    let (c1x, c1y) = (x1 + hw, y1 + hh);
    let (c2x, c2y) = (x2 + hw, y2 + hh);
    let (sx, sy) = box_exit(c1x, c1y, hw, hh, c2x, c2y);
    let (tx, ty) = box_exit(c2x, c2y, hw, hh, c1x, c1y);
    let (ex, ey) = (tx - sx, ty - sy);
    let len = (ex * ex + ey * ey).sqrt().max(1.0);
    let (nx, ny) = (-ey / len, ex / len);
    let bow = (len * 0.12).min(26.0);
    let (mx, my) = ((sx + tx) / 2.0 + nx * bow, (sy + ty) / 2.0 + ny * bow);
    format!("M{sx:.0},{sy:.0} Q{mx:.0},{my:.0} {tx:.0},{ty:.0}")
}

/// 从框中心 `(cx,cy)` 朝 `(tx,ty)` 方向，求射线与框（半宽 `hw`、半高 `hh`）边界的交点。
fn box_exit(cx: f64, cy: f64, hw: f64, hh: f64, tx: f64, ty: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (cx, cy);
    }
    let sx = if dx.abs() < 1e-6 { f64::INFINITY } else { hw / dx.abs() };
    let sy = if dy.abs() < 1e-6 { f64::INFINITY } else { hh / dy.abs() };
    let t = sx.min(sy);
    (cx + dx * t, cy + dy * t)
}

/// 子模块配色：每个模块一个色（按名排序、色相均匀铺开）。节点上色 + 图例共用，保证一致。
fn module_palette(dag: &Dag) -> Vec<(String, String)> {
    let mut mods: Vec<String> = dag
        .nodes
        .iter()
        .map(|n| n.module.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    mods.sort();
    let n = mods.len().max(1);
    mods.into_iter()
        .enumerate()
        .map(|(i, m)| (m, format!("hsl({:.0}, 55%, 86%)", (i as f64) * 360.0 / (n as f64))))
        .collect()
}

/// 子模块色块图例（与 [`dag_svg`] 节点配色一致）。
fn module_legend(dag: &Dag) -> String {
    let mut s = String::from("<div class=\"legend\">");
    for (m, c) in module_palette(dag) {
        s.push_str(&format!("<span style=\"background:{c}\">{}</span>", xml(&m)));
    }
    s.push_str("</div>");
    s
}

fn dag_svg(files: &[EquationFile], dag: &Dag, kind: LayoutKind) -> String {
    if dag.nodes.is_empty() {
        return "<p class=\"empty\">（无节点）</p>".to_string();
    }
    let g = Geom { bw: 160.0, bh: 38.0, hgap: 24.0, vgap: 64.0 };
    let (bw, bh) = (g.bw, g.bh);
    let nodes: Vec<&str> = dag.nodes.iter().map(|n| n.id.as_str()).collect();
    let edges: Vec<(&str, &str)> =
        dag.edges.iter().map(|e| (e.from.as_str(), e.to.as_str())).collect();
    // Forrester 布局需节点分类（存量/速率/边界=主干）；角色 DAG 也走主干布局（含方程级）。
    // 模块级节点非变量 → class_of 落到辅助 → 主干不足 → compute_forrester 内部回退力导向。
    let class: std::collections::HashMap<&str, VarClass> =
        dag.nodes.iter().map(|n| (n.id.as_str(), class_of(files, &n.id))).collect();
    let lay = match kind {
        LayoutKind::Forrester => layout::compute_forrester(&nodes, &edges, &class, g),
        _ => compute_layout(&nodes, &edges, kind, g),
    };
    let pos = &lay.pos;

    let mut s = format!(
        "<svg viewBox=\"0 0 {width:.0} {height:.0}\" class=\"dag-svg\" xmlns=\"http://www.w3.org/2000/svg\">\
         <defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10 z\" fill=\"#888\"/></marker></defs>",
        width = lay.width,
        height = lay.height,
    );
    // 边
    for e in &dag.edges {
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (pos.get(e.from.as_str()), pos.get(e.to.as_str())) {
            let d = edge_path(x1, y1, x2, y2, bw, bh, kind);
            s.push_str(&format!(
                "<path d=\"{d}\" class=\"edge\" data-from=\"{}\" data-to=\"{}\" marker-end=\"url(#arrow)\"/>",
                xml(&e.from), xml(&e.to)
            ));
        }
    }
    // 模块配色（节点按子模块上色，与图例一致）
    let palette = module_palette(dag);
    let color: std::collections::HashMap<&str, &str> =
        palette.iter().map(|(m, c)| (m.as_str(), c.as_str())).collect();
    // 节点
    for n in &dag.nodes {
        let (x, y) = pos[n.id.as_str()];
        let cls = format!("{:?}", n.node_type).to_lowercase();
        let short = n.id.rsplit('.').next().unwrap_or(&n.id);
        // 显示名：友好标签（变量 label → 方程中文名 → 代号），过长截断；其余进 tooltip
        let raw = n.metadata.get("label").map(|s| s.as_str()).unwrap_or(short);
        let label = if raw.chars().count() > 13 {
            format!("{}…", raw.chars().take(12).collect::<String>())
        } else {
            raw.to_string()
        };
        let fill = color.get(n.module.as_str()).copied().unwrap_or("#eef2ff");
        s.push_str(&format!(
            "<g class=\"node {cls}\" data-var=\"{dv}\" data-id=\"{did}\" data-cx=\"{cx:.0}\" data-cy=\"{cy:.0}\" data-hw=\"{hw:.0}\" data-hh=\"{hh:.0}\">\
             <rect x=\"{x:.0}\" y=\"{y:.0}\" width=\"{bw:.0}\" height=\"{bh:.0}\" rx=\"7\" style=\"fill:{fill}\"/>\
             <text x=\"{tx:.0}\" y=\"{ty:.0}\">{lbl}</text></g>",
            dv = xml(short),
            did = xml(&n.id),
            cx = x + bw / 2.0,
            cy = y + bh / 2.0,
            hw = bw / 2.0,
            hh = bh / 2.0,
            tx = x + bw / 2.0,
            ty = y + bh / 2.0 + 5.0,
            lbl = xml(&label),
        ));
    }
    s.push_str("</svg>");
    s
}

// ============================================
// Forrester 库存-流量图 -> SVG
// ============================================
//
// 把变量按 Forrester 系统动力学分类渲染成不同图元：存量(矩形)、速率(六边阀门)、
// 驱动(椭圆)、参数(胶囊)、辅助(圆角矩形)、半状态(虚框矩形)、边界(梯形云)。
// 流分两类：**物质流**（速率→存量的积分管道，橙色粗线）与**信息流**（其余引用，灰色虚线）。
// 复用 DAG 的节点与数据流边，另补上 schema 里 rate/prev 蕴含的积分/延迟边。

/// 由节点 id（`MODULE.name`）查其 Forrester 分类。
fn class_of(files: &[EquationFile], node_id: &str) -> VarClass {
    let (module, name) = node_id.split_once('.').unwrap_or(("", node_id));
    for f in files {
        if f.meta.id == module {
            if f.parameters.contains_key(name) {
                return VarClass::Parameter;
            }
            if let Some(v) = f.variables.get(name) {
                return v.effective_class();
            }
        }
    }
    VarClass::Auxiliary
}

/// 分类 -> CSS 类名。
fn fclass_css(c: VarClass) -> &'static str {
    match c {
        VarClass::State => "state",
        VarClass::Rate => "rate",
        VarClass::Auxiliary => "auxiliary",
        VarClass::Driving => "driving",
        VarClass::Parameter => "parameter",
        VarClass::Control => "control",
        VarClass::SemiState => "semistate",
        VarClass::Boundary => "boundary",
    }
}

/// 按分类生成对应 SVG 图元（含 `fsh <css>` 类，便于 CSS 上色/描边）。
fn fnode_shape(c: VarClass, x: f64, y: f64, bw: f64, bh: f64) -> String {
    let css = fclass_css(c);
    let (cx, cy) = (x + bw / 2.0, y + bh / 2.0);
    match c {
        // 存量 / 半状态：直角矩形（半状态的虚框由 CSS 控制）
        VarClass::State | VarClass::SemiState => format!(
            "<rect class=\"fsh {css}\" x=\"{x:.0}\" y=\"{y:.0}\" width=\"{bw:.0}\" height=\"{bh:.0}\"/>"
        ),
        // 辅助 / 控制：圆角矩形
        VarClass::Auxiliary | VarClass::Control => format!(
            "<rect class=\"fsh {css}\" x=\"{x:.0}\" y=\"{y:.0}\" width=\"{bw:.0}\" height=\"{bh:.0}\" rx=\"9\"/>"
        ),
        // 参数：胶囊
        VarClass::Parameter => format!(
            "<rect class=\"fsh {css}\" x=\"{x:.0}\" y=\"{y:.0}\" width=\"{bw:.0}\" height=\"{bh:.0}\" rx=\"{:.0}\"/>",
            bh / 2.0
        ),
        // 驱动：椭圆
        VarClass::Driving => format!(
            "<ellipse class=\"fsh {css}\" cx=\"{cx:.0}\" cy=\"{cy:.0}\" rx=\"{:.0}\" ry=\"{:.0}\"/>",
            bw / 2.0,
            bh / 2.0
        ),
        // 速率：六边形阀门
        VarClass::Rate => format!(
            "<polygon class=\"fsh {css}\" points=\"{:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}\"/>",
            x + 12.0, y,
            x + bw - 12.0, y,
            x + bw, cy,
            x + bw - 12.0, y + bh,
            x + 12.0, y + bh,
            x, cy
        ),
        // 边界：梯形（源/汇）
        VarClass::Boundary => format!(
            "<polygon class=\"fsh {css}\" points=\"{:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}\"/>",
            x + 14.0, y,
            x + bw - 14.0, y,
            x + bw, y + bh,
            x, y + bh
        ),
    }
}

fn forrester_svg(files: &[EquationFile], dag: &Dag, kind: LayoutKind) -> String {
    if dag.nodes.is_empty() {
        return "<p class=\"empty\">（无节点）</p>".to_string();
    }
    let idset: HashSet<&str> = dag.nodes.iter().map(|n| n.id.as_str()).collect();

    // 节点分类
    let class: HashMap<&str, VarClass> =
        dag.nodes.iter().map(|n| (n.id.as_str(), class_of(files, &n.id))).collect();

    // 边：DAG 的数据流边（信息流）+ schema 蕴含的积分边（速率→存量，物质流）+ 延迟边（虚线信息流）
    let mut edges: Vec<(String, String, bool)> = Vec::new(); // (from, to, is_material)
    for e in &dag.edges {
        edges.push((e.from.clone(), e.to.clone(), false));
    }
    for f in files {
        for (name, v) in &f.variables {
            let to = format!("{}.{}", f.meta.id, name);
            if let Some(r) = &v.rate {
                edges.push((format!("{}.{}", f.meta.id, r), to.clone(), true)); // 积分=物质流
            }
            if let Some(p) = &v.prev {
                edges.push((format!("{}.{}", f.meta.id, p), to.clone(), false)); // 延迟=信息流
            }
        }
    }
    edges.retain(|(a, b, _)| idset.contains(a.as_str()) && idset.contains(b.as_str()));

    // 布局：复用 layout 模块（含速率→存量的积分边、延迟边一起参与排布）。
    let g = Geom { bw: 150.0, bh: 40.0, hgap: 26.0, vgap: 70.0 };
    let (bw, bh) = (g.bw, g.bh);
    let nodes: Vec<&str> = dag.nodes.iter().map(|n| n.id.as_str()).collect();
    let layout_edges: Vec<(&str, &str)> =
        edges.iter().map(|(a, b, _)| (a.as_str(), b.as_str())).collect();
    let lay = match kind {
        // Forrester 学术风需要节点分类（存量/速率/…）来排主干 + 卫星
        LayoutKind::Forrester => layout::compute_forrester(&nodes, &layout_edges, &class, g),
        _ => compute_layout(&nodes, &layout_edges, kind, g),
    };
    let pos = &lay.pos;

    // Forrester 学术风：主干横向、天然较宽 → 用原始尺寸 + 容器横向滚动（标签清晰），
    // 不像 force/layered 那样缩放填满面板宽（会把字压小）。
    let (size_attr, extra_cls) = if matches!(kind, LayoutKind::Forrester) {
        (format!(" width=\"{:.0}\" height=\"{:.0}\"", lay.width, lay.height), " natural")
    } else {
        (String::new(), "")
    };
    let mut s = format!(
        "<svg viewBox=\"0 0 {width:.0} {height:.0}\"{size_attr} class=\"dag-svg forr{extra_cls}\" xmlns=\"http://www.w3.org/2000/svg\">\
         <defs>\
         <marker id=\"farrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"#94a3b8\"/></marker>\
         <marker id=\"fmat\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"#f97316\"/></marker>\
         </defs>",
        width = lay.width,
        height = lay.height,
    );
    // 边
    for (a, b, mat) in &edges {
        if let (Some(&(x1, y1)), Some(&(x2, y2))) = (pos.get(a.as_str()), pos.get(b.as_str())) {
            let d = edge_path(x1, y1, x2, y2, bw, bh, kind);
            let (cls, mk) = if *mat { ("material", "fmat") } else { ("info", "farrow") };
            s.push_str(&format!(
                "<path d=\"{d}\" class=\"fedge {cls}\" data-from=\"{}\" data-to=\"{}\" marker-end=\"url(#{mk})\"/>",
                xml(a), xml(b)
            ));
        }
    }
    // 节点
    for n in &dag.nodes {
        let (x, y) = pos[n.id.as_str()];
        let c = class[n.id.as_str()];
        let short = n.id.rsplit('.').next().unwrap_or(&n.id);
        let label = if short.chars().count() > 16 {
            format!("{}…", short.chars().take(15).collect::<String>())
        } else {
            short.to_string()
        };
        s.push_str(&format!(
            "<g class=\"fnode {}\" data-var=\"{}\" data-id=\"{}\" data-cx=\"{:.0}\" data-cy=\"{:.0}\" data-hw=\"{:.0}\" data-hh=\"{:.0}\">",
            fclass_css(c), xml(short), xml(&n.id), x + bw / 2.0, y + bh / 2.0, bw / 2.0, bh / 2.0
        ));
        s.push_str(&fnode_shape(c, x, y, bw, bh));
        // 分类代号角标
        s.push_str(&format!(
            "<text class=\"fcode\" x=\"{:.0}\" y=\"{:.0}\">{}</text>",
            x + 10.0,
            y + 13.0,
            c.code()
        ));
        // 标签
        s.push_str(&format!(
            "<text x=\"{:.0}\" y=\"{:.0}\">{}</text></g>",
            x + bw / 2.0,
            y + bh / 2.0 + 5.0,
            xml(&label)
        ));
    }
    s.push_str("</svg>");
    s
}

/// Forrester 图例。
fn forrester_legend() -> String {
    "<div class=\"legend forr-legend\">\
     <span class=\"l state\">▭ 存量 S</span>\
     <span class=\"l rate\">⬡ 速率 V</span>\
     <span class=\"l driving\">⬭ 驱动 R</span>\
     <span class=\"l auxiliary\">▢ 辅助 A</span>\
     <span class=\"l parameter\">▢ 参数 D</span>\
     <span class=\"l semistate\">▭ 半状态 M</span>\
     <span class=\"l boundary\">▱ 边界 B</span>\
     <span class=\"l mat\">— 物质流</span>\
     <span class=\"l inf\">┈ 信息流</span></div>"
        .to_string()
}

// ============================================
// HTML 报告
// ============================================

const CSS: &str = r#"
:root { --bg:#fafbfc; --card:#fff; --ink:#1f2933; --sub:#6b7280; --line:#e5e7eb; --accent:#2563eb; }
* { box-sizing: border-box; }
body { margin:0; padding:0 0 60px; background:var(--bg); color:var(--ink);
  font-family: -apple-system,"Segoe UI","Microsoft YaHei",sans-serif; line-height:1.5; }
h1 { font-size:22px; padding:20px 28px; margin:0; border-bottom:1px solid var(--line); background:var(--card); }
h2 { font-size:17px; margin:28px 28px 12px; }
h2 .sub { color:var(--sub); font-weight:400; font-size:13px; margin-left:8px; }
.wrap { max-width:1100px; margin:0 auto; padding:0 8px; }
.dag { overflow-x:auto; background:var(--card); border:1px solid var(--line); border-radius:10px; margin:0 28px; padding:12px; cursor:grab; }
.dag-svg { min-width:100%; }
.dag-svg.natural { min-width:0; }  /* Forrester 学术风：原始尺寸，靠容器横向滚动 */
.dag-svg .edge { fill:none; stroke:#aab; stroke-width:1.5; }
.dag-svg .node rect { fill:#eef2ff; stroke:#c7d2fe; stroke-width:1.2; }
.dag-svg .node.parameter rect { fill:#ecfdf5; stroke:#a7f3d0; }
.dag-svg .node.variable rect { fill:#eff6ff; stroke:#bfdbfe; }
.dag-svg .node.equation rect { fill:#fef3c7; stroke:#fde68a; }
.dag-svg .node text { text-anchor:middle; font-size:14px; fill:#1f2933; }
/* —— Forrester 库存-流量图 —— */
.dag-svg.forr .fnode text { text-anchor:middle; font-size:14px; fill:#1f2933; }
.dag-svg.forr .fcode { text-anchor:start; font-size:9px; fill:#64748b; font-weight:700; }
.dag-svg.forr .fsh { stroke-width:1.4; }
.dag-svg.forr .fsh.state      { fill:#dbeafe; stroke:#3b82f6; stroke-width:2.2; }
.dag-svg.forr .fsh.semistate  { fill:#dbeafe; stroke:#3b82f6; stroke-width:1.6; stroke-dasharray:5 3; }
.dag-svg.forr .fsh.rate       { fill:#ffedd5; stroke:#f97316; stroke-width:1.8; }
.dag-svg.forr .fsh.driving    { fill:#dcfce7; stroke:#22c55e; }
.dag-svg.forr .fsh.parameter  { fill:#f3f4f6; stroke:#9ca3af; }
.dag-svg.forr .fsh.auxiliary  { fill:#f8fafc; stroke:#cbd5e1; }
.dag-svg.forr .fsh.control    { fill:#fae8ff; stroke:#d946ef; }
.dag-svg.forr .fsh.boundary   { fill:#ffffff; stroke:#94a3b8; stroke-dasharray:4 3; }
/* —— 点节点联动：可点 + 选中高亮（事件由 Studio 注入，报告本身零 JS） —— */
.dag-svg .fnode, .dag-svg .node { cursor:pointer; }
.dag-svg.forr .fnode.hl .fsh { stroke:#1d4ed8 !important; stroke-width:3.4 !important; }
.dag-svg .node.hl rect { stroke:#1d4ed8 !important; stroke-width:3 !important; fill:#dbeafe; }
.dag-svg .fnode.hl text, .dag-svg .node.hl text { font-weight:700; }
.eq.hl { outline:2px solid #2563eb; outline-offset:2px; background:#eff6ff; }
.dag-svg.forr .fedge { fill:none; }
.dag-svg.forr .fedge.material { stroke:#f97316; stroke-width:3; }
.dag-svg.forr .fedge.info     { stroke:#94a3b8; stroke-width:1.2; stroke-dasharray:4 3; }
.forr-legend .l { border:1px solid var(--line); }
.forr-legend .l.state { background:#dbeafe; } .forr-legend .l.rate { background:#ffedd5; }
.forr-legend .l.driving { background:#dcfce7; } .forr-legend .l.auxiliary { background:#f8fafc; }
.forr-legend .l.parameter { background:#f3f4f6; } .forr-legend .l.semistate { background:#dbeafe; border-style:dashed; }
.forr-legend .l.boundary { background:#fff; border-style:dashed; }
.forr-legend .l.mat { color:#f97316; font-weight:700; } .forr-legend .l.inf { color:#64748b; }
.eq { background:var(--card); border:1px solid var(--line); border-radius:10px; margin:10px 28px; padding:14px 18px; }
.eqhead { font-weight:600; font-size:14px; }
.eqhead .eqid { color:var(--sub); font-weight:400; font-size:12px; margin-left:8px; }
.eq math { font-size:1.25em; margin:8px 0; }
.meta { color:var(--sub); font-size:12px; margin-top:6px; }
.fdisp { font-family:"Cambria Math","Times New Roman",serif; color:#334155; font-size:13px; margin:4px 0; }
.cite { font-size:12px; margin-top:6px; padding:3px 8px; border-radius:6px; display:inline-block;
  background:#eff6ff; color:#1d4ed8; border:1px solid #bfdbfe; }
.cite.nocite { background:#fffbeb; color:#b45309; border-color:#fde68a; }
.legend { margin:8px 28px; font-size:12px; color:var(--sub); }
.legend span { display:inline-block; padding:1px 8px; border-radius:4px; margin-right:8px; }
.empty { color:var(--sub); padding:8px; }
"#;

/// 生成自包含 HTML 报告（默认分层布局，向后兼容）。
pub fn generate_report(files: &[EquationFile], dag: &Dag) -> String {
    generate_report_with(files, dag, LayoutKind::Layered)
}

/// 生成自包含 HTML 报告，指定结构图布局（向后兼容：变量级）。
pub fn generate_report_with(files: &[EquationFile], dag: &Dag, layout: LayoutKind) -> String {
    generate_report_leveled(files, dag, layout, DagLevel::Variable)
}

/// 生成自包含 HTML 报告，指定布局 + 粒度层级（变量/方程/模块）。
/// 调用方先用 [`crate::dag::collapse_dag`] 把 `dag` 折叠到对应粒度再传入。
pub fn generate_report_leveled(
    files: &[EquationFile],
    dag: &Dag,
    layout: LayoutKind,
    level: DagLevel,
) -> String {
    let title = files
        .first()
        .map(|f| {
            if f.meta.model.is_empty() {
                f.meta.id.clone()
            } else {
                f.meta.model.clone()
            }
        })
        .unwrap_or_else(|| "EQC 模型".to_string());

    // 结构图放在窄栏(.wrap)之外 → 占满整屏宽（「专注」全屏时不被 1100px 限住）；公式留在窄栏里好读。
    let mut body = String::new();

    if level == DagLevel::Variable {
        // Forrester 库存-流量图（动态结构：存量/速率/驱动/物质流）
        body.push_str("<h2>Forrester 库存-流量图<span class=\"sub\">动态结构：存量·速率·驱动·物质流</span></h2>");
        body.push_str(&forrester_legend());
        body.push_str(&format!("<div class=\"dag\">{}</div>", forrester_svg(files, dag, layout)));

        // 依赖关系图（按子模块分色的拓扑 DAG；节点名=变量label→方程中文名→代号）
        body.push_str("<h2>依赖关系图 (DAG)<span class=\"sub\">按子模块分色 · 节点名取方程中文名</span></h2>");
        body.push_str(&module_legend(dag));
        body.push_str(&format!("<div class=\"dag\">{}</div>", dag_svg(files, dag, layout)));
    } else {
        // 方程级 / 模块级：折叠后的依赖图（Forrester 为变量级专属，此处不画）；按子模块分色
        let (h, sub) = if level == DagLevel::Module {
            ("模块级结构图", "子模块 + 跨模块数据流 —— 一眼看整体运算逻辑")
        } else {
            ("方程级结构图", "方程节点（隐去参数叶子，节点名取方程中文名）—— 计算骨架")
        };
        body.push_str(&format!("<h2>{h}<span class=\"sub\">{sub}</span></h2>"));
        body.push_str(&module_legend(dag));
        body.push_str(&format!("<div class=\"dag\">{}</div>", dag_svg(files, dag, layout)));
    }

    // 公式区（窄栏，便于阅读）
    body.push_str("<div class=\"wrap\">");
    for f in files {
        body.push_str(&format!(
            "<h2>模块：{}<span class=\"sub\">{}</span></h2>",
            xml(&f.meta.name_cn),
            xml(&f.meta.id)
        ));
        for eq in &f.equations {
            let unit = f
                .variables
                .get(&eq.output)
                .and_then(|v| v.unit.clone())
                .unwrap_or_default();
            let meta = if unit.is_empty() {
                format!("输出：{}", xml(&eq.output))
            } else {
                format!("输出：{} · 单位 {}", xml(&eq.output), xml(&unit))
            };
            // 可读公式（若提供）
            let fdisp = eq
                .formula_display
                .as_ref()
                .map(|s| format!("<div class=\"fdisp\">{}</div>", xml(s)))
                .unwrap_or_default();
            // 来源标注：有则显示「📖 来源」，无则高亮「未标注来源」以便发现出处缺口
            let cite = match &eq.reference {
                Some(r) => format!("<div class=\"cite\">📖 来源：{}</div>", xml(r)),
                None => "<div class=\"cite nocite\">⚠ 未标注来源</div>".to_string(),
            };
            body.push_str(&format!(
                "<div class=\"eq\" data-output=\"{out}\"><div class=\"eqhead\">{}<span class=\"eqid\">{}</span></div>\
                 <math display=\"block\"><mrow>{}<mo>=</mo>{}</mrow></math>\
                 {fdisp}<div class=\"meta\">{meta}</div>{cite}</div>",
                xml(&eq.name),
                xml(&eq.id),
                ident(&eq.output),
                mml(&eq.expression),
                out = xml(&eq.output),
            ));
        }
    }
    body.push_str("</div>");

    format!(
        "<!DOCTYPE html><html lang=\"zh\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{t} · EQC 模型报告</title><style>{CSS}</style></head>\
         <body><h1>{t} · EQC 模型报告</h1>{body}</body></html>",
        t = xml(&title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mml_two_dimensional_forms() {
        // (div (add a b) 2) -> 分式
        let e = Expr::div(Expr::add(Expr::var("a"), Expr::var("b")), Expr::Const(2.0));
        assert!(mml(&e).contains("<mfrac>"));
        // (pow x 2) -> 上标
        assert!(mml(&Expr::pow(Expr::var("x"), Expr::Const(2.0))).contains("<msup>"));
        // (sqrt x) -> 根号
        assert!(mml(&Expr::sqrt(Expr::var("x"))).contains("<msqrt>"));
        // 下标变量名
        assert!(ident("Pmax_l").contains("<msub>"));
    }

    #[test]
    fn test_generate_report_smoke() {
        use crate::schema::{DataType, Equation, EquationFile, Metadata, Variable, VariableType};

        let mut variables = indexmap::IndexMap::new();
        let var = |t: VariableType, u: &str| Variable {
            var_type: t,
            dtype: DataType::Float,
            unit: Some(u.to_string()),
            description: None,
            label: None,
            measurable: false,
            stress_factor: None,
            stress_reduce: None,
            source: None,
            class: None,
            init: None,
            rate: None,
            prev: None,
        };
        variables.insert("y".to_string(), var(VariableType::Output, "kPa"));
        variables.insert("x".to_string(), var(VariableType::Input, "degC"));

        let file = EquationFile {
            meta: Metadata {
                id: "M".into(),
                model: "Demo".into(),
                name_cn: "演示模块".into(),
                name_en: None,
                version: "1.0".into(),
                description: None,
                reference: None,
                source_files: vec![],
                dt: 1.0,
                dt_seconds: None,
                calibration: None,
                modules: Default::default(),
            },
            parameters: Default::default(),
            variables,
            equations: vec![Equation {
                id: "E1".into(),
                name: "测试方程".into(),
                output: "y".into(),
                expression: Expr::mul(Expr::var("x"), Expr::Const(2.0)),
                formula_display: None,
                reference: None, gp_target: None,
            }],
        };
        let files = vec![file];
        let dag = crate::dag::build_dag(&files).unwrap();
        let html = generate_report(&files, &dag);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<math"), "应含 MathML 公式");
        assert!(html.contains("<svg"), "应含 SVG DAG");
        assert!(html.contains("演示模块"), "应含中文模块名");
        // Forrester 视图：x 为驱动(driving)、y 为输出辅助(auxiliary)
        assert!(html.contains("Forrester"), "应含 Forrester 图标题");
        assert!(html.contains("class=\"dag-svg forr\""), "应含 Forrester SVG");
        assert!(html.contains("fsh driving"), "输入 x 应分类为驱动");
        // 点节点联动用的数据标记：节点带 data-var、公式块带 data-output（事件由 Studio 注入）
        assert!(html.contains("data-var="), "节点应带 data-var");
        assert!(html.contains("data-output=\"y\""), "公式块应带 data-output");
        // 完全离线、零第三方 JS（联动靠数据属性 + CSS，不在报告里加脚本）
        assert!(!html.contains("<script"), "报告不应含任何 JS");
    }

    #[test]
    fn test_forrester_classes_and_material_flow() {
        use crate::schema::{DataType, Equation, EquationFile, Metadata, Variable, VariableType};

        // 极简动态模型：驱动 T、速率 R、积分状态 X（X.rate=R）
        let mk = |class: Option<VarClass>, t: VariableType, rate: Option<&str>, init: Option<f64>| Variable {
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
        };
        let mut variables = indexmap::IndexMap::new();
        variables.insert("T".into(), mk(Some(VarClass::Driving), VariableType::Input, None, None));
        variables.insert("R".into(), mk(Some(VarClass::Rate), VariableType::Intermediate, None, None));
        variables.insert("X".into(), mk(Some(VarClass::State), VariableType::Output, Some("R"), Some(0.0)));

        let file = EquationFile {
            meta: Metadata { id: "M".into(), model: "M".into(), name_cn: "动态".into(), name_en: None, version: "1.0".into(), description: None, reference: None, source_files: vec![], dt: 1.0, dt_seconds: None, calibration: None, modules: Default::default() },
            parameters: Default::default(),
            variables,
            equations: vec![Equation { id: "E".into(), name: "速率".into(), output: "R".into(), expression: Expr::mul(Expr::var("T"), Expr::Const(2.0)), formula_display: None, reference: None, gp_target: None }],
        };
        let files = vec![file];
        let dag = crate::dag::build_dag(&files).unwrap();
        let html = generate_report(&files, &dag);

        assert!(html.contains("fsh state"), "X 应渲染为存量");
        assert!(html.contains("fsh rate"), "R 应渲染为速率阀门(polygon)");
        assert!(html.contains("fedge material"), "速率→存量 应为物质流");
        assert!(html.contains("<polygon"), "速率阀门用 polygon");
    }
}
