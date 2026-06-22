//! `eqc serve`：本地预览/交互服务（EQC Studio 的后端）。
//!
//! 「EQC 交互式前端」的落地（route：本地服务）。设计原则：**EQC 始终是唯一权威**，
//! 前端只显示 EQC（Rust）生成的产物——Forrester 图/公式（report 的 SVG+MathML）、
//! 仿真轨迹折线图（chart 的 SVG）、模型结构（export 的 JSON 契约）。前端与 EQC 之间
//! 只有一条**可检视、只增不改**的 JSON/SVG 契约，所以同步低风险、可增量、好排查。
//!
//! 路由：
//! - `/`               → Studio 页面（打包进二进制的静态 HTML，零构建步骤）
//! - `/api/model`      → 模型 JSON 契约（[`crate::export`]）
//! - `/api/report`     → 自包含 HTML 报告（Forrester 图 + 二维公式）
//! - `/api/simulate`   → 逐日仿真轨迹 JSON（需 `--drivers`）
//! - `/api/chart.svg?vars=Y,TDM` → 轨迹折线图 SVG（需 `--drivers`）
//! - `/api/optimize?spec=problem.yaml` → 跑优化，返回最优旋钮+收敛轨迹 JSON（与 `eqc optimize` 同结构）
//! - `/__version`      → 版本号（前端轮询，文件改动即整页刷新）
//!
//! 用极小的手写 HTTP（`std::net`，零新依赖）。监听模型文件 mtime，存盘即 +版本 → 自动刷新。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::dag::build_dag;
use crate::parser::{parse_directory, parse_file};
use crate::report::{generate_report_with, LayoutKind};
use crate::schema::EquationFile;
use crate::sim::{simulate, SimInput, SimOutput};

/// 打包进二进制的 Studio 前端页面（零构建步骤；以后可换成真正的 `frontend/` 构建产物）。
const STUDIO_HTML: &str = include_str!("serve_assets/studio.html");

/// 服务上下文：模型路径 + 预加载的情景数据 + 实测数据目录 + 版本号。
struct Ctx {
    path: PathBuf,
    /// 预加载的驱动量（步数, 名->序列）；未提供则无法仿真。
    drivers: Option<(usize, HashMap<String, Vec<f64>>)>,
    params: Option<HashMap<String, f64>>,
    /// 园区录入的实测数据目录（每处理区一个 `<zone>.csv`，稀疏 observed 格式）。
    /// `/api/observations` 在此读写；正是 `eqc calibrate --observed` 的输入。
    data_dir: PathBuf,
    version: AtomicU64,
}

/// 启动本地服务，阻塞运行直到进程退出（Ctrl+C）。
pub fn serve(
    path: &Path,
    port: u16,
    drivers_path: Option<&PathBuf>,
    params_path: Option<&PathBuf>,
    data_dir_arg: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 预加载情景数据（出错只警告、不阻断——模型结构仍可看）
    let drivers = match drivers_path {
        Some(p) => match crate::scenario::load_drivers_csv(p) {
            Ok(d) => {
                println!("   驱动量：{}（{} 天 × {} 列）", p.display(), d.0, d.1.len());
                Some(d)
            }
            Err(e) => {
                eprintln!("⚠️  驱动量加载失败（仿真不可用）：{e}");
                None
            }
        },
        None => None,
    };
    let params = match params_path {
        Some(p) => crate::scenario::load_params_json(p)
            .map_err(|e| {
                eprintln!("⚠️  参数 JSON 加载失败：{e}");
            })
            .ok(),
        None => None,
    };

    // 实测数据目录：显式 --data-dir 优先，否则模型同级的 observations/。
    let data_dir = data_dir_arg.cloned().unwrap_or_else(|| {
        let base = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
        };
        base.join("observations")
    });
    println!("   实测数据目录：{}（园区录入写入 <zone>.csv，缺则首次保存时创建）", data_dir.display());

    let ctx = Arc::new(Ctx {
        path: path.to_path_buf(),
        drivers,
        params,
        data_dir,
        version: AtomicU64::new(1),
    });

    // 文件监听线程：mtime 变化 → 版本 +1 → 前端整页刷新
    {
        let ctx = Arc::clone(&ctx);
        std::thread::spawn(move || {
            let mut last = fingerprint(&ctx.path);
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let fp = fingerprint(&ctx.path);
                if fp != last {
                    last = fp;
                    let v = ctx.version.fetch_add(1, Ordering::SeqCst) + 1;
                    println!("🔄 检测到改动（v{v}），前端将自动刷新");
                }
            }
        });
    }

    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("无法绑定 127.0.0.1:{port}（端口被占用？换 --port）：{e}"))?;
    println!("🌐 EQC Studio 运行中：  http://localhost:{port}/");
    println!("   监听 {}", path.display());
    println!("   编辑模型并保存即自动刷新；Ctrl+C 退出。");

    for stream in listener.incoming().flatten() {
        let ctx = Arc::clone(&ctx);
        std::thread::spawn(move || {
            let _ = handle(stream, &ctx);
        });
    }
    Ok(())
}

