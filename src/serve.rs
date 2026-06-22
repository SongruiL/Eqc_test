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

use crate::dag::{build_dag, collapse_dag, DagLevel};
use crate::parser::{parse_directory, parse_file};
use crate::report::{generate_report_leveled, LayoutKind};
use crate::schema::EquationFile;
use crate::sim::{simulate, SimInput, SimOutput};

/// 打包进二进制的 Studio 前端页面（零构建步骤；以后可换成真正的 `frontend/` 构建产物）。
const STUDIO_HTML: &str = include_str!("serve_assets/studio.html");

/// 工作区清单（多模型，免重启切模型）：`eqc serve eqc-workspace.yaml`，或
/// `eqc serve <目录>`（其中含 `eqc-workspace.yaml`）。每个模型**显式**声明
/// id/友好名/模型路径/驱动——因为作物目录里是版本史（s1..s8、t1..t3、bb1..bb5）、
/// 且每模型驱动不同（草莓日级 / 番茄小时 / 蓝莓需冷 / 温室室外），纯目录扫描无法判定。
#[derive(serde::Deserialize)]
struct WorkspaceManifest {
    models: Vec<ManifestEntry>,
    /// 耦合视图（step 3）：把多个模型连成一张大图（温室↔作物）。
    #[serde(default)]
    couplings: Vec<CouplingDecl>,
}

/// 清单中一条模型声明（路径相对清单所在目录解析）。
#[derive(serde::Deserialize)]
struct ManifestEntry {
    id: String,
    #[serde(default)]
    name: Option<String>,
    path: String,
    #[serde(default)]
    drivers: Option<String>,
    #[serde(default)]
    params: Option<String>,
    #[serde(default)]
    data_dir: Option<String>,
}

/// 耦合声明：`models` 引用上面 `models` 的 id，`links` 把作物驱动量接到温室输出。
/// 视图层概念——**不改 canonical 模型**，serve 加载时在内存里给作物 Input 注入 `source`。
#[derive(serde::Deserialize)]
struct CouplingDecl {
    id: String,
    #[serde(default)]
    name: Option<String>,
    models: Vec<String>,
    #[serde(default)]
    links: Vec<LinkDecl>,
}

/// 一条跨模型链接：作物驱动 `to`（`CROP.invar`）← 温室输出 `from`（`GH.outvar`）。
#[derive(serde::Deserialize)]
struct LinkDecl {
    from: String,
    to: String,
}

/// 耦合视图的运行态：参与文件 + 内存注入的 source 链接。
struct Coupling {
    /// 参与耦合的模型文件（按 `models` 顺序）。
    paths: Vec<PathBuf>,
    /// (from = "GH.outvar", to = "CROP.invar")；加载后给 CROP.invar set source=from。
    links: Vec<(String, String)>,
}

/// 花名册中的一个模型：路径 + 预载情景数据 + 本模型实测数据目录。
/// 单模型模式 = 1 个条目的花名册（行为与历史逐位一致、零回归）。
struct ModelEntry {
    /// 选择器/`?model=` 用的稳定标识（文件名安全：字母/数字/_/-）。单模型 = `"default"`。
    id: String,
    /// 友好名（清单 `name:` → 模型 `meta.name_cn` → id）。
    name: String,
    path: PathBuf,
    /// 预载驱动量（步数, 名->序列）；未提供则该模型无法仿真。
    drivers: Option<(usize, HashMap<String, Vec<f64>>)>,
    params: Option<HashMap<String, f64>>,
    /// 本模型园区录入的实测数据目录（每处理区一个 `<zone>.csv`，稀疏 observed）。
    /// `/api/observations` 在此读写；正是 `eqc calibrate --observed` 的输入。多模型按 id 隔离。
    data_dir: PathBuf,
    /// `Some` = 耦合视图条目（step 3）：path/drivers/data_dir 不用；结构图把多模型连成一张大图。
    /// 仿真/录入/标定对耦合条目返回友好错误（需单作物模型）。
    coupling: Option<Coupling>,
}

