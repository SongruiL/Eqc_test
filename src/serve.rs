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

/// 服务上下文：模型路径 + 预加载的情景数据 + 版本号。
struct Ctx {
    path: PathBuf,
    /// 预加载的驱动量（步数, 名->序列）；未提供则无法仿真。
    drivers: Option<(usize, HashMap<String, Vec<f64>>)>,
    params: Option<HashMap<String, f64>>,
    version: AtomicU64,
}

/// 启动本地服务，阻塞运行直到进程退出（Ctrl+C）。
pub fn serve(
    path: &Path,
    port: u16,
    drivers_path: Option<&PathBuf>,
    params_path: Option<&PathBuf>,
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

    let ctx = Arc::new(Ctx {
        path: path.to_path_buf(),
        drivers,
        params,
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
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let target = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");
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
        "/api/simulate" => match run_sim(ctx) {
            Ok(out) => ("200 OK", "application/json; charset=utf-8", trajectory_json(&out)),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        "/api/chart.svg" => {
            let vars = parse_vars(query);
            let svg = match run_sim(ctx) {
                Ok(out) => {
                    let refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
                    crate::chart::line_chart_svg(&out, &refs, 720.0, 360.0)
                }
                Err(e) => error_svg(&e),
            };
            ("200 OK", "image/svg+xml; charset=utf-8", svg)
        }
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

/// 用预加载的驱动量/参数跑一次仿真（单模型，取第一个模块）。
fn run_sim(ctx: &Ctx) -> Result<SimOutput, String> {
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
    simulate(file, &input).map_err(|e| format!("仿真失败: {e}"))
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
        // 布局切换条已打包
        assert!(STUDIO_HTML.contains("layoutSeg"));
        assert!(STUDIO_HTML.contains("/api/report?layout="));
    }

    #[test]
    fn test_parse_layout() {
        assert_eq!(parse_layout("layout=force&_=1"), LayoutKind::Force);
        assert_eq!(parse_layout("layout=forrester"), LayoutKind::Forrester);
        assert_eq!(parse_layout("layout=layered"), LayoutKind::Layered);
        assert_eq!(parse_layout(""), LayoutKind::Layered); // 缺省回退
        assert_eq!(parse_layout("vars=Y"), LayoutKind::Layered);
    }
}