/// 处理一个连接。
fn handle(mut stream: TcpStream, ctx: &Ctx) -> std::io::Result<()> {
    let (head, req_body) = read_request(&mut stream)?;
    let mut first = head.lines().next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("GET");
    let target = first.next().unwrap_or("/");
    let (route, query) = match target.split_once('?') {
        Some((r, q)) => (r, q),
        None => (target, ""),
    };

    let (status, ctype, body): (&str, &str, String) = match route {
        "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", STUDIO_HTML.to_string()),
        "/__version" => (
            "200 OK",
            "text/plain; charset=utf-8",
            ctx.version.load(Ordering::SeqCst).to_string(),
        ),
        "/api/model" => match load_files(&ctx.path) {
            Ok(files) => ("200 OK", "application/json; charset=utf-8", crate::export::to_json_string(&files)),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        "/api/report" => match render_report(&ctx.path, parse_layout(query)) {
            Ok(h) => ("200 OK", "text/html; charset=utf-8", h),
            Err(e) => ("200 OK", "text/html; charset=utf-8", error_html(&e)),
        },
        "/api/simulate" => {
            let (pv, iv, dv) = (
                parse_overrides(query, "p"),
                parse_overrides(query, "init"),
                parse_overrides(query, "d"),
            );
            match run_sim(ctx, &pv, &iv, &dv) {
                Ok(out) => ("200 OK", "application/json; charset=utf-8", trajectory_json(&out)),
                Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
            }
        }
        "/api/chart.svg" => {
            let vars = parse_vars(query);
            let (pv, iv, dv) = (
                parse_overrides(query, "p"),
                parse_overrides(query, "init"),
                parse_overrides(query, "d"),
            );
            let svg = match run_sim(ctx, &pv, &iv, &dv) {
                Ok(out) => {
                    let refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
                    crate::chart::line_chart_svg(&out, &refs, 720.0, 360.0)
                }
                Err(e) => error_svg(&e),
            };
            ("200 OK", "image/svg+xml; charset=utf-8", svg)
        }
        "/api/optimize" => match run_optimize(ctx, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // 用某处理区录入的实测数据标定模型参数（录入→标定闭环的「标定」端）。
        "/api/calibrate" => match run_calibrate(ctx, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // 实测数据读写（园区录入 → 标定输入）。GET 读回某处理区的稀疏 observed；
        // POST 写出规范稀疏 CSV（EQC 权威拥有格式，前端只递交结构化数据）。
        "/api/observations" => match method {
            "GET" => ("200 OK", "application/json; charset=utf-8", read_observations(ctx, query)),
            "POST" => match write_observations(ctx, query, &req_body) {
                Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
                Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
            },
            _ => ("405 Method Not Allowed", "text/plain; charset=utf-8", "Method Not Allowed".to_string()),
        },
        // 每处理区的管理设置（灌溉/施氮/EC/CO₂…），存 <zone>.json；标定时叠加该区管理。
        "/api/zone" => match method {
            "GET" => ("200 OK", "application/json; charset=utf-8", read_zone(ctx, query)),
            "POST" => match write_zone(ctx, query, &req_body) {
                Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
                Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
            },
            _ => ("405 Method Not Allowed", "text/plain; charset=utf-8", "Method Not Allowed".to_string()),
        },
        _ => ("404 Not Found", "text/plain; charset=utf-8", "Not Found".to_string()),
    };

    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}",
        body.as_bytes().len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()
}

/// `?vars=Y,TDM` → ["Y","TDM"]。
fn parse_vars(query: &str) -> Vec<String> {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("vars=") {
            return url_decode(v)
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    Vec::new()
}

/// 极简 URL 解码（%XX + '+'→空格），够用于变量名。
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    out.push(b);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn load_files(path: &Path) -> Result<Vec<EquationFile>, String> {
    let files = if path.is_dir() {
        parse_directory(path).map_err(|e| e.to_string())?
    } else {
        vec![parse_file(path).map_err(|e| e.to_string())?]
    };
    crate::validator::validate(&files).map_err(|e| e.to_string())?;
    Ok(files)
}

fn render_report(path: &Path, layout: LayoutKind) -> Result<String, String> {
    let files = load_files(path)?;
    let dag = build_dag(&files).map_err(|e| e.to_string())?;
    Ok(generate_report_with(&files, &dag, layout))
}

/// `?layout=force` → 对应布局（未提供/未知 → 分层）。
fn parse_layout(query: &str) -> LayoutKind {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("layout=") {
            return LayoutKind::parse(&url_decode(v));
        }
    }
    LayoutKind::Layered
}

/// 用预加载的驱动量 + 参数跑一次仿真（单模型，取第一个模块）。
/// `param_ov`/`init_ov`/`driver_ov`：请求级覆盖（情景探索器 / 最优轨迹叠加传来），
/// 叠加在启动级 `--params`/`--drivers` 之上。`driver_ov` 把某驱动整列设成常数
/// （对应 `driver_const` 旋钮——这样优化得到的恒定 CO₂ 等也能画出其最优轨迹）。
fn run_sim(
    ctx: &Ctx,
    param_ov: &HashMap<String, f64>,
    init_ov: &HashMap<String, f64>,
    driver_ov: &HashMap<String, f64>,
) -> Result<SimOutput, String> {
    let files = load_files(&ctx.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?;
    let (steps, dmap) = ctx
        .drivers
        .as_ref()
        .ok_or_else(|| "未提供驱动量（启动时加 --drivers w.csv）——无法仿真".to_string())?;
    let mut input = SimInput::new(*steps);
    input.drivers = dmap.clone();
    if let Some(p) = &ctx.params {
        input.param_overrides = p.clone();
    }
    // 请求级覆盖叠加（优先级最高）
    for (k, v) in param_ov {
        input.param_overrides.insert(k.clone(), *v);
    }
    input.init_overrides = init_ov.clone();
    // 驱动常量覆盖：整列设成常数
    for (k, v) in driver_ov {
        input.drivers.insert(k.clone(), vec![*v; *steps]);
    }
    simulate(file, &input).map_err(|e| format!("仿真失败: {e}"))
}

/// `/api/optimize?spec=<路径>`：读决策 spec，跑优化，返回与 CLI 同一份结果 JSON。
/// spec 路径相对模型所在目录解析；环境驱动量取 spec 的 `environment:`（相对 spec 目录），
/// 缺省回退到启动级 `--drivers`。
fn run_optimize(ctx: &Ctx, query: &str) -> Result<String, String> {
    use crate::optimize::{self, load_problem};

    let spec_arg = parse_spec(query)
        .ok_or_else(|| "缺少 spec 参数（/api/optimize?spec=problem.yaml）".to_string())?;
    // spec 路径：绝对直接用，否则相对模型所在目录
    let model_dir: PathBuf = if ctx.path.is_dir() {
        ctx.path.clone()
    } else {
        ctx.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
    };
    let spec_path = {
        let p = PathBuf::from(&spec_arg);
        if p.is_absolute() {
            p
        } else {
            model_dir.join(&spec_arg)
        }
    };

    let problem = load_problem(&spec_path)?;
    let files = load_files(&ctx.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?;

    // 环境驱动量：spec 的 environment（相对 spec 目录）优先，否则启动级 --drivers
    let (steps, driver_map) = match &problem.environment {
        Some(env) => {
            let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));
            crate::scenario::load_drivers_csv(&spec_dir.join(env))?
        }
        None => match &ctx.drivers {
            Some((rows, map)) => (*rows, map.clone()),
            None => {
                return Err("决策 spec 无 environment 且启动时未提供 --drivers——无环境驱动量".into())
            }
        },
    };

    // 多目标：MO-DE 跑出 Pareto 前沿，注入散点图 SVG（点选叠加轨迹靠它）。
    if problem.is_multi() {
        let mr = optimize::run_mo(file, &problem, &driver_map, steps)?;
        let mut j = optimize::mo_result_json(file, &problem, &mr);
        if let Some(obj) = j.as_object_mut() {
            let pts: Vec<(f64, f64)> = mr
                .front
                .iter()
                .filter(|p| p.objectives.len() >= 2)
                .map(|p| (p.objectives[0], p.objectives[1]))
                .collect();
            let (xl, yl) = (problem.objective.expr.clone(), problem.objective2.as_ref().unwrap().expr.clone());
            obj.insert(
                "pareto_svg".to_string(),
                serde_json::Value::String(crate::chart::pareto_chart_svg(&pts, &xl, &yl, 700.0, 380.0)),
            );
        }
        return Ok(j.to_string());
    }

    let res = optimize::run(file, &problem, &driver_map, steps)?;
    // 数据 JSON（与 CLI 同结构）+ 注入 EQC 自生成的收敛曲线 SVG（供 Studio 直接显示；
    // CLI 写文件的 result_json 保持纯数据、不含 SVG）。
    let mut j = optimize::result_json(file, &problem, &res);
    if let Some(obj) = j.as_object_mut() {
        obj.insert(
            "convergence_svg".to_string(),
            serde_json::Value::String(crate::chart::convergence_chart_svg(&res.history, 720.0, 300.0)),
        );
    }
    Ok(j.to_string())
}

/// `/api/calibrate?spec=<路径>&zone=<处理区>`：用某处理区录入的实测数据标定模型参数。
/// 与 `eqc calibrate` 共用 `optimize::run_obs`。observed **优先**取该处理区录入的 CSV
/// （录入→标定闭环），否则回退 spec 的 `observed:`；同期天气取 spec 的 `environment:`，
/// 否则启动级 `--drivers`。返回与 `eqc calibrate` 同结构的结果 JSON + 注入收敛曲线 SVG。
fn run_calibrate(ctx: &Ctx, query: &str) -> Result<String, String> {
    use crate::optimize::{self, load_problem};

    let spec_arg = parse_spec(query)
        .ok_or_else(|| "缺少 spec 参数（/api/calibrate?spec=calib.yaml）".to_string())?;
    let zone = parse_zone(query);
    let model_dir: PathBuf = if ctx.path.is_dir() {
        ctx.path.clone()
    } else {
        ctx.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
    };
    let spec_path = {
        let p = PathBuf::from(&spec_arg);
        if p.is_absolute() {
            p
        } else {
            model_dir.join(&spec_arg)
        }
    };

    let problem = load_problem(&spec_path)?;
    if problem.is_multi() {
        return Err("标定暂为单目标（误差最小化）：spec 请用单个 objective".into());
    }
    let files = load_files(&ctx.path)?;
    // 克隆模型：本区管理通过改管理参数 default 注入 → 标定按本区处理仿真。
    let mut file = files.first().ok_or_else(|| "无模型".to_string())?.clone();
    let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));

    // 同期天气：spec 的 environment（相对 spec 目录）优先，否则启动级 --drivers
    let (steps, mut driver_map) = match &problem.environment {
        Some(env) => crate::scenario::load_drivers_csv(&spec_dir.join(env))?,
        None => match &ctx.drivers {
            Some((rows, map)) => (*rows, map.clone()),
            None => {
                return Err("标定 spec 无 environment 且启动时未提供 --drivers——无同期天气".into())
            }
        },
    };

    // 叠加本区管理：params 改模型参数 default，drivers 设为常数列（CO₂ 等控制量）。
    // 这样 calibrate 在「本区处理」下仿真——拿低氮区数据就用低氮管理拟合（多处理区标定的关键）。
    let (zparams, zdrivers) = read_zone_management(ctx, &zone);
    for (name, val) in &zparams {
        if let Some(p) = file.parameters.get_mut(name) {
            p.default = *val;
        }
    }
    for (name, val) in &zdrivers {
        driver_map.insert(name.clone(), vec![*val; steps]);
    }

    // 实测数据：本处理区录入的 observed CSV 优先（录入→标定闭环），否则 spec 的 observed
    let zone_csv = ctx.data_dir.join(format!("{zone}.csv"));
    let obs_path: PathBuf = if zone_csv.exists() {
        zone_csv
    } else {
        match &problem.observed {
            Some(o) => spec_dir.join(o),
            None => {
                return Err(format!(
                    "处理区 '{zone}' 暂无实测数据，且 spec 未写 observed:——请先在园区视图录入并保存"
                ))
            }
        }
    };
    let observed_data = crate::scenario::load_observed_csv(&obs_path)?;
    let n_obs: usize = observed_data.values().map(|v| v.len()).sum();

    let res = optimize::run_obs(&file, &problem, &driver_map, steps, &observed_data)?;
    let mut j = optimize::result_json(&file, &problem, &res);
    if let Some(obj) = j.as_object_mut() {
        obj.insert(
            "convergence_svg".to_string(),
            serde_json::Value::String(crate::chart::convergence_chart_svg(&res.history, 720.0, 300.0)),
        );
        obj.insert("zone".to_string(), serde_json::Value::String(zone));
        obj.insert(
            "observed_path".to_string(),
            serde_json::Value::String(obs_path.display().to_string()),
        );
        obj.insert("n_obs".to_string(), serde_json::json!(n_obs));
    }
    Ok(j.to_string())
}