/// 服务上下文：模型花名册 + 版本号。单/多模型统一成一份 roster（≥1 条）。
struct Ctx {
    models: Vec<ModelEntry>,
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
    // 工作区清单（多模型）优先；否则单模型/目录合并模式（与历史逐位一致）。
    let models = if let Some(manifest) = workspace_manifest_path(path) {
        if drivers_path.is_some() || params_path.is_some() || data_dir_arg.is_some() {
            eprintln!("⚠️  工作区模式：--drivers/--params/--data-dir 被忽略（由清单按模型声明）");
        }
        println!("📦 工作区清单：{}", manifest.display());
        build_roster_from_manifest(&manifest)?
    } else {
        vec![build_single_entry(path, drivers_path, params_path, data_dir_arg)]
    };
    if models.is_empty() {
        return Err("工作区清单没有任何模型".into());
    }
    for m in &models {
        let drv = match &m.drivers {
            Some((n, cols)) => format!("{n} 步 × {} 列", cols.len()),
            None => "无（仿真不可用）".to_string(),
        };
        println!(
            "   • {} [{}]  {}  驱动:{}  实测:{}",
            m.name, m.id, m.path.display(), drv, m.data_dir.display()
        );
    }

    let ctx = Arc::new(Ctx { models, version: AtomicU64::new(1) });