/// `?spec=problem.yaml` → 路径串（url 解码）。
fn parse_spec(query: &str) -> Option<String> {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("spec=") {
            let s = url_decode(v);
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

/// 解析 `key=name:val,name2:val2`（url 解码后），用于 `p=`（参数）/`init=`（初值）覆盖。
fn parse_overrides(query: &str, key: &str) -> HashMap<String, f64> {
    let prefix = format!("{key}=");
    let mut out = HashMap::new();
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix(&prefix) {
            for pair in url_decode(v).split(',') {
                if let Some((name, val)) = pair.split_once(':') {
                    if let Ok(f) = val.trim().parse::<f64>() {
                        let n = name.trim();
                        if !n.is_empty() && f.is_finite() {
                            out.insert(n.to_string(), f);
                        }
                    }
                }
            }
        }
    }
    out
}

/// 轨迹 JSON：`{ "steps": N, "series": { name: [..], .. } }`。
fn trajectory_json(out: &SimOutput) -> String {
    let mut series = serde_json::Map::new();
    for (k, v) in &out.trajectories {
        series.insert(k.clone(), serde_json::json!(v));
    }
    serde_json::json!({ "steps": out.steps, "series": series }).to_string()
}

fn error_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

fn error_svg(msg: &str) -> String {
    let esc = msg.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        "<svg viewBox=\"0 0 720 360\" xmlns=\"http://www.w3.org/2000/svg\">\
         <text x=\"360\" y=\"180\" font-size=\"13\" fill=\"#dc2626\" text-anchor=\"middle\">{esc}</text></svg>"
    )
}

fn error_html(msg: &str) -> String {
    let esc = msg.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        "<!DOCTYPE html><html lang=\"zh\"><head><meta charset=\"utf-8\">\
         <title>EQC · 模型错误</title></head>\
         <body style=\"font-family:-apple-system,'Segoe UI','Microsoft YaHei',sans-serif;padding:24px;color:#1f2933\">\
         <h2 style=\"color:#dc2626\">⚠️ 模型解析 / 校验错误</h2>\
         <pre style=\"background:#fef2f2;border:1px solid #fecaca;padding:14px;border-radius:8px;white-space:pre-wrap\">{esc}</pre>\
         <p style=\"color:#6b7280\">修正后保存，页面会自动刷新。</p></body></html>"
    )
}

/// 读完整 HTTP 请求：返回 (头部文本, 请求体字节)。
/// 手写最小实现：先读到空行（头部结束），解析 `Content-Length`，再补足请求体。
/// （原实现只读一个 2048 缓冲、不读 body；POST 录入需要完整 body。）
fn read_request(stream: &mut TcpStream) -> std::io::Result<(String, Vec<u8>)> {
    let mut data: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];
    let mut head_end: Option<usize> = None;
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(p) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            head_end = Some(p + 4);
            break;
        }
        if data.len() > 64 * 1024 {
            break; // 头部异常大，止损
        }
    }
    let he = match head_end {
        Some(x) => x,
        None => return Ok((String::from_utf8_lossy(&data).into_owned(), Vec::new())),
    };
    let head = String::from_utf8_lossy(&data[..he]).into_owned();
    let content_len = head
        .lines()
        .find_map(|l| {
            let lower = l.to_ascii_lowercase();
            lower
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
        })
        .unwrap_or(0);
    const MAX_BODY: usize = 8 * 1024 * 1024; // 8MB 止损
    let want = content_len.min(MAX_BODY);
    let mut body = data[he..].to_vec();
    while body.len() < want {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(want);
    Ok((head, body))
}