    // 文件监听线程：任一模型文件 mtime 变化 → 版本 +1 → 前端整页刷新
    {
        let ctx = Arc::clone(&ctx);
        std::thread::spawn(move || {
            let mut last = roster_fingerprint(&ctx.models);
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let fp = roster_fingerprint(&ctx.models);
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
    println!("   {} 个模型；编辑模型并保存即自动刷新；Ctrl+C 退出。", ctx.models.len());

    for stream in listener.incoming().flatten() {
        let ctx = Arc::clone(&ctx);
        std::thread::spawn(move || {
            let _ = handle(stream, &ctx);
        });
    }
    Ok(())
}

/// 判定输入是否指向工作区清单：单文件且是**非** `.eq.` 的 `.yaml/.yml` → 它本身；
/// 目录 → 找其中的 `eqc-workspace.yaml`/`.yml`。否则 `None`（= 单模型/目录合并模式）。
/// `.eq.` 中缀是全代码库识别「方程文件」的约定，复用它当判别符。
fn workspace_manifest_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_yaml = name.ends_with(".yaml") || name.ends_with(".yml");
        if is_yaml && !name.contains(".eq.") {
            return Some(path.to_path_buf());
        }
    } else if path.is_dir() {
        for cand in ["eqc-workspace.yaml", "eqc-workspace.yml"] {
            let p = path.join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// 模型 id 必须文件名安全（用于 `observations/<id>` 目录，防路径穿越）。
fn valid_model_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// 预载驱动量（出错只警告、不阻断——模型结构仍可看）。
fn load_drivers_opt(p: Option<&Path>) -> Option<(usize, HashMap<String, Vec<f64>>)> {
    let p = p?;
    match crate::scenario::load_drivers_csv(p) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("⚠️  驱动量加载失败（{}，仿真不可用）：{e}", p.display());
            None
        }
    }
}

/// 预载参数覆盖 JSON（出错只警告）。
fn load_params_opt(p: Option<&Path>) -> Option<HashMap<String, f64>> {
    let p = p?;
    crate::scenario::load_params_json(p)
        .map_err(|e| eprintln!("⚠️  参数 JSON 加载失败（{}）：{e}", p.display()))
        .ok()
}

/// 读模型友好名（`meta.name_cn`）；解析失败则 `None`。
fn model_display_name(path: &Path) -> Option<String> {
    parse_file(path).ok().map(|f| f.meta.name_cn)
}

/// 单模型模式：1 个条目的花名册（驱动/参数/实测目录与历史逐位一致）。
fn build_single_entry(
    path: &Path,
    drivers_path: Option<&PathBuf>,
    params_path: Option<&PathBuf>,
    data_dir_arg: Option<&PathBuf>,
) -> ModelEntry {
    let drivers = load_drivers_opt(drivers_path.map(|p| p.as_path()));
    if let Some((n, cols)) = &drivers {
        println!("   驱动量：{n} 步 × {} 列", cols.len());
    }
    let params = load_params_opt(params_path.map(|p| p.as_path()));
    // 实测数据目录：显式 --data-dir 优先，否则模型同级的 observations/（不按 id 分子目录，保持历史路径）。
    let data_dir = data_dir_arg.cloned().unwrap_or_else(|| {
        let base = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
        };
        base.join("observations")
    });
    let name = model_display_name(path).unwrap_or_else(|| {
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("model").to_string()
    });
    ModelEntry {
        id: "default".to_string(),
        name,
        path: path.to_path_buf(),
        drivers,
        params,
        data_dir,
        coupling: None,
    }
}

/// 工作区模式：按清单逐条建模型（路径相对清单目录；实测目录缺省 `<清单目录>/observations/<id>/`）。
fn build_roster_from_manifest(manifest: &Path) -> Result<Vec<ModelEntry>, String> {
    let txt = std::fs::read_to_string(manifest)
        .map_err(|e| format!("读工作区清单失败（{}）：{e}", manifest.display()))?;
    let ws: WorkspaceManifest = serde_yaml::from_str(&txt)
        .map_err(|e| format!("工作区清单不是合法 YAML（需顶层 `models: [...]`）：{e}"))?;
    if ws.models.is_empty() {
        return Err("工作区清单 models 为空".into());
    }
    let base = manifest.parent().unwrap_or_else(|| Path::new("."));
    let resolve = |rel: &str| -> PathBuf {
        let p = PathBuf::from(rel);
        if p.is_absolute() {
            p
        } else {
            base.join(rel)
        }
    };
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for e in ws.models {
        if !valid_model_id(&e.id) {
            return Err(format!("非法模型 id '{}'（只允许字母/数字/下划线/连字符、长度 1..=64）", e.id));
        }
        if !seen.insert(e.id.clone()) {
            return Err(format!("模型 id 重复：'{}'", e.id));
        }
        let path = resolve(&e.path);
        if !path.exists() {
            eprintln!("⚠️  模型 '{}' 文件不存在：{}", e.id, path.display());
        }
        let drivers = match &e.drivers {
            Some(d) => {
                let p = resolve(d);
                load_drivers_opt(Some(p.as_path()))
            }
            None => None,
        };
        let params = match &e.params {
            Some(p) => {
                let pp = resolve(p);
                load_params_opt(Some(pp.as_path()))
            }
            None => None,
        };
        let data_dir = match &e.data_dir {
            Some(d) => resolve(d),
            None => base.join("observations").join(&e.id),
        };
        let name = e
            .name
            .clone()
            .or_else(|| model_display_name(&path))
            .unwrap_or_else(|| e.id.clone());
        out.push(ModelEntry { id: e.id, name, path, drivers, params, data_dir, coupling: None });
    }

    // 耦合视图条目（step 3）：解析 models→已建模型条目的文件路径，校验 links。
    for c in ws.couplings {
        if !valid_model_id(&c.id) {
            return Err(format!("非法耦合 id '{}'", c.id));
        }
        if !seen.insert(c.id.clone()) {
            return Err(format!("id 重复（模型/耦合）：'{}'", c.id));
        }
        if c.models.len() < 2 {
            return Err(format!("耦合 '{}' 至少需 2 个模型", c.id));
        }
        let mut paths = Vec::new();
        for mid in &c.models {
            match out.iter().find(|m| &m.id == mid && m.coupling.is_none()) {
                Some(m) => paths.push(m.path.clone()),
                None => return Err(format!("耦合 '{}' 引用了不存在的模型 id '{mid}'", c.id)),
            }
        }
        let links: Vec<(String, String)> = c
            .links
            .iter()
            .filter_map(|l| {
                if l.from.contains('.') && l.to.contains('.') {
                    Some((l.from.clone(), l.to.clone()))
                } else {
                    eprintln!("⚠️  耦合 '{}' 链接格式应为 MODULE.var：from='{}' to='{}'（已跳过）", c.id, l.from, l.to);
                    None
                }
            })
            .collect();
        let name = c.name.clone().unwrap_or_else(|| c.id.clone());
        out.push(ModelEntry {
            id: c.id,
            name,
            path: PathBuf::new(),
            drivers: None,
            params: None,
            data_dir: PathBuf::new(),
            coupling: Some(Coupling { paths, links }),
        });
    }
    Ok(out)
}

/// 加载某条目的方程文件：单模型 = 直接解析校验；耦合 = 加载各参与文件 + 按 links
/// **在内存里**给作物 Input 注入 `source`（不落盘）→ `build_dag` 自然产出跨模型边 →
/// 因两模块都在场，validator 不会报 source 模块缺失。
fn load_model_files(m: &ModelEntry) -> Result<Vec<EquationFile>, String> {
    match &m.coupling {
        None => load_files(&m.path),
        Some(c) => {
            let mut files: Vec<EquationFile> = Vec::new();
            for p in &c.paths {
                files.push(parse_file(p).map_err(|e| e.to_string())?);
            }
            for (from, to) in &c.links {
                if let Some((tomod, tovar)) = to.split_once('.') {
                    let mut hit = false;
                    for f in &mut files {
                        if f.meta.id == tomod {
                            if let Some(v) = f.variables.get_mut(tovar) {
                                v.source = Some(from.clone());
                                hit = true;
                            }
                        }
                    }
                    if !hit {
                        eprintln!("⚠️  耦合链接 to='{to}' 在模型里找不到对应 Input 变量（已跳过）");
                    }
                }
            }
            crate::validator::validate(&files).map_err(|e| e.to_string())?;
            Ok(files)
        }
    }
}

/// 整个花名册的监听指纹（各模型文件 mtime 的有序快照；任一变化即触发刷新）。
fn roster_fingerprint(models: &[ModelEntry]) -> Vec<Option<SystemTime>> {
    models.iter().map(|m| fingerprint(&m.path)).collect()
}

/// 按 `?model=<id>` 解析当前模型（缺省/未知 → 花名册第一个；花名册恒 ≥1 条）。
fn resolve_model<'a>(ctx: &'a Ctx, query: &str) -> &'a ModelEntry {
    if let Some(id) = parse_model(query) {
        if let Some(m) = ctx.models.iter().find(|m| m.id == id) {
            return m;
        }
    }
    &ctx.models[0]
}

/// `?model=strawberry` → "strawberry"（url 解码；缺省 None）。
fn parse_model(query: &str) -> Option<String> {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("model=") {
            let s = url_decode(v);
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

/// 模型花名册 JSON（前端据此建顶部选择器；不硬编码作物清单——问 EQC 要）。
/// `coupled` 标记耦合视图条目（前端可分组/提示其只看结构图）。
fn models_json(ctx: &Ctx) -> String {
    let arr: Vec<serde_json::Value> = ctx
        .models
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id, "name": m.name,
                "has_drivers": m.drivers.is_some(),
                "coupled": m.coupling.is_some()
            })
        })
        .collect();
    serde_json::json!({ "models": arr }).to_string()
}

/// 耦合视图条目不支持仿真/录入/标定（需单作物模型）；返回友好错误串，否则 None。
fn coupled_guard(m: &ModelEntry) -> Option<String> {
    m.coupling.as_ref().map(|_| {
        format!("「{}」是耦合视图，暂只支持结构图（仿真/录入/标定请选单个作物模型）", m.name)
    })
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

    // 当前模型（按 `?model=<id>`，缺省第一个）。/、/__version、/api/models 不依赖它，但解析无害。
    let m = resolve_model(ctx, query);

    let (status, ctype, body): (&str, &str, String) = match route {
        "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", STUDIO_HTML.to_string()),
        "/__version" => (
            "200 OK",
            "text/plain; charset=utf-8",
            ctx.version.load(Ordering::SeqCst).to_string(),
        ),
        // 模型花名册（前端建顶部选择器用）。
        "/api/models" => ("200 OK", "application/json; charset=utf-8", models_json(ctx)),
        "/api/model" => match load_model_files(m) {
            Ok(files) => ("200 OK", "application/json; charset=utf-8", crate::export::to_json_string(&files)),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        "/api/report" => match load_model_files(m).and_then(|f| render_report(&f, parse_layout(query), parse_dag_level(query))) {
            Ok(h) => ("200 OK", "text/html; charset=utf-8", h),
            Err(e) => ("200 OK", "text/html; charset=utf-8", error_html(&e)),
        },
        "/api/simulate" => {
            let (pv, iv, dv) = (
                parse_overrides(query, "p"),
                parse_overrides(query, "init"),
                parse_overrides(query, "d"),
            );
            match run_sim(m, &pv, &iv, &dv) {
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
            let svg = match run_sim(m, &pv, &iv, &dv) {
                Ok(out) => {
                    let refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
                    crate::chart::line_chart_svg(&out, &refs, 720.0, 360.0)
                }
                Err(e) => error_svg(&e),
            };
            ("200 OK", "image/svg+xml; charset=utf-8", svg)
        }
        "/api/optimize" => match run_optimize(m, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // 用某处理区录入的实测数据标定模型参数（录入→标定闭环的「标定」端）。
        "/api/calibrate" => match run_calibrate(m, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // 实测数据读写（园区录入 → 标定输入）。GET 读回某处理区的稀疏 observed；
        // POST 写出规范稀疏 CSV（EQC 权威拥有格式，前端只递交结构化数据）。
        "/api/observations" => match method {
            "GET" => ("200 OK", "application/json; charset=utf-8", read_observations(m, query)),
            "POST" => match write_observations(m, query, &req_body) {
                Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
                Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
            },
            _ => ("405 Method Not Allowed", "text/plain; charset=utf-8", "Method Not Allowed".to_string()),
        },
        // 每处理区的管理设置（灌溉/施氮/EC/CO₂…），存 <zone>.json；标定时叠加该区管理。
        "/api/zone" => match method {
            "GET" => ("200 OK", "application/json; charset=utf-8", read_zone(m, query)),
            "POST" => match write_zone(m, query, &req_body) {
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

fn render_report(files: &[EquationFile], layout: LayoutKind, level: DagLevel) -> Result<String, String> {
    let dag = build_dag(files).map_err(|e| e.to_string())?;
    let collapsed = collapse_dag(&dag, files, level);
    Ok(generate_report_leveled(files, &collapsed, layout, level))
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

/// `?level=module` → 对应粒度（未提供/未知 → 变量级）。
fn parse_dag_level(query: &str) -> DagLevel {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("level=") {
            return DagLevel::parse(&url_decode(v));
        }
    }
    DagLevel::Variable
}

/// 用预加载的驱动量 + 参数跑一次仿真（单模型，取第一个模块）。
/// `param_ov`/`init_ov`/`driver_ov`：请求级覆盖（情景探索器 / 最优轨迹叠加传来），
/// 叠加在启动级 `--params`/`--drivers` 之上。`driver_ov` 把某驱动整列设成常数
/// （对应 `driver_const` 旋钮——这样优化得到的恒定 CO₂ 等也能画出其最优轨迹）。
fn run_sim(
    m: &ModelEntry,
    param_ov: &HashMap<String, f64>,
    init_ov: &HashMap<String, f64>,
    driver_ov: &HashMap<String, f64>,
) -> Result<SimOutput, String> {
    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }
    let files = load_files(&m.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?;
    let (steps, dmap) = m
        .drivers
        .as_ref()
        .ok_or_else(|| "未提供驱动量（该模型清单未声明 drivers，或启动时未加 --drivers）——无法仿真".to_string())?;
    let mut input = SimInput::new(*steps);
    input.drivers = dmap.clone();
    if let Some(p) = &m.params {
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
fn run_optimize(m: &ModelEntry, query: &str) -> Result<String, String> {
    use crate::optimize::{self, load_problem};

    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }
    let spec_arg = parse_spec(query)
        .ok_or_else(|| "缺少 spec 参数（/api/optimize?spec=problem.yaml）".to_string())?;
    // spec 路径：绝对直接用，否则相对模型所在目录
    let model_dir: PathBuf = if m.path.is_dir() {
        m.path.clone()
    } else {
        m.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
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
    let files = load_files(&m.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?;

    // 环境驱动量：spec 的 environment（相对 spec 目录）优先，否则该模型预载 drivers
    let (steps, driver_map) = match &problem.environment {
        Some(env) => {
            let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));
            crate::scenario::load_drivers_csv(&spec_dir.join(env))?
        }
        None => match &m.drivers {
            Some((rows, map)) => (*rows, map.clone()),
            None => {
                return Err("决策 spec 无 environment 且该模型无预载驱动量——无环境驱动量".into())
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
fn run_calibrate(m: &ModelEntry, query: &str) -> Result<String, String> {
    use crate::optimize::{self, load_problem};

    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }
    let spec_arg = parse_spec(query)
        .ok_or_else(|| "缺少 spec 参数（/api/calibrate?spec=calib.yaml）".to_string())?;
    let zone = parse_zone(query);
    let model_dir: PathBuf = if m.path.is_dir() {
        m.path.clone()
    } else {
        m.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
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
    let files = load_files(&m.path)?;
    // 克隆模型：本区管理通过改管理参数 default 注入 → 标定按本区处理仿真。
    let mut file = files.first().ok_or_else(|| "无模型".to_string())?.clone();
    let spec_dir = spec_path.parent().unwrap_or_else(|| Path::new("."));

    // 同期天气：spec 的 environment（相对 spec 目录）优先，否则该模型预载 drivers
    let (steps, mut driver_map) = match &problem.environment {
        Some(env) => crate::scenario::load_drivers_csv(&spec_dir.join(env))?,
        None => match &m.drivers {
            Some((rows, map)) => (*rows, map.clone()),
            None => {
                return Err("标定 spec 无 environment 且该模型无预载驱动量——无同期天气".into())
            }
        },
    };

    // 叠加本区管理：params 改模型参数 default，drivers 设为常数列（CO₂ 等控制量）。
    // 这样 calibrate 在「本区处理」下仿真——拿低氮区数据就用低氮管理拟合（多处理区标定的关键）。
    let (zparams, zdrivers) = read_zone_management(&m.data_dir, &zone);
    for (name, val) in &zparams {
        if let Some(p) = file.parameters.get_mut(name) {
            p.default = *val;
        }
    }
    for (name, val) in &zdrivers {
        driver_map.insert(name.clone(), vec![*val; steps]);
    }

    // 实测数据：本处理区录入的 observed CSV 优先（录入→标定闭环），否则 spec 的 observed
    let zone_csv = m.data_dir.join(format!("{zone}.csv"));
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
        obj.insert("zone".to_string(), serde_json::Value::String(zone.clone()));
        obj.insert(
            "observed_path".to_string(),
            serde_json::Value::String(obs_path.display().to_string()),
        );
        obj.insert("n_obs".to_string(), serde_json::json!(n_obs));
    }

    // 4B：标定成功 → 持久化本区标定状态（<zone>.calib.json）；看懂卡徽章据此翻「本区已标定」。
    // 模型级 meta.calibration 仍诚实保持"整体未标定"（区级 ≠ 跨区联合标定）。
    if res.outcome.objective.is_some() {
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let calib = serde_json::json!({
            "spec": spec_arg,
            "error": res.outcome.objective,
            "objective": problem.objective.expr,
            "n_obs": n_obs,
            "knobs": j.get("best_knobs").cloned().unwrap_or(serde_json::Value::Null),
            "at": at,
        });
        let _ = std::fs::create_dir_all(&m.data_dir);
        let _ = std::fs::write(
            m.data_dir.join(format!("{zone}.calib.json")),
            serde_json::to_string_pretty(&calib).unwrap_or_default(),
        );
        if let Some(obj) = j.as_object_mut() {
            obj.insert("calibrated_at".to_string(), serde_json::json!(at));
        }
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
fn read_observations(m: &ModelEntry, query: &str) -> String {
    if let Some(e) = coupled_guard(m) {
        return error_json(&e);
    }
    let zone = parse_zone(query);
    let path = m.data_dir.join(format!("{zone}.csv"));
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
fn write_observations(m: &ModelEntry, query: &str, body: &[u8]) -> Result<String, String> {
    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }
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
    let files = load_files(&m.path)?;
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

    std::fs::create_dir_all(&m.data_dir).map_err(|e| format!("建目录失败: {e}"))?;
    let path = m.data_dir.join(format!("{zone}.csv"));
    let tmp = m.data_dir.join(format!(".{zone}.csv.tmp"));
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
fn read_zone_management(data_dir: &Path, zone: &str) -> (HashMap<String, f64>, HashMap<String, f64>) {
    let mut params = HashMap::new();
    let mut drivers = HashMap::new();
    if let Ok(txt) = std::fs::read_to_string(data_dir.join(format!("{zone}.json"))) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            extract_scalar_map(v.get("params"), &mut params);
            extract_scalar_map(v.get("drivers"), &mut drivers);
        }
    }
    (params, drivers)
}

/// 读某处理区的标定状态 `<zone>.calib.json`（4B）；缺/坏 → `null`。
fn read_calib_status(data_dir: &Path, zone: &str) -> serde_json::Value {
    match std::fs::read_to_string(data_dir.join(format!("{zone}.calib.json"))) {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or(serde_json::Value::Null),
        Err(_) => serde_json::Value::Null,
    }
}

/// `GET /api/zone?zone=A` → 该区管理设置 + 是否已有观测 + 标定状态。
fn read_zone(m: &ModelEntry, query: &str) -> String {
    if let Some(e) = coupled_guard(m) {
        return error_json(&e);
    }
    let zone = parse_zone(query);
    let (params, drivers) = read_zone_management(&m.data_dir, &zone);
    let has_obs = m.data_dir.join(format!("{zone}.csv")).exists();
    let pj: serde_json::Map<String, serde_json::Value> =
        params.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();
    let dj: serde_json::Map<String, serde_json::Value> =
        drivers.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();
    serde_json::json!({
        "zone": zone, "params": pj, "drivers": dj,
        "has_observed": has_obs, "calibration": read_calib_status(&m.data_dir, &zone)
    })
    .to_string()
}

/// `POST /api/zone?zone=A` body `{params:{}, drivers:{}}` → 写 `<zone>.json`（原子替换）。
fn write_zone(m: &ModelEntry, query: &str, body: &[u8]) -> Result<String, String> {
    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }
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
    std::fs::create_dir_all(&m.data_dir).map_err(|e| format!("建目录失败: {e}"))?;
    let path = m.data_dir.join(format!("{zone}.json"));
    let tmp = m.data_dir.join(format!(".{zone}.json.tmp"));
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
        // 4：默认进园区 + 区级标定徽章
        assert!(STUDIO_HTML.contains("ZONE_CALIB"));
        assert!(STUDIO_HTML.contains("本区已标定"));
        // DAG 粒度切换（变量/方程/模块）
        assert!(STUDIO_HTML.contains("levelSeg"));
        assert!(STUDIO_HTML.contains("&level="));
        // 多模型选择器（免重启切模型）：花名册端点 + 选择器 + 切换逻辑
        assert!(STUDIO_HTML.contains("/api/models"));
        assert!(STUDIO_HTML.contains("modelSel"));
        assert!(STUDIO_HTML.contains("applyModel"));
        assert!(STUDIO_HTML.contains("modelParam"));
        // 耦合视图（step 3）：optgroup 分组 + 标题提示
        assert!(STUDIO_HTML.contains("耦合视图"));
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
    fn test_parse_model() {
        assert_eq!(parse_model("model=tomato&level=module").as_deref(), Some("tomato"));
        assert_eq!(parse_model("layout=force&model=strawberry").as_deref(), Some("strawberry"));
        assert!(parse_model("vars=Y").is_none());
        assert!(parse_model("model=").is_none());
    }

    #[test]
    fn test_valid_model_id() {
        assert!(valid_model_id("strawberry"));
        assert!(valid_model_id("bb5_gh-1"));
        assert!(!valid_model_id("")); // 空
        assert!(!valid_model_id("../etc")); // 路径穿越
        assert!(!valid_model_id("a/b"));
        assert!(!valid_model_id("a b")); // 空格
    }

    #[test]
    fn test_workspace_manifest_path() {
        let dir = std::env::temp_dir().join("eqc_ws_test");
        let _ = std::fs::create_dir_all(&dir);
        // 单模型 .eq.yaml → 非工作区
        let model = dir.join("strawberry_s8.eq.yaml");
        std::fs::write(&model, "x").unwrap();
        assert!(workspace_manifest_path(&model).is_none());
        // 普通 .yaml 文件 → 工作区清单本身
        let ws = dir.join("my-workspace.yaml");
        std::fs::write(&ws, "models: []").unwrap();
        assert_eq!(workspace_manifest_path(&ws).as_deref(), Some(ws.as_path()));
        // 目录含 eqc-workspace.yaml → 找到它
        let canonical = dir.join("eqc-workspace.yaml");
        std::fs::write(&canonical, "models: []").unwrap();
        assert_eq!(workspace_manifest_path(&dir).as_deref(), Some(canonical.as_path()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_roster_from_manifest() {
        let dir = std::env::temp_dir().join("eqc_roster_test");
        let _ = std::fs::create_dir_all(&dir);
        let ws = dir.join("eqc-workspace.yaml");
        // 提供 name: 故无需解析真实模型文件；path 不存在只告警不失败
        std::fs::write(
            &ws,
            "models:\n  - { id: strawberry, name: 草莓 S8, path: a/s8.eq.yaml }\n  - { id: tomato, name: 番茄 T3, path: b/t3.eq.yaml, data_dir: obs_t }\n",
        )
        .unwrap();
        let roster = build_roster_from_manifest(&ws).unwrap();
        assert_eq!(roster.len(), 2);
        assert_eq!(roster[0].id, "strawberry");
        assert_eq!(roster[0].name, "草莓 S8");
        // 路径相对清单目录解析
        assert_eq!(roster[0].path, dir.join("a/s8.eq.yaml"));
        // 缺省实测目录 = <清单目录>/observations/<id>
        assert_eq!(roster[0].data_dir, dir.join("observations").join("strawberry"));
        // 显式 data_dir 相对清单目录解析
        assert_eq!(roster[1].data_dir, dir.join("obs_t"));

        // 重复 id → 错误
        std::fs::write(&ws, "models:\n  - { id: x, path: a }\n  - { id: x, path: b }\n").unwrap();
        assert!(build_roster_from_manifest(&ws).is_err());
        // 非法 id → 错误
        std::fs::write(&ws, "models:\n  - { id: \"../x\", path: a }\n").unwrap();
        assert!(build_roster_from_manifest(&ws).is_err());
        // 空 models → 错误
        std::fs::write(&ws, "models: []\n").unwrap();
        assert!(build_roster_from_manifest(&ws).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_roster_couplings() {
        let dir = std::env::temp_dir().join("eqc_couple_test");
        let _ = std::fs::create_dir_all(&dir);
        let ws = dir.join("eqc-workspace.yaml");
        std::fs::write(
            &ws,
            "models:\n  - { id: gh, name: 温室, path: gh.eq.yaml }\n  - { id: bb, name: 蓝莓, path: bb.eq.yaml }\ncouplings:\n  - id: gh_bb\n    name: 温室x蓝莓\n    models: [gh, bb]\n    links:\n      - { from: GREENHOUSE_V1.T_air, to: BLUEBERRY_BB5.T }\n",
        )
        .unwrap();
        let roster = build_roster_from_manifest(&ws).unwrap();
        assert_eq!(roster.len(), 3); // 2 模型 + 1 耦合
        let c = roster.iter().find(|m| m.id == "gh_bb").unwrap();
        let cp = c.coupling.as_ref().expect("应是耦合条目");
        assert_eq!(cp.paths.len(), 2);
        assert_eq!(cp.links, vec![("GREENHOUSE_V1.T_air".to_string(), "BLUEBERRY_BB5.T".to_string())]);
        assert!(coupled_guard(c).is_some()); // 耦合条目被仿真/录入守卫拦下
        let single = roster.iter().find(|m| m.id == "gh").unwrap();
        assert!(coupled_guard(single).is_none()); // 单模型放行

        // 引用不存在的模型 → 错误
        std::fs::write(&ws, "models:\n  - { id: gh, path: g.eq.yaml }\ncouplings:\n  - { id: x, models: [gh, nope] }\n").unwrap();
        assert!(build_roster_from_manifest(&ws).is_err());
        // <2 模型 → 错误
        std::fs::write(&ws, "models:\n  - { id: gh, path: g.eq.yaml }\ncouplings:\n  - { id: x, models: [gh] }\n").unwrap();
        assert!(build_roster_from_manifest(&ws).is_err());
        // 耦合 id 与模型 id 撞 → 错误
        std::fs::write(&ws, "models:\n  - { id: gh, path: g.eq.yaml }\n  - { id: bb, path: b.eq.yaml }\ncouplings:\n  - { id: gh, models: [gh, bb] }\n").unwrap();
        assert!(build_roster_from_manifest(&ws).is_err());
        let _ = std::fs::remove_dir_all(&dir);
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