/// 处理区名消毒：只允许字母/数字/下划线/连字符，长度 1..=64（防路径穿越）。
fn sanitize_zone(z: &str) -> Option<String> {
    let z = z.trim();
    if z.is_empty() || z.len() > 64 {
        return None;
    }
    if z.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        Some(z.to_string())
    } else {
        None
    }
}

/// `?zone=A` → 消毒后的区名；缺省/非法 → "default"。
fn parse_zone(query: &str) -> String {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("zone=") {
            if let Some(z) = sanitize_zone(&url_decode(v)) {
                return z;
            }
        }
    }
    "default".to_string()
}

/// 读回某处理区已录入的稀疏实测数据（JSON）。文件不存在 → 空集（非错误）。
fn read_observations(ctx: &Ctx, query: &str) -> String {
    let zone = parse_zone(query);
    let path = ctx.data_dir.join(format!("{zone}.csv"));
    if !path.exists() {
        return serde_json::json!({ "zone": zone, "exists": false, "observations": {}, "days": [] })
            .to_string();
    }
    match crate::scenario::load_observed_csv(&path) {
        Ok(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut obs = serde_json::Map::new();
            let mut days: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
            for k in keys {
                let series = &map[k];
                let arr: Vec<serde_json::Value> =
                    series.iter().map(|(d, v)| serde_json::json!([d, v])).collect();
                for (d, _) in series {
                    days.insert(*d);
                }
                obs.insert(k.clone(), serde_json::Value::Array(arr));
            }
            let days: Vec<usize> = days.into_iter().collect();
            serde_json::json!({ "zone": zone, "exists": true, "observations": obs, "days": days })
                .to_string()
        }
        Err(e) => error_json(&format!("读取实测数据失败: {e}")),
    }
}

/// 写出某处理区的稀疏实测 CSV（园区录入 → 标定输入）。EQC 拥有格式，前端只递交结构化数据：
/// body = `{ "columns": ["Y","TDM",...], "rows": [ {"DAT":30,"Y":1.2,"TDM":12.5}, ... ] }`。
/// 校验：列名须是模型变量、DAT 正整数、值有限；按 DAT 排序去重；原子替换写盘。
fn write_observations(ctx: &Ctx, query: &str, body: &[u8]) -> Result<String, String> {
    let zone = parse_zone(query);
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("请求体不是合法 JSON: {e}"))?;

    let columns: Vec<String> = v
        .get("columns")
        .and_then(|c| c.as_array())
        .ok_or_else(|| "缺 columns 数组".to_string())?
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
        .collect();
    if columns.is_empty() {
        return Err("columns 为空".into());
    }
    // 列名须是模型变量（防笔误、保证 CSV 可被 calibrate 消费）
    let files = load_files(&ctx.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?;
    for c in &columns {
        if c.is_empty() || c.contains(',') || c.contains('\n') || c.eq_ignore_ascii_case("DAT") {
            return Err(format!("非法列名: '{c}'"));
        }
        if !file.variables.contains_key(c) {
            return Err(format!("列 '{c}' 不是模型变量"));
        }
    }

    let rows = v
        .get("rows")
        .and_then(|r| r.as_array())
        .ok_or_else(|| "缺 rows 数组".to_string())?;
    let (csv, nrows) = build_observed_csv(&columns, rows)?;

    std::fs::create_dir_all(&ctx.data_dir).map_err(|e| format!("建目录失败: {e}"))?;
    let path = ctx.data_dir.join(format!("{zone}.csv"));
    let tmp = ctx.data_dir.join(format!(".{zone}.csv.tmp"));
    std::fs::write(&tmp, csv.as_bytes()).map_err(|e| format!("写临时文件失败: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("替换文件失败: {e}"))?;
    Ok(serde_json::json!({
        "ok": true, "zone": zone, "path": path.display().to_string(),
        "rows": nrows, "columns": columns
    })
    .to_string())
}

/// 把结构化行装成规范稀疏 CSV（首列 DAT + 各列；空格=未测）。返回 (CSV 文本, 数据行数)。
/// 同 DAT 合并（后写覆盖），按 DAT 升序；整行无值则跳过。纯函数、便于单测。
fn build_observed_csv(
    columns: &[String],
    rows: &[serde_json::Value],
) -> Result<(String, usize), String> {
    let mut by_day: std::collections::BTreeMap<i64, HashMap<String, f64>> =
        std::collections::BTreeMap::new();
    for (ri, row) in rows.iter().enumerate() {
        let obj = row
            .as_object()
            .ok_or_else(|| format!("第 {} 行不是对象", ri + 1))?;
        let dat = obj
            .get("DAT")
            .or_else(|| obj.get("dat"))
            .and_then(|d| d.as_f64())
            .ok_or_else(|| format!("第 {} 行缺 DAT", ri + 1))?;
        if dat <= 0.0 || dat.fract() != 0.0 {
            return Err(format!("第 {} 行 DAT 须为正整数: {dat}", ri + 1));
        }
        let entry = by_day.entry(dat as i64).or_default();
        for c in columns {
            if let Some(val) = obj.get(c) {
                if val.is_null() {
                    continue;
                }
                let f = val
                    .as_f64()
                    .ok_or_else(|| format!("第 {} 行列 '{c}' 不是数值", ri + 1))?;
                if !f.is_finite() {
                    return Err(format!("第 {} 行列 '{c}' 非有限值", ri + 1));
                }
                entry.insert(c.clone(), f);
            }
        }
    }
    let mut csv = String::from("DAT");
    for c in columns {
        csv.push(',');
        csv.push_str(c);
    }
    csv.push('\n');
    let mut nrows = 0usize;
    for (day, vals) in &by_day {
        if columns.iter().all(|c| !vals.contains_key(c)) {
            continue; // 整行空，跳过
        }
        csv.push_str(&day.to_string());
        for c in columns {
            csv.push(',');
            if let Some(f) = vals.get(c) {
                csv.push_str(&format!("{f}"));
            }
        }
        csv.push('\n');
        nrows += 1;
    }
    Ok((csv, nrows))
}

/// 从 JSON 对象抽取 `{name: 有限数}` 到 `dst`。
fn extract_scalar_map(v: Option<&serde_json::Value>, dst: &mut HashMap<String, f64>) {
    if let Some(obj) = v.and_then(|x| x.as_object()) {
        for (k, val) in obj {
            if let Some(f) = val.as_f64() {
                if f.is_finite() {
                    dst.insert(k.clone(), f);
                }
            }
        }
    }
}

/// 读某处理区的管理设置 `<zone>.json`（`{params:{}, drivers:{}}`）。缺/坏 → 空。
/// `params` = 管理**参数**覆盖（改模型参数）；`drivers` = **控制量**常数（如 CO₂）。
fn read_zone_management(ctx: &Ctx, zone: &str) -> (HashMap<String, f64>, HashMap<String, f64>) {
    let mut params = HashMap::new();
    let mut drivers = HashMap::new();
    if let Ok(txt) = std::fs::read_to_string(ctx.data_dir.join(format!("{zone}.json"))) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            extract_scalar_map(v.get("params"), &mut params);
            extract_scalar_map(v.get("drivers"), &mut drivers);
        }
    }
    (params, drivers)
}

/// `GET /api/zone?zone=A` → 该区管理设置 + 是否已有观测。
fn read_zone(ctx: &Ctx, query: &str) -> String {
    let zone = parse_zone(query);
    let (params, drivers) = read_zone_management(ctx, &zone);
    let has_obs = ctx.data_dir.join(format!("{zone}.csv")).exists();
    let pj: serde_json::Map<String, serde_json::Value> =
        params.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();
    let dj: serde_json::Map<String, serde_json::Value> =
        drivers.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();
    serde_json::json!({ "zone": zone, "params": pj, "drivers": dj, "has_observed": has_obs }).to_string()
}

/// `POST /api/zone?zone=A` body `{params:{}, drivers:{}}` → 写 `<zone>.json`（原子替换）。
fn write_zone(ctx: &Ctx, query: &str, body: &[u8]) -> Result<String, String> {
    let zone = parse_zone(query);
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("请求体不是合法 JSON: {e}"))?;
    let mut params = HashMap::new();
    let mut drivers = HashMap::new();
    extract_scalar_map(v.get("params"), &mut params);
    extract_scalar_map(v.get("drivers"), &mut drivers);
    let pj: serde_json::Map<String, serde_json::Value> =
        params.iter().map(|(k, v)| (k.clone(), serde_json::json!(v))).collect();
    let dj: serde_json::Map<String, serde_json::Value> =
        drivers.iter().map(|(k, v)| (k.clone(), serde_json::json!(v))).collect();
    let out = serde_json::json!({ "params": pj, "drivers": dj });
    std::fs::create_dir_all(&ctx.data_dir).map_err(|e| format!("建目录失败: {e}"))?;
    let path = ctx.data_dir.join(format!("{zone}.json"));
    let tmp = ctx.data_dir.join(format!(".{zone}.json.tmp"));
    std::fs::write(&tmp, serde_json::to_string_pretty(&out).unwrap_or_default())
        .map_err(|e| format!("写临时文件失败: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("替换文件失败: {e}"))?;
    Ok(serde_json::json!({ "ok": true, "zone": zone, "params": params.len(), "drivers": drivers.len() }).to_string())
}

/// 监听指纹：单文件取其 mtime；目录取所有 `.eq.yaml` 的最大 mtime。
fn fingerprint(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return std::fs::metadata(path).and_then(|m| m.modified()).ok();
    }
    let mut max: Option<SystemTime> = None;
    for entry in walkdir::WalkDir::new(path).into_iter().flatten() {
        let p = entry.path();
        let is_eq = p.extension().is_some_and(|e| e == "yaml" || e == "yml")
            && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains(".eq."));
        if is_eq {
            if let Ok(m) = p.metadata().and_then(|m| m.modified()) {
                max = Some(max.map_or(m, |x| x.max(m)));
            }
        }
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vars() {
        assert_eq!(parse_vars("vars=Y,TDM&_=123"), vec!["Y", "TDM"]);
        assert_eq!(parse_vars("vars=DF__1%2CDF__2"), vec!["DF__1", "DF__2"]);
        assert!(parse_vars("").is_empty());
    }

    #[test]
    fn test_error_helpers() {
        assert!(error_json("x").contains("\"error\""));
        assert!(error_svg("bad <x>").contains("&lt;x&gt;"));
        assert!(error_html("e").contains("模型解析"));
    }

    #[test]
    fn test_studio_html_bundled() {
        assert!(STUDIO_HTML.contains("EQC Studio"));
        assert!(STUDIO_HTML.contains("/api/model"));
        // 布局切换条 + 缩放 + 专注控件已打包
        assert!(STUDIO_HTML.contains("layoutSeg"));
        assert!(STUDIO_HTML.contains("/api/report?layout="));
        assert!(STUDIO_HTML.contains("zoomSeg"));
        assert!(STUDIO_HTML.contains("focusBtn"));
        // 节点悬停注释 + 点击联动
        assert!(STUDIO_HTML.contains("nodeTip"));
        assert!(STUDIO_HTML.contains("wireNodeClicks"));
        // 决策优化面板 + 多目标前沿渲染
        assert!(STUDIO_HTML.contains("optRun"));
        assert!(STUDIO_HTML.contains("/api/optimize?spec="));
        assert!(STUDIO_HTML.contains("renderParetoResult"));
        // 园区/简明视图：视图切换 + 录入网格 + 实测数据端点 + 标定
        assert!(STUDIO_HTML.contains("modeSeg"));
        assert!(STUDIO_HTML.contains("entryTable"));
        assert!(STUDIO_HTML.contains("/api/observations?zone="));
        assert!(STUDIO_HTML.contains("calibRun"));
        assert!(STUDIO_HTML.contains("/api/calibrate?spec="));
        // 看懂卡：标定徽章 + 头条 + 胁迫红绿灯
        assert!(STUDIO_HTML.contains("calBadge"));
        assert!(STUDIO_HTML.contains("renderUnderstand"));
        assert!(STUDIO_HTML.contains("stress-lights"));
        // 管理建议（大白话优化）
        assert!(STUDIO_HTML.contains("adviceRun"));
        assert!(STUDIO_HTML.contains("runAdvice"));
        // 多处理区：全局处理区栏 + 本区管理编辑器 + zone 端点
        assert!(STUDIO_HTML.contains("zone-bar"));
        assert!(STUDIO_HTML.contains("mgmtEditor"));
        assert!(STUDIO_HTML.contains("/api/zone?zone="));
    }

    #[test]
    fn test_parse_overrides() {
        let p = parse_overrides("vars=Y&p=Tbase:3.5,LUE:2.8&init=TF:0&_=1", "p");
        assert_eq!(p.get("Tbase"), Some(&3.5));
        assert_eq!(p.get("LUE"), Some(&2.8));
        let i = parse_overrides("p=Tbase:3.5&init=TF:0,DF:1.5", "init");
        assert_eq!(i.get("TF"), Some(&0.0));
        assert_eq!(i.get("DF"), Some(&1.5));
        assert!(parse_overrides("vars=Y", "p").is_empty());
        assert!(parse_overrides("p=bad:xx", "p").is_empty()); // 非数值忽略
    }

    #[test]
    fn test_parse_spec() {
        assert_eq!(parse_spec("spec=opt.yaml&_=1").as_deref(), Some("opt.yaml"));
        assert_eq!(parse_spec("spec=sub%2Fp.yaml").as_deref(), Some("sub/p.yaml"));
        assert!(parse_spec("vars=Y").is_none());
        assert!(parse_spec("spec=").is_none());
    }

    #[test]
    fn test_parse_layout() {
        assert_eq!(parse_layout("layout=force&_=1"), LayoutKind::Force);
        assert_eq!(parse_layout("layout=forrester"), LayoutKind::Forrester);
        assert_eq!(parse_layout("layout=layered"), LayoutKind::Layered);
        assert_eq!(parse_layout(""), LayoutKind::Layered); // 缺省回退
        assert_eq!(parse_layout("vars=Y"), LayoutKind::Layered);
    }

    #[test]
    fn test_sanitize_zone() {
        assert_eq!(sanitize_zone("zone_A-1").as_deref(), Some("zone_A-1"));
        assert_eq!(sanitize_zone("  A  ").as_deref(), Some("A"));
        assert!(sanitize_zone("../etc").is_none()); // 路径穿越
        assert!(sanitize_zone("a/b").is_none());
        assert!(sanitize_zone("a.b").is_none());
        assert!(sanitize_zone("").is_none());
    }

    #[test]
    fn test_parse_zone() {
        assert_eq!(parse_zone("zone=A1&_=1"), "A1");
        assert_eq!(parse_zone("vars=Y"), "default");
        assert_eq!(parse_zone("zone=../x"), "default"); // 非法 → default
    }

    #[test]
    fn test_build_observed_csv() {
        let cols = vec!["Y".to_string(), "TDM".to_string()];
        let rows = vec![
            serde_json::json!({"DAT": 60, "Y": 1.2}),
            serde_json::json!({"DAT": 30, "TDM": 12.5}),
            serde_json::json!({"DAT": 60, "TDM": 40.0}), // 同 DAT 合并
        ];
        let (csv, n) = build_observed_csv(&cols, &rows).unwrap();
        assert_eq!(n, 2);
        // 按 DAT 升序；列序 DAT,Y,TDM；DAT=30 行 Y 空（稀疏）
        assert_eq!(csv, "DAT,Y,TDM\n30,,12.5\n60,1.2,40\n");
        // 往返：写出的 CSV 能被 load_observed_csv 解析回来
        let dir = std::env::temp_dir().join("eqc_obs_test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("rt.csv");
        std::fs::write(&p, &csv).unwrap();
        let back = crate::scenario::load_observed_csv(&p).unwrap();
        assert_eq!(back["TDM"], vec![(30, 12.5), (60, 40.0)]);
        assert_eq!(back["Y"], vec![(60, 1.2)]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_build_observed_csv_rejects_bad() {
        let cols = vec!["Y".to_string()];
        assert!(build_observed_csv(&cols, &[serde_json::json!({"DAT": 1.5, "Y": 1.0})]).is_err()); // 非整 DAT
        assert!(build_observed_csv(&cols, &[serde_json::json!({"Y": 1.0})]).is_err()); // 缺 DAT
        assert!(build_observed_csv(&cols, &[serde_json::json!({"DAT": 10, "Y": "x"})]).is_err()); // 非数值
        // 空表也能产出（只有表头）
        let (csv, n) = build_observed_csv(&cols, &[]).unwrap();
        assert_eq!(n, 0);
        assert_eq!(csv, "DAT,Y\n");
    }
}
