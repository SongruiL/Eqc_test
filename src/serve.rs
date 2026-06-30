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
//! - `/api/layout3d`   → 3D 拓扑力导向坐标 JSON（GA-5/GA-6；前端 3D 视图消费）
//! - `/api/simulate`   → 逐日仿真轨迹 JSON（需 `--drivers`）
//! - `/api/chart.svg?vars=Y,TDM` → 轨迹折线图 SVG（需 `--drivers`）
//! - `/api/optimize?spec=problem.yaml` → 跑优化，返回最优旋钮+收敛轨迹 JSON（与 `eqc optimize` 同结构）
//! - `/__version`      → 版本号（前端轮询，文件改动即整页刷新）
//!
//! 用极小的手写 HTTP（`std::net`，零新依赖）。监听模型文件 mtime，存盘即 +版本 → 自动刷新。

use std::collections::HashMap;
use std::io::{Read, Write};

use indexmap::IndexMap;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::dag::{build_dag, collapse_dag, DagLevel};
use crate::parser::{parse_directory, parse_file, parse_str};
use crate::report::{generate_report_leveled, ColorMode, LayoutKind};
use crate::schema::EquationFile;
use crate::sim::{simulate, simulate_coupled, CoupledInput, SimInput, SimOutput};

/// 打包进二进制的 Studio 前端页面（零构建步骤；以后可换成真正的 `frontend/` 构建产物）。
const STUDIO_HTML: &str = include_str!("serve_assets/studio.html");

/// 前端 v2（重构中，`docs/spec-studio-frontend-v2.md`）：`frontend/` 的 Vite+Svelte+TS 源码
/// 构建出的**单 HTML**（vite-plugin-singlefile 内联全部 JS/CSS）。按 spec §4，构建产物
/// `npm run build` 拷进此 committed 资产 → cargo build 不需 node、clone 即可编译。`/v2` served。
/// 当前是 P0 spike（一个面板）；P1 起扩成工作区外壳。改前端须 `cd frontend && npm run build` 重生成并提交。
const STUDIO_V2_HTML: &str = include_str!("serve_assets/studio_v2.html");

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

/// 耦合声明。两种：**视图专用**（`models`+`links`，画结构图）；**可仿真**（`fast`/`slow`/
/// `weather`/`feedback`/`fast_params`，能跑 `/api/couple` 耦合仿真+优化，且视图用 fast/slow+反馈
/// 注入 source → 画出**双向边**）。`fast` 存在 = 可仿真模式。
#[derive(serde::Deserialize)]
struct CouplingDecl {
    id: String,
    #[serde(default)]
    name: Option<String>,
    // —— 视图专用模式 ——
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    links: Vec<LinkDecl>,
    // —— 可仿真模式（fast 存在即启用）——
    #[serde(default)]
    fast: Option<String>,
    #[serde(default)]
    slow: Option<String>,
    #[serde(default)]
    weather: Option<String>,
    #[serde(default)]
    feedback: Vec<LinkDecl>,
    #[serde(default)]
    fast_params: HashMap<String, f64>,
    #[serde(default)]
    slow_params: HashMap<String, f64>,
    #[serde(default)]
    steps: Option<usize>,
}

/// 一条跨模型链接：`to` ← `from`。视图：仅 from/to；仿真链接额外带 `agg`/`scale`，
/// 反馈额外带 `scale`/`init`。
#[derive(serde::Deserialize)]
struct LinkDecl {
    from: String,
    to: String,
    #[serde(default)]
    agg: Option<String>,
    #[serde(default)]
    scale: Option<f64>,
    #[serde(default)]
    init: Option<f64>,
}

/// 耦合的运行态：视图参与文件 + 内存注入的 source 链接 + （可选）仿真配置。
struct Coupling {
    /// 视图参与的模型文件。
    paths: Vec<PathBuf>,
    /// 视图 source 注入 (from = "MOD.out", to = "MOD.in")；双向都列。
    links: Vec<(String, String)>,
    /// `Some` = 可仿真（`/api/couple` 跑 simulate_coupled）。
    sim: Option<CoupledSim>,
}

/// 可仿真耦合的运行态配置（预载好）。
struct CoupledSim {
    fast: PathBuf,
    slow: PathBuf,
    /// 室外驱动（步数, 名->序列）。
    weather: (usize, HashMap<String, Vec<f64>>),
    links: Vec<crate::sim::CoupledLink>,
    feedback: Vec<crate::sim::FeedbackLink>,
    fast_params: HashMap<String, f64>,
    slow_params: HashMap<String, f64>,
    steps: Option<usize>,
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

/// 服务上下文：模型花名册 + 版本号 + GP 异步任务表。单/多模型统一成一份 roster（≥1 条）。
struct Ctx {
    models: Vec<ModelEntry>,
    version: AtomicU64,
    /// GP 异步进化任务表（task_id → 状态/进度/结果）。后台线程更新、`/api/evolve/status` 读。
    tasks: Arc<Mutex<HashMap<String, GpTask>>>,
    /// task_id 计数器（无需 rand）。
    task_seq: AtomicU64,
}

/// 一个 GP 异步进化任务的状态（S4：后台线程跑、前端轮询）。
struct GpTask {
    status: &'static str, // "running" | "done" | "error"
    gen: usize,           // 当前代（进度回调更新）
    total_gens: usize,
    history: Vec<f64>,    // 每代归档最小误差（画收敛曲线）
    result: Option<String>, // 完成时的完整结果 JSON（与 /api/evolve 同结构）
    error: Option<String>,
}

/// 启动本地服务，阻塞运行直到进程退出（Ctrl+C）。
pub fn serve(
    path: &Path,
    port: u16,
    drivers_path: Option<&PathBuf>,
    params_path: Option<&PathBuf>,
    data_dir_arg: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 本地密钥文件（gitignored）：设一次 key/代理/模型，启动自动加载进 env（不覆盖已设的）。
    load_secret_file();
    match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => {
            let model = std::env::var("EQC_LLM_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".into());
            let proxy = std::env::var("EQC_LLM_PROXY").unwrap_or_else(|_| "直连".into());
            println!("🤖 AI 助手已配置（模型 {model}；出站 {proxy}）");
        }
        _ => println!("🤖 AI 助手未配置（设 ANTHROPIC_API_KEY 或写 .eqc-secret 后重启即可启用）"),
    }

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

    let ctx = Arc::new(Ctx {
        models,
        version: AtomicU64::new(1),
        tasks: Arc::new(Mutex::new(HashMap::new())),
        task_seq: AtomicU64::new(1),
    });

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
        // 大栈（64MB）：大模型（数百器官级方程，如 FSPM F4 的 48 果 621 方程）的 pass/eval 递归在
        // 默认 spawn 线程栈（Windows ~1MB）会溢出（CLI 主线程 8MB 不会）→ 显式放大，让 serve 像 CLI 一样扛大模型。
        let _ = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
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

    // 耦合条目（step 3 + Studio 耦合）：视图专用 或 可仿真（fast 存在）。
    for c in ws.couplings {
        if !valid_model_id(&c.id) {
            return Err(format!("非法耦合 id '{}'", c.id));
        }
        if !seen.insert(c.id.clone()) {
            return Err(format!("id 重复（模型/耦合）：'{}'", c.id));
        }
        let name = c.name.clone().unwrap_or_else(|| c.id.clone());

        let coupling = if let Some(fast_rel) = &c.fast {
            // —— 可仿真模式：fast/slow + agg 链接 + 反馈 + 天气 ——
            use crate::sim::{Agg, CoupledLink, FeedbackLink};
            let slow_rel = c.slow.as_ref().ok_or_else(|| format!("耦合 '{}' 有 fast 但缺 slow", c.id))?;
            let fast_path = resolve(fast_rel);
            let slow_path = resolve(slow_rel);
            let fast_file = parse_file(&fast_path).map_err(|e| format!("耦合 '{}' fast 解析失败: {e}", c.id))?;
            let slow_file = parse_file(&slow_path).map_err(|e| format!("耦合 '{}' slow 解析失败: {e}", c.id))?;
            let (fid, sid) = (fast_file.meta.id.clone(), slow_file.meta.id.clone());

            let mut view_links: Vec<(String, String)> = Vec::new();
            let mut sim_links: Vec<CoupledLink> = Vec::new();
            for l in &c.links {
                // 视图边：温室输出 → 作物输入（模块前缀）
                view_links.push((format!("{fid}.{}", l.from), format!("{sid}.{}", l.to)));
                sim_links.push(CoupledLink {
                    to: l.to.clone(),
                    from: l.from.clone(),
                    agg: Agg::parse(l.agg.as_deref().unwrap_or("mean"))
                        .ok_or_else(|| format!("耦合 '{}' 未知 agg '{:?}'", c.id, l.agg))?,
                    scale: l.scale.unwrap_or(1.0),
                });
            }
            let mut sim_fb: Vec<FeedbackLink> = Vec::new();
            for f in &c.feedback {
                // 视图边：作物输出 → 温室输入（双向，模块前缀）
                view_links.push((format!("{sid}.{}", f.from), format!("{fid}.{}", f.to)));
                sim_fb.push(FeedbackLink {
                    to: f.to.clone(),
                    from: f.from.clone(),
                    scale: f.scale.unwrap_or(1.0),
                    init: f.init.unwrap_or(0.0),
                });
            }
            let weather = match &c.weather {
                Some(w) => crate::scenario::load_drivers_csv(&resolve(w))
                    .map_err(|e| format!("耦合 '{}' 天气加载失败: {e}", c.id))?,
                None => return Err(format!("可仿真耦合 '{}' 缺 weather", c.id)),
            };
            Coupling {
                paths: vec![fast_path.clone(), slow_path.clone()],
                links: view_links,
                sim: Some(CoupledSim {
                    fast: fast_path,
                    slow: slow_path,
                    weather,
                    links: sim_links,
                    feedback: sim_fb,
                    fast_params: c.fast_params.clone(),
                    slow_params: c.slow_params.clone(),
                    steps: c.steps,
                }),
            }
        } else {
            // —— 视图专用模式（现状）——
            if c.models.len() < 2 {
                return Err(format!("耦合 '{}' 至少需 2 个模型（或用 fast/slow 可仿真模式）", c.id));
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
            Coupling { paths, links, sim: None }
        };

        out.push(ModelEntry {
            id: c.id,
            name,
            path: PathBuf::new(),
            drivers: None,
            params: None,
            data_dir: PathBuf::new(),
            coupling: Some(coupling),
        });
    }
    Ok(out)
}

/// 跑一次耦合条目的仿真（`/api/couple`）。`param_ov` 叠加在 fast/slow 固定参数上
/// （前缀 `gh:`→温室、`crop:`→作物；无前缀→两者都试）。
fn run_couple(m: &ModelEntry, param_ov: &HashMap<String, f64>) -> Result<crate::sim::CoupledOutput, String> {
    let sim = m
        .coupling
        .as_ref()
        .and_then(|c| c.sim.as_ref())
        .ok_or_else(|| "该条目不是可仿真耦合（清单需写 fast/slow/weather）".to_string())?;
    let fast = parse_file(&sim.fast).map_err(|e| e.to_string())?;
    let slow = parse_file(&sim.slow).map_err(|e| e.to_string())?;
    let (rows, wmap) = &sim.weather;
    let dtf = fast.meta.dt_seconds.ok_or("温室模型缺 meta.dt_seconds")?;
    let dts = slow.meta.dt_seconds.ok_or("作物模型缺 meta.dt_seconds")?;
    let r = (dts / dtf).round().max(1.0) as usize;
    let slow_steps = sim.steps.unwrap_or(rows / r);
    let need = slow_steps * r;
    if *rows < need {
        return Err(format!("室外天气 {rows} 行 < 慢步数·R = {need}"));
    }
    let weather: HashMap<String, Vec<f64>> =
        wmap.iter().map(|(k, v)| (k.clone(), v[..need.min(v.len())].to_vec())).collect();

    let mut input = CoupledInput::new(&fast, &slow, sim.links.clone(), weather, slow_steps);
    input.feedback = sim.feedback.clone();
    input.fast_params = sim.fast_params.clone();
    input.slow_params = sim.slow_params.clone();
    // 请求级覆盖：gh:name→温室、crop:name→作物
    for (k, v) in param_ov {
        if let Some(n) = k.strip_prefix("gh:") {
            input.fast_params.insert(n.to_string(), *v);
        } else if let Some(n) = k.strip_prefix("crop:") {
            input.slow_params.insert(n.to_string(), *v);
        }
    }
    simulate_coupled(&input).map_err(|e| format!("耦合仿真失败: {e}"))
}

/// `/api/couple-optimize?model=<耦合id>&spec=<决策spec路径>`：在耦合（双向）前向模型上跑 DE。
/// spec（knobs/objective）按 CWD（serve 启动目录）解析；耦合配置取自清单的该条目。
fn run_couple_optimize(m: &ModelEntry, query: &str) -> Result<String, String> {
    use crate::optimize::{load_problem, run_coupled, CoupledModel};

    let sim = m
        .coupling
        .as_ref()
        .and_then(|c| c.sim.as_ref())
        .ok_or_else(|| "该条目不是可仿真耦合（清单需写 fast/slow/weather）".to_string())?;
    let spec_arg = parse_spec(query)
        .ok_or_else(|| "缺少 spec 参数（/api/couple-optimize?spec=problem.yaml）".to_string())?;
    let problem = load_problem(&PathBuf::from(&spec_arg))?;

    let fast = parse_file(&sim.fast).map_err(|e| e.to_string())?;
    let slow = parse_file(&sim.slow).map_err(|e| e.to_string())?;
    let (rows, wmap) = &sim.weather;
    let dtf = fast.meta.dt_seconds.ok_or("温室模型缺 meta.dt_seconds")?;
    let dts = slow.meta.dt_seconds.ok_or("作物模型缺 meta.dt_seconds")?;
    let r = (dts / dtf).round().max(1.0) as usize;
    let slow_steps = sim.steps.unwrap_or(rows / r);
    let need = slow_steps * r;
    if *rows < need {
        return Err(format!("室外天气 {rows} 行 < 慢步数·R = {need}"));
    }
    let weather: HashMap<String, Vec<f64>> =
        wmap.iter().map(|(k, v)| (k.clone(), v[..need.min(v.len())].to_vec())).collect();

    let model = CoupledModel {
        fast: &fast,
        slow: &slow,
        links: sim.links.clone(),
        feedback: sim.feedback.clone(),
        weather,
        slow_steps,
        base_fast_params: sim.fast_params.clone(),
        base_slow_params: sim.slow_params.clone(),
    };
    let res = run_coupled(&model, &problem)?;

    let knobs: serde_json::Map<String, serde_json::Value> = problem
        .knobs
        .iter()
        .zip(&res.best_knobs)
        .map(|(k, v)| (k.var.clone(), serde_json::json!(v)))
        .collect();
    let j = serde_json::json!({
        "coupled": true,
        "best_objective": res.best_objective,
        "best_knobs": knobs,
        "objective": problem.objective.expr,
        "sense": problem.objective.sense.as_str(),
        "convergence_svg": crate::chart::convergence_chart_svg(&res.history, 720.0, 300.0),
    });
    Ok(j.to_string())
}

/// 把耦合输出（作物 slow + 温室 fast 日均）合成一份轨迹：作物变量原名，温室变量加 `温室:` 前缀。
fn couple_trajectories(out: &crate::sim::CoupledOutput) -> IndexMap<String, Vec<f64>> {
    let mut t: IndexMap<String, Vec<f64>> = IndexMap::new();
    for (k, v) in &out.slow.trajectories {
        t.insert(k.clone(), v.clone());
    }
    for (k, v) in &out.fast.trajectories {
        t.insert(format!("温室:{k}"), v.clone());
    }
    t
}

/// 轨迹键 → 友好图例标签：保留「温室:」前缀 + 向量「[i]」后缀，中间变量名按契约
/// `display_name` 翻译。与前端 studio.html 的 `coupleLabel` 同逻辑（Rust 这份服务端
/// 渲染 SVG 图例用——静态 SVG 没有 hover，故图例直接显友好名）。
fn trajectory_label(files: &[EquationFile], key: &str) -> String {
    let (prefix, rest) = match key.strip_prefix("温室:") {
        Some(r) => ("温室:", r),
        None => ("", key),
    };
    let (base, suffix) = match rest.rfind('[') {
        Some(p) if rest.ends_with(']') => (&rest[..p], &rest[p..]),
        _ => (rest, ""),
    };
    let disp = files
        .iter()
        .find(|f| {
            f.variables.contains_key(base)
                || f.parameters.contains_key(base)
                || f.equations.iter().any(|e| e.output == base)
        })
        .map(|f| f.display_name(base))
        .unwrap_or_else(|| base.to_string());
    format!("{prefix}{disp}{suffix}")
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
                "coupled": m.coupling.is_some(),
                "sim_capable": m.coupling.as_ref().is_some_and(|c| c.sim.is_some())
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

    // 流式 LLM：直接接管 TcpStream 逐块写 SSE（不走下面「组完整 body 再一次写」的常规路径）。
    if route == "/api/llm/stream" && method == "POST" {
        return proxy_llm_stream(&mut stream, &req_body);
    }

    // 当前模型（按 `?model=<id>`，缺省第一个）。/、/__version、/api/models 不依赖它，但解析无害。
    let m = resolve_model(ctx, query);

    let (status, ctype, body): (&str, &str, String) = match route {
        "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", STUDIO_HTML.to_string()),
        // 前端 v2（重构中）：Vite+Svelte+TS 构建的单 HTML，消费 live /api/*。studio.html(v1) 仍是默认 /。
        "/v2" => ("200 OK", "text/html; charset=utf-8", STUDIO_V2_HTML.to_string()),
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
        "/api/report" => match load_model_files(m).and_then(|f| render_report(&f, parse_layout(query), parse_dag_level(query), parse_color(query))) {
            Ok(h) => ("200 OK", "text/html; charset=utf-8", h),
            Err(e) => ("200 OK", "text/html; charset=utf-8", error_html(&e)),
        },
        // 3D 拓扑布局（GA-6）：每请求新鲜算 GA-5 力导向坐标（节点 size/community/depth + 边 + bound）。
        // 前端 Structure 工作区的 3D 视图消费；坐标 Rust 算、前端只渲染（守单一真相源）。
        "/api/layout3d" => match load_model_files(m) {
            Ok(files) => (
                "200 OK",
                "application/json; charset=utf-8",
                crate::export::layout3d_json_string(&crate::graph::layout3d(&files)),
            ),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // GA-6b 生长动画 plan（按子系统声明序的章节 + 旁白；2D/3D 同步消费）。
        "/api/growth" => match load_model_files(m) {
            Ok(files) => (
                "200 OK",
                "application/json; charset=utf-8",
                crate::export::growth_json_string(&files),
            ),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
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
                    let files = load_model_files(m).unwrap_or_default();
                    crate::chart::line_chart_svg(&out, &refs, 720.0, 360.0, |k| trajectory_label(&files, k))
                }
                Err(e) => error_svg(&e),
            };
            ("200 OK", "image/svg+xml; charset=utf-8", svg)
        }
        // 耦合仿真（可仿真耦合条目）：跑 simulate_coupled → 作物+温室合成轨迹。
        "/api/couple" => {
            let pv = parse_overrides(query, "p");
            match run_couple(m, &pv) {
                Ok(out) => {
                    let traj = couple_trajectories(&out);
                    let so = SimOutput { steps: out.slow_steps, trajectories: traj };
                    ("200 OK", "application/json; charset=utf-8", trajectory_json(&so))
                }
                Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
            }
        }
        "/api/couple.svg" => {
            let vars = parse_vars(query);
            let pv = parse_overrides(query, "p");
            let svg = match run_couple(m, &pv) {
                Ok(out) => {
                    let traj = couple_trajectories(&out);
                    let so = SimOutput { steps: out.slow_steps, trajectories: traj };
                    let refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
                    let files = load_model_files(m).unwrap_or_default();
                    crate::chart::line_chart_svg(&so, &refs, 720.0, 360.0, |k| trajectory_label(&files, k))
                }
                Err(e) => error_svg(&e),
            };
            ("200 OK", "image/svg+xml; charset=utf-8", svg)
        }
        "/api/couple-optimize" => match run_couple_optimize(m, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        "/api/optimize" => match run_optimize(m, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // 受约束 GP：在某 gp_target 靶点进化方程结构（同步 Pareto + 形式识别 + rediscovery）。
        "/api/evolve" => match run_evolve(m, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        // 异步 GP（S4）：起后台任务（放开 memetic/大规模）→ {task_id}；前端轮询 status 拿进度+结果。
        "/api/evolve/start" => match run_evolve_start(ctx, m, query) {
            Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
            Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
        },
        "/api/evolve/status" => (
            "200 OK",
            "application/json; charset=utf-8",
            run_evolve_status(ctx, query),
        ),
        // 模型源码（浏览器内编辑器 C1）：GET 取原文；POST 受控写回（先校验+自动备份+原子写）。
        "/api/source" => match method {
            "GET" => ("200 OK", "application/json; charset=utf-8", read_source(m)),
            "POST" => match write_source(m, &req_body) {
                Ok(j) => ("200 OK", "application/json; charset=utf-8", j),
                Err(e) => ("200 OK", "application/json; charset=utf-8", error_json(&e)),
            },
            _ => ("405 Method Not Allowed", "text/plain; charset=utf-8", "Method Not Allowed".to_string()),
        },
        "/api/validate" => match method {
            "POST" => ("200 OK", "application/json; charset=utf-8", run_validate(query, &req_body)),
            _ => ("405 Method Not Allowed", "text/plain; charset=utf-8", "Method Not Allowed".to_string()),
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
        // 前端 LLM Agent：薄代理 Claude（Anthropic Messages API）。前端跑 agent loop、组完整
        // 请求体（model/system+cache_control/tools/messages），后端只**注入 key + 转发**，key 绝不下发浏览器。
        // 失败统一返回 Anthropic 风格 {type:"error",error:{...}} 信封，前端一处处理、可降级。
        "/api/llm" => match method {
            "POST" => ("200 OK", "application/json; charset=utf-8", proxy_llm(&req_body)),
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

// ───────────────────────── 前端 LLM Agent 代理 ─────────────────────────

/// 本地密钥文件（gitignored）→ env。`EQC_SECRET_FILE` 指定路径，否则用 CWD `.eqc-secret`。
/// 格式：每行 `KEY=VALUE`（`#` 注释、空行忽略；值不去引号外的空白）。只设尚未存在的 env，
/// 即真·环境变量优先于文件。可放 ANTHROPIC_API_KEY / EQC_LLM_PROXY / EQC_LLM_MODEL。
fn load_secret_file() {
    let path = std::env::var("EQC_SECRET_FILE").unwrap_or_else(|_| ".eqc-secret".into());
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return, // 没有文件 = 正常（用真 env 或未配置）
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            if !k.is_empty() && std::env::var(k).is_err() {
                std::env::set_var(k, v);
            }
        }
    }
    println!("🔑 已加载密钥文件：{path}");
}

/// Anthropic 风格错误信封（前端与真·API 错误一处处理）。
fn llm_error(kind: &str, msg: &str) -> String {
    serde_json::json!({ "type": "error", "error": { "type": kind, "message": msg } }).to_string()
}

/// 出站 HTTPS Agent。`EQC_LLM_PROXY`（如 `http://127.0.0.1:10808`）设了就走代理，否则直连。
/// （本机实测：直连被 403 地理封锁，必须走本地代理；见 tests/llm_spike.rs。）
fn build_llm_agent() -> ureq::Agent {
    let b = ureq::AgentBuilder::new();
    if let Ok(p) = std::env::var("EQC_LLM_PROXY") {
        if !p.is_empty() {
            if let Ok(proxy) = ureq::Proxy::new(&p) {
                return b.proxy(proxy).build();
            }
        }
    }
    b.build()
}

/// `POST /api/llm`：把前端组好的完整 Anthropic 请求体注入 key 后转发，原样回传 Claude 的 JSON。
/// key 取自 env `ANTHROPIC_API_KEY`（不进浏览器/repo）；缺失→友好错误，前端禁用 AI、其余照常。
fn proxy_llm(body: &[u8]) -> String {
    if mock_enabled() {
        let (blocks, stop, err) = build_mock(body);
        return match err {
            Some(e) => llm_error("mock_error", &e),
            None => serde_json::json!({"type":"message","role":"assistant","model":"mock","content":blocks,"stop_reason":stop,"usage":{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":0}}).to_string(),
        };
    }
    let key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return llm_error("missing_api_key", "后端未配置 ANTHROPIC_API_KEY；设置该环境变量后重启 serve 即可启用 AI。"),
    };
    let read_resp = |r: ureq::Response| -> String {
        r.into_string().unwrap_or_else(|e| llm_error("read_error", &format!("读取 Claude 响应失败: {e}")))
    };
    // 模型一行 env 切换（EQC_LLM_MODEL）：覆盖前端请求体里的 model，无需重建前端。
    let payload: Vec<u8> = match std::env::var("EQC_LLM_MODEL") {
        Ok(m) if !m.is_empty() => match serde_json::from_slice::<serde_json::Value>(body) {
            Ok(mut v) => {
                v["model"] = serde_json::Value::String(m);
                serde_json::to_vec(&v).unwrap_or_else(|_| body.to_vec())
            }
            Err(_) => body.to_vec(),
        },
        _ => body.to_vec(),
    };
    let req = build_llm_agent()
        .post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", &key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .timeout(Duration::from_secs(120));
    match req.send_bytes(&payload) {
        Ok(resp) => read_resp(resp),
        // 4xx/5xx：Anthropic 已返回 {type:"error",...} JSON，原样透传给前端。
        Err(ureq::Error::Status(_code, resp)) => read_resp(resp),
        Err(ureq::Error::Transport(t)) => {
            llm_error("upstream_unreachable", &format!("连接 Claude 失败（检查网络/代理 EQC_LLM_PROXY）：{t}"))
        }
    }
}

/// `POST /api/llm/stream`：同 proxy_llm，但**强制 `stream:true` 并把上游 SSE 原样透传**给浏览器
/// （`Connection: close`，逐块 flush；前端用 fetch 流式 reader 解析）。错误也以一条 SSE `data:` 事件下发。
fn proxy_llm_stream(stream: &mut TcpStream, body: &[u8]) -> std::io::Result<()> {
    if mock_enabled() {
        return mock_stream(stream, body);
    }
    // 写 SSE 头 + 一条事件（用于在「尚未透传上游流」时下发错误/上游错误 JSON）。
    fn sse_once(stream: &mut TcpStream, data: &str) -> std::io::Result<()> {
        stream.write_all(SSE_HEAD.as_bytes())?;
        stream.write_all(format!("data: {data}\n\n").as_bytes())?;
        stream.flush()
    }

    let key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return sse_once(stream, &llm_error("missing_api_key", "后端未配置 ANTHROPIC_API_KEY；设置后重启 serve 即可启用 AI。")),
    };
    let mut v: serde_json::Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return sse_once(stream, &llm_error("bad_request", "请求体不是合法 JSON")),
    };
    v["stream"] = serde_json::Value::Bool(true);
    if let Ok(m) = std::env::var("EQC_LLM_MODEL") {
        if !m.is_empty() {
            v["model"] = serde_json::Value::String(m);
        }
    }
    let payload = serde_json::to_vec(&v).unwrap_or_else(|_| body.to_vec());

    let req = build_llm_agent()
        .post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", &key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .timeout(Duration::from_secs(300));
    let resp = match req.send_bytes(&payload) {
        Ok(r) => r,
        // 4xx/5xx：上游返回的是非流式错误 JSON（已是 {type:error,...}）→ 包成一条 SSE 事件透传。
        Err(ureq::Error::Status(_c, r)) => {
            let txt = r.into_string().unwrap_or_default();
            let data = if txt.trim().is_empty() { llm_error("upstream_error", "Claude 返回错误") } else { txt };
            return sse_once(stream, &data);
        }
        Err(ureq::Error::Transport(t)) => {
            return sse_once(stream, &llm_error("upstream_unreachable", &format!("连接 Claude 失败（检查网络/代理 EQC_LLM_PROXY）：{t}")));
        }
    };

    // 透传上游 SSE：写头后逐块 read→write→flush，直到上游关闭。
    stream.write_all(SSE_HEAD.as_bytes())?;
    stream.flush()?;
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                stream.write_all(&buf[..n])?;
                stream.flush()?;
            }
            Err(_) => break, // 浏览器断开/上游中断 → 收尾
        }
    }
    stream.flush()
}

/// SSE 响应头（流式 LLM 与 mock 共用）。
const SSE_HEAD: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n";

// ───────────────────────── Mock LLM（确定性 e2e 用） ─────────────────────────

/// `EQC_LLM_MOCK` 非空即启用 mock：不调 Anthropic，按请求里的 `[[MOCK …]]` 指令确定性作答。
fn mock_enabled() -> bool {
    std::env::var("EQC_LLM_MOCK").map(|x| !x.is_empty()).unwrap_or(false)
}

/// 从最后一条 user 消息读指令，确定性产出助手内容块 + stop_reason（+ 可选错误）：
/// - 最后一条是 tool_result（content 为数组）→ 一句文本 + `end_turn`（结束 loop）。
/// - 文本含 `[[MOCK_ERROR 信息]]` → 返回错误。
/// - 文本含一个或多个 `[[MOCK 工具名 {json入参}]]` → 对应 tool_use（多条=并行）+ `tool_use`。
/// - 否则 → 一句文本 + `end_turn`。
/// 这样测试用例自描述「模型该调哪个工具」，驱动**真**前端 loop/handler/confirm，零成本可重复。
fn build_mock(body: &[u8]) -> (Vec<serde_json::Value>, String, Option<String>) {
    use serde_json::json;
    let v: serde_json::Value = serde_json::from_slice(body).unwrap_or(json!({}));
    let last = v.get("messages").and_then(|m| m.as_array()).and_then(|a| a.last());
    let content = last.and_then(|m| m.get("content"));
    // tool_result 轮：数组里含 tool_result 块 → 结束。（注意：#3 缓存把末条 user 字符串也包成
    // text 块数组，所以「是数组」不等于 tool_result，必须按块 type 判定。）
    let blocks_arr = content.and_then(|c| c.as_array());
    let is_tool_result = blocks_arr
        .map(|a| a.iter().any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result")))
        .unwrap_or(false);
    if is_tool_result {
        return (vec![json!({"type":"text","text":"完成 ✅（mock）"})], "end_turn".into(), None);
    }
    // 取用户文本：字符串内容，或数组里 text 块拼接（覆盖缓存包装后的形态）。
    let owned;
    let text: &str = if let Some(s) = content.and_then(|c| c.as_str()) {
        s
    } else if let Some(a) = blocks_arr {
        owned = a
            .iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        owned.as_str()
    } else {
        ""
    };
    if let Some(s) = text.find("[[MOCK_ERROR") {
        let msg = text[s + "[[MOCK_ERROR".len()..].split("]]").next().unwrap_or("").trim();
        return (vec![], String::new(), Some(if msg.is_empty() { "mock 错误".into() } else { msg.to_string() }));
    }
    let mut blocks = vec![];
    let mut rest = text;
    while let Some(s) = rest.find("[[MOCK ") {
        let after = &rest[s + "[[MOCK ".len()..];
        let end = match after.find("]]") {
            Some(e) => e,
            None => break,
        };
        let spec = after[..end].trim();
        rest = &after[end + 2..];
        let (name, inp) = match spec.find('{') {
            Some(b) => (spec[..b].trim().to_string(), spec[b..].trim().to_string()),
            None => (spec.to_string(), "{}".to_string()),
        };
        let input: serde_json::Value = serde_json::from_str(&inp).unwrap_or(json!({}));
        blocks.push(json!({"type":"tool_use","id":format!("mock_{}", blocks.len()),"name":name,"input":input}));
    }
    if blocks.is_empty() {
        return (vec![json!({"type":"text","text":"（mock）我已就绪。"})], "end_turn".into(), None);
    }
    (blocks, "tool_use".into(), None)
}

/// mock 的流式 SSE：把 build_mock 的内容块拆成 Anthropic SSE 事件序列写给浏览器。
fn mock_stream(stream: &mut TcpStream, body: &[u8]) -> std::io::Result<()> {
    use serde_json::json;
    let (blocks, stop, err) = build_mock(body);
    stream.write_all(SSE_HEAD.as_bytes())?;
    let mut send = |data: serde_json::Value| -> std::io::Result<()> {
        stream.write_all(format!("data: {}\n\n", data).as_bytes())?;
        stream.flush()
    };
    if let Some(e) = err {
        return send(json!({"type":"error","error":{"type":"mock_error","message":e}}));
    }
    send(json!({"type":"message_start","message":{"model":"mock","role":"assistant","content":[],"usage":{"input_tokens":1,"output_tokens":1}}}))?;
    for (i, b) in blocks.iter().enumerate() {
        if b["type"] == "text" {
            send(json!({"type":"content_block_start","index":i,"content_block":{"type":"text","text":""}}))?;
            send(json!({"type":"content_block_delta","index":i,"delta":{"type":"text_delta","text":b["text"]}}))?;
        } else {
            send(json!({"type":"content_block_start","index":i,"content_block":{"type":"tool_use","id":b["id"],"name":b["name"],"input":{}}}))?;
            let pj = serde_json::to_string(&b["input"]).unwrap_or_else(|_| "{}".into());
            send(json!({"type":"content_block_delta","index":i,"delta":{"type":"input_json_delta","partial_json":pj}}))?;
        }
        send(json!({"type":"content_block_stop","index":i}))?;
    }
    send(json!({"type":"message_delta","delta":{"stop_reason":stop}}))?;
    send(json!({"type":"message_stop"}))
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

fn render_report(
    files: &[EquationFile],
    layout: LayoutKind,
    level: DagLevel,
    color: ColorMode,
) -> Result<String, String> {
    let dag = build_dag(files).map_err(|e| e.to_string())?;
    let collapsed = collapse_dag(&dag, files, level);
    Ok(generate_report_leveled(files, &collapsed, layout, level, color))
}

/// `?color=module` → 按子系统配色（未提供/未知 → 按类别）。与 3D `topoColorMode` 对齐。
fn parse_color(query: &str) -> ColorMode {
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("color=") {
            return ColorMode::parse(&url_decode(v));
        }
    }
    ColorMode::Class
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

/// `/api/evolve?model=&target=&zone=&pop=&gens=&seed=&pareto=&parsimony=&observed=&baseline_form=`
///
/// 受约束 GP 面板的后端（spec-gp-studio §4，S1）：在某 `gp_target` 靶点进化方程结构，返回
/// **Pareto 前沿**（精度 vs 复杂度）+ 每点的拟合轨迹/机理形式识别/rediscovery 判定，外加现有形式
/// （baseline）对比、观测散点、前沿散点 SVG。靶点元数据（语法/输入/输出/边界/单调）从模型
/// `gp_target` 自动取（G0）——前端面板只递交「选哪个靶 + 几个进化旋钮」。observed 取本处理区录入
/// 的稀疏 CSV（录入→GP 同源），缺则 `?observed=` 文件兜底。**memetic 不在 S1**（计算量大，待异步 S4）。
/// 薄编排：`evolve_pareto` + `form_report` + `patch_model`/`simulate`，所有计算在 Rust 侧。
fn run_evolve(m: &ModelEntry, query: &str) -> Result<String, String> {
    // 同步端点：memetic 计算量大 → 仍挡在外（请用异步 /api/evolve/start）。
    if query_flag(query, "memetic") {
        return Err(
            "memetic（内层 DE 标定常数）计算量大——同步端点不支持，请用异步任务 /api/evolve/start"
                .into(),
        );
    }
    let job = prepare_evolve(m, query)?;
    Ok(run_evolve_job(&job, &mut |_, _| {}))
}

/// GP 进化任务的全部输入（同步 setup 产物；都是 owned 数据、Send，可移进后台线程跑）。
struct EvolveJob {
    file: EquationFile,
    target: String,
    output: String,
    eqname: String,
    grammar: String,
    ctx: crate::gp::GpContext,
    unit_env: HashMap<String, crate::units::Dimension>,
    sim_input: SimInput,
    observed_data: crate::gp::Observed,
    obs_series: Vec<(usize, f64)>,
    pcfg: crate::gp::ParetoConfig,
    baseline_form: Option<String>,
}

/// 同步快速 setup（解析模型/靶点/驱动/观测/量纲/配置）→ `EvolveJob`。失败立即返错。
/// 同步与异步端点共用；跑 `evolve_pareto` 的重活在 `run_evolve_job`（可在后台线程跑）。
fn prepare_evolve(m: &ModelEntry, query: &str) -> Result<EvolveJob, String> {
    use crate::gp;
    use crate::units::{parse_dimension, Dimension};

    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }

    let target = query_get(query, "target")
        .ok_or_else(|| "缺少 target 参数（/api/evolve?target=<方程id>）".to_string())?;
    let zone = parse_zone(query);

    let files = load_files(&m.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?;

    // 靶点元数据（语法/输入/边界/单调）从模型 gp_target 自动取（G0）。
    let (grammar, ctx) = gp::context_from_target(file, &target)
        .ok_or_else(|| format!("方程 {target} 无 gp_target（不是进化靶点）"))?;
    // 拟合输出 = 该方程的 output 变量。
    let output = file
        .equations
        .iter()
        .find(|e| e.id == target)
        .map(|e| e.output.clone())
        .ok_or_else(|| format!("找不到方程 {target}"))?;

    // 驱动量（模型预载）+ steps。
    let (steps_default, driver_map) = m
        .drivers
        .as_ref()
        .map(|(r, map)| (*r, map.clone()))
        .ok_or_else(|| {
            "该模型未预载驱动量（启动 --drivers 或清单声明）——GP 无法仿真".to_string()
        })?;
    let steps = query_usize(query, "steps").unwrap_or(steps_default);

    // 观测：本处理区录入 CSV 优先（录入→GP 同源），否则 ?observed= 文件兜底。
    let zone_csv = m.data_dir.join(format!("{zone}.csv"));
    let obs_path: PathBuf = if zone_csv.exists() {
        zone_csv
    } else if let Some(o) = query_get(query, "observed") {
        let p = PathBuf::from(&o);
        if p.is_absolute() {
            p
        } else {
            let model_dir = if m.path.is_dir() {
                m.path.clone()
            } else {
                m.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
            };
            model_dir.join(&o)
        }
    } else {
        return Err(format!(
            "处理区 '{zone}' 暂无实测数据，且未提供 ?observed=——请先在园区视图录入并保存，或指定 observed 文件"
        ));
    };
    let observed_data = crate::scenario::load_observed_csv(&obs_path).map_err(|e| e.to_string())?;
    let obs_series = observed_data
        .get(&output)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("观测数据里没有目标输出 '{output}' 的点——GP 无拟合目标"))?
        .clone();

    let mut sim_input = SimInput::new(steps);
    sim_input.drivers = driver_map;
    if let Some(p) = &m.params {
        sim_input.param_overrides = p.clone();
    }

    // 量纲环境（GP 量纲软过滤用）。
    let mut unit_env: HashMap<String, Dimension> = HashMap::new();
    for (n, p) in &file.parameters {
        if let Some(u) = &p.unit {
            if let Some(d) = parse_dimension(u) {
                unit_env.insert(n.clone(), d);
            }
        }
    }
    for (n, v) in &file.variables {
        if let Some(u) = &v.unit {
            if let Some(d) = parse_dimension(u) {
                unit_env.insert(n.clone(), d);
            }
        }
    }

    // 进化配置（带默认；Pareto 是 GP 招牌输出，恒走 Pareto）。
    let pop = query_usize(query, "pop").unwrap_or(60);
    let gens = query_usize(query, "gens").unwrap_or(40);
    let seed = query_usize(query, "seed").map(|s| s as u64).unwrap_or(1);
    let sweep_hi = query_f64(query, "sweep_hi").unwrap_or(50.0);
    let archive_cap = query_usize(query, "archive_cap").unwrap_or(24);
    // memetic（内层 DE 标定常数）：异步端点放开；缺省 co-evolve（同步端点已在外层挡掉 memetic）。
    let memetic = query_flag(query, "memetic").then(|| crate::optimize::DeConfig {
        pop: query_usize(query, "memetic_pop").unwrap_or(16),
        iters: query_usize(query, "memetic_iters").unwrap_or(30),
        seed: 1,
        f: 0.6,
        cr: 0.9,
    });
    // baseline_form：query 显式值优先；否则自动识别当前方程的机理形式（B2）——面板不传也能判
    // rediscovery（GP 撞回现有形式=机理验证）。识别不出（手写形式超语法）→ None。
    let baseline_form = query_get(query, "baseline_form").or_else(|| {
        file.equations
            .iter()
            .find(|e| e.id == target)
            .and_then(|e| gp::identify_form_of_expr(&e.expression, &grammar, &ctx))
            .map(|i| gp::form_name(&grammar, i).to_string())
    });
    // 目标方程友好名（采纳 YAML 片段用）。
    let eqname = file
        .equations
        .iter()
        .find(|e| e.id == target)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| target.clone());

    let pcfg = gp::ParetoConfig { pop, gens, seed, sweep_hi, archive_cap, memetic };
    Ok(EvolveJob {
        file: file.clone(),
        target,
        output,
        eqname,
        grammar,
        ctx,
        unit_env,
        sim_input,
        observed_data,
        obs_series,
        pcfg,
        baseline_form,
    })
}

/// 跑 GP 进化（`evolve_pareto_cb` + 拼结果 JSON）；`progress(gen,best)` 供异步任务画收敛曲线，
/// 同步端点传 no-op。返回与 spec §4 同结构的结果 JSON 字符串（前沿+采纳产物+baseline+observed+SVG）。
fn run_evolve_job(job: &EvolveJob, progress: &mut dyn FnMut(usize, f64)) -> String {
    use crate::gp;
    let file = &job.file;
    let target = job.target.as_str();
    let output = job.output.as_str();
    let grammar = job.grammar.as_str();
    let eqname = job.eqname.as_str();
    let ctx = &job.ctx;
    let unit_env = &job.unit_env;
    let sim_input = &job.sim_input;
    let observed_data = &job.observed_data;
    let obs_series = &job.obs_series;
    let baseline_form = job.baseline_form.as_deref();

    let front = gp::evolve_pareto_cb(
        grammar,
        ctx,
        unit_env,
        &job.pcfg,
        |cand| gp::evaluate_in_model(file, target, output, cand, sim_input, observed_data),
        progress,
    );

    // 每个前沿点：机理形式识别 + rediscovery 判定 + 拟合轨迹 + 采纳产物（溯源草稿 + YAML 片段）。
    let front_json: Vec<serde_json::Value> = front
        .iter()
        .map(|e| {
            let report = gp::form_report(&e.cand, e.error, e.complexity, grammar, ctx, baseline_form);
            let traj = candidate_trajectory(file, target, output, &e.cand, sim_input);
            let stub = gp::provenance_stub(&report, target, output, grammar);
            let yaml_fragment = candidate_yaml_fragment(target, eqname, output, &e.cand);
            let structure_diff = gp_structure_diff(file, target, &e.cand);
            serde_json::json!({
                "complexity": e.complexity,
                "error": e.error,
                "consts": e.cand.consts,
                "formula": gp::render_formula(&e.cand),
                "formula_mathml": crate::report::expr_mathml(&candidate_expr(&e.cand)),
                "mechanistic_form": report.form,
                "rediscovery": report.rediscovery,
                "provenance_suggestion": report.suggestion,
                "trajectory": traj,
                "provenance_stub": stub,
                "yaml_fragment": yaml_fragment,
                "structure_diff": structure_diff,
            })
        })
        .collect();

    // baseline：现模型当前形式（未 patch 的原方程）+ 其仿真轨迹 + rmse/复杂度（与候选并排对比）。
    let baseline = {
        let eq = file.equations.iter().find(|e| e.id == target);
        let formula = eq.map(|e| e.expression.to_python("")).unwrap_or_default();
        let formula_mathml = eq.map(|e| crate::report::expr_mathml(&e.expression)).unwrap_or_default();
        let complexity = eq.map(|e| gp::complexity(&e.expression));
        let out = simulate(file, sim_input).ok();
        let error = out.as_ref().and_then(|o| rmse_on_obs(o.series(output), obs_series));
        let traj = match &out {
            Some(o) => series_to_traj(o, output),
            None => serde_json::Value::Null,
        };
        serde_json::json!({
            "formula": formula, "formula_mathml": formula_mathml,
            "form": baseline_form, "trajectory": traj,
            "error": error, "complexity": complexity,
        })
    };

    let observed_json = obs_to_traj(obs_series);
    // 前沿散点 SVG：x=复杂度、y=rmse（复用 pareto_chart_svg，点可点击 data-i）。
    let pts: Vec<(f64, f64)> = front.iter().map(|e| (e.complexity as f64, e.error)).collect();
    let pareto_svg = crate::chart::pareto_chart_svg(&pts, "复杂度(节点)", "拟合误差(rmse)", 700.0, 380.0);

    serde_json::json!({
        "target": target,
        "output": output,
        "grammar": grammar,
        "mode": if job.pcfg.memetic.is_some() { "Pareto+memetic" } else { "Pareto" },
        "n_obs": obs_series.len(),
        "pareto_front": front_json,
        "baseline": baseline,
        "observed": observed_json,
        "pareto_svg": pareto_svg,
    })
    .to_string()
}

/// 多槽位联合进化任务的输入（S5；同步 setup 产物，Send）。每槽对齐 outputs/eqnames/baseline_forms。
struct JointJob {
    file: EquationFile,
    slots: Vec<crate::gp::Slot>,
    outputs: Vec<String>,
    eqnames: Vec<String>,
    baseline_forms: Vec<Option<String>>,
    unit_env: HashMap<String, crate::units::Dimension>,
    sim_input: SimInput,
    observed_data: crate::gp::Observed,
    jcfg: crate::gp::JointConfig,
}

/// 同步 setup：`targets=` 子集（"all"/空→全部 gp_target）→ 多槽位 `JointJob`。
/// observed **过滤到各槽输出**（联合适应度=各槽输出平均 rmse）；缺任一槽输出观测 → 报错。
fn prepare_evolve_joint(m: &ModelEntry, query: &str) -> Result<JointJob, String> {
    use crate::gp;
    use crate::units::{parse_dimension, Dimension};

    if let Some(e) = coupled_guard(m) {
        return Err(e);
    }
    let files = load_files(&m.path)?;
    let file = files.first().ok_or_else(|| "无模型".to_string())?.clone();

    // targets=A,B（"all"/空 → 全部 gp_target 槽位）
    let only: Option<Vec<String>> = query_get(query, "targets").and_then(|s| {
        let ids: Vec<String> = s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty() && x != "all")
            .collect();
        (!ids.is_empty()).then_some(ids)
    });
    let slots = gp::slots_from_model(&file, only.as_deref());
    if slots.is_empty() {
        return Err("没有可联合进化的 gp_target 槽位（检查 targets= 与模型标注）".into());
    }

    // 逐槽：输出变量 / 方程名 / baseline_form（B2 自动识别）。
    let mut outputs = Vec::new();
    let mut eqnames = Vec::new();
    let mut baseline_forms = Vec::new();
    for slot in &slots {
        let eq = file
            .equations
            .iter()
            .find(|e| e.id == slot.target_id)
            .ok_or_else(|| format!("找不到方程 {}", slot.target_id))?;
        outputs.push(eq.output.clone());
        eqnames.push(eq.name.clone());
        baseline_forms.push(
            gp::identify_form_of_expr(&eq.expression, &slot.grammar, &slot.ctx)
                .map(|i| gp::form_name(&slot.grammar, i).to_string()),
        );
    }

    // 驱动量 + steps。
    let (steps_default, driver_map) = m
        .drivers
        .as_ref()
        .map(|(r, map)| (*r, map.clone()))
        .ok_or_else(|| "该模型未预载驱动量——GP 无法仿真".to_string())?;
    let steps = query_usize(query, "steps").unwrap_or(steps_default);

    // 观测：本处理区 CSV 优先，否则 ?observed= 兜底。
    let zone = parse_zone(query);
    let zone_csv = m.data_dir.join(format!("{zone}.csv"));
    let obs_path: PathBuf = if zone_csv.exists() {
        zone_csv
    } else if let Some(o) = query_get(query, "observed") {
        let p = PathBuf::from(&o);
        if p.is_absolute() {
            p
        } else {
            let model_dir = if m.path.is_dir() {
                m.path.clone()
            } else {
                m.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
            };
            model_dir.join(&o)
        }
    } else {
        return Err(format!("处理区 '{zone}' 暂无实测数据，且未提供 ?observed="));
    };
    let all_observed = crate::scenario::load_observed_csv(&obs_path).map_err(|e| e.to_string())?;
    // 过滤到各槽输出（联合适应度只看这些）；缺任一 → 报错列出。
    let mut observed_data: crate::gp::Observed = HashMap::new();
    let mut missing = Vec::new();
    for output in &outputs {
        match all_observed.get(output) {
            Some(v) if !v.is_empty() => {
                observed_data.insert(output.clone(), v.clone());
            }
            _ => missing.push(output.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "以下靶点输出在处理区 '{zone}' 无观测：{}——请先在园区视图录入各输出变量",
            missing.join("、")
        ));
    }

    let mut sim_input = SimInput::new(steps);
    sim_input.drivers = driver_map;
    if let Some(p) = &m.params {
        sim_input.param_overrides = p.clone();
    }

    let mut unit_env: HashMap<String, Dimension> = HashMap::new();
    for (n, p) in &file.parameters {
        if let Some(u) = &p.unit {
            if let Some(d) = parse_dimension(u) {
                unit_env.insert(n.clone(), d);
            }
        }
    }
    for (n, v) in &file.variables {
        if let Some(u) = &v.unit {
            if let Some(d) = parse_dimension(u) {
                unit_env.insert(n.clone(), d);
            }
        }
    }

    let pop = query_usize(query, "pop").unwrap_or(60);
    let gens = query_usize(query, "gens").unwrap_or(40);
    let seed = query_usize(query, "seed").map(|s| s as u64).unwrap_or(1);
    let sweep_hi = query_f64(query, "sweep_hi").unwrap_or(50.0);
    let archive_cap = query_usize(query, "archive_cap").unwrap_or(24);
    let jcfg = gp::JointConfig { pop, gens, seed, sweep_hi, archive_cap, ..Default::default() };

    Ok(JointJob { file, slots, outputs, eqnames, baseline_forms, unit_env, sim_input, observed_data, jcfg })
}

/// 跑多槽位联合 Pareto（`evolve_joint_pareto_cb`）+ 拼结果 JSON。每前沿点=整模型一套配置：
/// `patch_multi` 全槽→**一次仿真**→逐槽抽 trajectory/rmse + form_report(rediscovery)/stub/yaml_fragment。
fn run_evolve_joint_job(job: &JointJob, progress: &mut dyn FnMut(usize, f64)) -> String {
    use crate::gp;
    let file = &job.file;
    let slots = &job.slots;
    let sim_input = &job.sim_input;
    let observed_data = &job.observed_data;
    let obs_of = |output: &str| -> &[(usize, f64)] {
        observed_data.get(output).map(|v| v.as_slice()).unwrap_or(&[])
    };

    let front = gp::evolve_joint_pareto_cb(
        slots,
        &job.unit_env,
        &job.jcfg,
        |genome| gp::evaluate_multi(file, slots, genome, sim_input, observed_data),
        progress,
    );

    // baselines（共享，未 patch 原模型跑一次）：逐槽现有形式 formula/form/trajectory/rmse/复杂度。
    let base_out = simulate(file, sim_input).ok();
    let mut baselines = serde_json::Map::new();
    let mut observed_map = serde_json::Map::new();
    for (k, slot) in slots.iter().enumerate() {
        let output = job.outputs[k].as_str();
        let eq = file.equations.iter().find(|e| e.id == slot.target_id);
        let formula = eq.map(|e| e.expression.to_python("")).unwrap_or_default();
        let formula_mathml = eq.map(|e| crate::report::expr_mathml(&e.expression)).unwrap_or_default();
        let complexity = eq.map(|e| gp::complexity(&e.expression));
        let (traj, error) = match &base_out {
            Some(o) => (series_to_traj(o, output), rmse_on_obs(o.series(output), obs_of(output))),
            None => (serde_json::Value::Null, None),
        };
        baselines.insert(
            slot.target_id.clone(),
            serde_json::json!({
                "formula": formula, "formula_mathml": formula_mathml,
                "form": job.baseline_forms[k], "trajectory": traj,
                "error": error, "complexity": complexity,
            }),
        );
        observed_map.insert(output.to_string(), obs_to_traj(obs_of(output)));
    }

    // 每前沿点（整模型一套配置）。
    let front_json: Vec<serde_json::Value> = front
        .iter()
        .map(|entry| {
            let out = gp::patch_multi(file, slots, &entry.genome).and_then(|m| simulate(&m, sim_input).ok());
            let slots_json: Vec<serde_json::Value> = slots
                .iter()
                .enumerate()
                .map(|(k, slot)| {
                    let cand = &entry.genome[k];
                    let output = job.outputs[k].as_str();
                    let cplx = gp::complexity(&cand.expr);
                    let (traj, error) = match &out {
                        Some(o) => (series_to_traj(o, output), rmse_on_obs(o.series(output), obs_of(output))),
                        None => (serde_json::Value::Null, None),
                    };
                    let report = gp::form_report(
                        cand, error.unwrap_or(0.0), cplx, &slot.grammar, &slot.ctx,
                        job.baseline_forms[k].as_deref(),
                    );
                    let stub = gp::provenance_stub(&report, &slot.target_id, output, &slot.grammar);
                    let yaml_fragment = candidate_yaml_fragment(&slot.target_id, &job.eqnames[k], output, cand);
                    let structure_diff = gp_structure_diff(file, &slot.target_id, cand);
                    serde_json::json!({
                        "target": slot.target_id, "output": output,
                        "formula": gp::render_formula(cand),
                        "formula_mathml": crate::report::expr_mathml(&candidate_expr(cand)),
                        "mechanistic_form": report.form,
                        "rediscovery": report.rediscovery,
                        "provenance_suggestion": report.suggestion,
                        "error": error, "complexity": cplx,
                        "trajectory": traj,
                        "provenance_stub": stub, "yaml_fragment": yaml_fragment,
                        "structure_diff": structure_diff,
                    })
                })
                .collect();
            serde_json::json!({ "complexity": entry.complexity, "error": entry.error, "slots": slots_json })
        })
        .collect();

    let pts: Vec<(f64, f64)> = front.iter().map(|e| (e.complexity as f64, e.error)).collect();
    let pareto_svg = crate::chart::pareto_chart_svg(&pts, "总复杂度(节点)", "平均误差(rmse)", 700.0, 380.0);

    serde_json::json!({
        "mode": "joint-pareto",
        "joint": true,
        "targets": slots.iter().map(|s| s.target_id.clone()).collect::<Vec<_>>(),
        "n_obs": observed_data.values().map(|v| v.len()).sum::<usize>(),
        "pareto_front": front_json,
        "baselines": baselines,
        "observed": observed_map,
        "pareto_svg": pareto_svg,
    })
    .to_string()
}

/// 在任务表登记一个新任务，返回 task_id。
fn new_gp_task(ctx: &Ctx, total_gens: usize) -> String {
    let id = format!("gp{}", ctx.task_seq.fetch_add(1, Ordering::SeqCst));
    if let Ok(mut t) = ctx.tasks.lock() {
        t.insert(
            id.clone(),
            GpTask { status: "running", gen: 0, total_gens, history: vec![], result: None, error: None },
        );
    }
    id
}

/// 后台线程跑完写回结果 + 标 done。`run` 是 `FnOnce(&mut progress) -> String`（单/多槽通用）。
fn spawn_gp_task<R>(tasks: Arc<Mutex<HashMap<String, GpTask>>>, id: String, run: R)
where
    R: FnOnce(&mut dyn FnMut(usize, f64)) -> String + Send + 'static,
{
    std::thread::spawn(move || {
        let t1 = Arc::clone(&tasks);
        let id1 = id.clone();
        let json = run(&mut move |gen, best| {
            if let Ok(mut t) = t1.lock() {
                if let Some(task) = t.get_mut(&id1) {
                    task.gen = gen;
                    task.history.push(best);
                }
            }
        });
        if let Ok(mut t) = tasks.lock() {
            if let Some(task) = t.get_mut(&id) {
                task.status = "done";
                task.result = Some(json);
            }
        }
    });
}

/// `/api/evolve/start?...&memetic=` / `&targets=`：异步起一个 GP 进化后台任务，立即返回 `{task_id}`。
/// setup（prepare_evolve[_joint]）同步完成（失败立即返错）；重活在后台线程跑、每代回调更新进度/收敛史。
/// 有 `targets=` → **多槽位联合进化**（S5）；否则单靶。异步路径放开 memetic + 大 pop/gens。
fn run_evolve_start(ctx: &Ctx, m: &ModelEntry, query: &str) -> Result<String, String> {
    if query_get(query, "targets").is_some() {
        // —— 多槽位联合进化（S5）——
        let job = prepare_evolve_joint(m, query)?;
        let id = new_gp_task(ctx, job.jcfg.gens.max(1));
        spawn_gp_task(Arc::clone(&ctx.tasks), id.clone(), move |p| run_evolve_joint_job(&job, p));
        Ok(format!("{{\"task_id\":\"{id}\"}}"))
    } else {
        // —— 单靶进化 ——
        let job = prepare_evolve(m, query)?;
        let id = new_gp_task(ctx, job.pcfg.gens.max(1));
        spawn_gp_task(Arc::clone(&ctx.tasks), id.clone(), move |p| run_evolve_job(&job, p));
        Ok(format!("{{\"task_id\":\"{id}\"}}"))
    }
}

/// `/api/evolve/status?id=<task_id>`：返回任务状态 + 进度（当前代/总代）+ 实时收敛曲线 SVG；
/// 完成时内嵌完整结果 JSON（`result`，与 `/api/evolve` 同结构）。未知 id → error。
fn run_evolve_status(ctx: &Ctx, query: &str) -> String {
    let id = match query_get(query, "id") {
        Some(i) => i,
        None => return error_json("缺少 id 参数（/api/evolve/status?id=<task_id>）"),
    };
    let t = match ctx.tasks.lock() {
        Ok(t) => t,
        Err(_) => return error_json("任务表锁错误"),
    };
    let task = match t.get(&id) {
        Some(task) => task,
        None => return error_json("无此任务（id 错或服务已重启）"),
    };
    let conv = crate::chart::convergence_chart_svg(&task.history, 720.0, 240.0);
    let mut j = serde_json::json!({
        "status": task.status,
        "gen": task.gen,
        "total_gens": task.total_gens,
        "convergence_svg": conv,
    });
    let obj = j.as_object_mut().unwrap();
    if let Some(r) = &task.result {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(r) {
            obj.insert("result".to_string(), v);
        }
    }
    if let Some(e) = &task.error {
        obj.insert("error".to_string(), serde_json::Value::String(e.clone()));
    }
    j.to_string()
}

/// `GET /api/source`：返回当前模型 `.eq.yaml` 原文（浏览器内编辑器 C1）。
/// v1 仅单文件模型可编辑；耦合视图 / 目录模型 → `editable:false` + 提示。
fn read_source(m: &ModelEntry) -> String {
    if m.coupling.is_some() {
        return serde_json::json!({ "editable": false, "error": "耦合视图不可编辑（请选单个作物模型）" }).to_string();
    }
    if !m.path.is_file() {
        return serde_json::json!({
            "editable": false, "path": m.path.display().to_string(),
            "error": "多文件/目录模型暂不支持编辑（v1 仅单文件）"
        })
        .to_string();
    }
    match std::fs::read_to_string(&m.path) {
        Ok(src) => serde_json::json!({ "editable": true, "source": src, "path": m.path.display().to_string() }).to_string(),
        Err(e) => serde_json::json!({ "editable": false, "error": format!("读取失败: {e}") }).to_string(),
    }
}

/// `POST /api/source`（body = 编辑后的 YAML 文本）：**受控写回**模型文件（编辑器 C1 增强）。
/// 先校验（非法不写）→ 备份现文件到 `<file>.bak` → 原子写（临时文件 + rename）。
/// 仅单文件模型；耦合视图/目录模型拒绝。返回 `{ok, path, backup}` 或 `{error}`。
fn write_source(m: &ModelEntry, body: &[u8]) -> Result<String, String> {
    if m.coupling.is_some() {
        return Err("耦合视图不可编辑（请选单个作物模型）".into());
    }
    if !m.path.is_file() {
        return Err("多文件/目录模型暂不支持保存（v1 仅单文件）".into());
    }
    let text = String::from_utf8_lossy(body);
    // 先校验：非法不写盘（守护"不弄坏正在用的模型"）。
    let file = parse_str(&text).map_err(|e| format!("校验未通过，未保存：{e}"))?;
    crate::validator::validate(&[file]).map_err(|e| format!("校验未通过，未保存：{e}"))?;
    // 备份现文件 → <file>.bak（即时安全网；git 仍是真正的版本史）。
    let bak = {
        let mut s = m.path.as_os_str().to_os_string();
        s.push(".bak");
        PathBuf::from(s)
    };
    std::fs::copy(&m.path, &bak).map_err(|e| format!("备份失败: {e}"))?;
    // 原子写：临时文件 + rename。
    let tmp = {
        let mut s = m.path.as_os_str().to_os_string();
        s.push(".tmp");
        PathBuf::from(s)
    };
    std::fs::write(&tmp, text.as_bytes()).map_err(|e| format!("写临时文件失败: {e}"))?;
    std::fs::rename(&tmp, &m.path).map_err(|e| format!("替换文件失败: {e}"))?;
    Ok(serde_json::json!({
        "ok": true, "path": m.path.display().to_string(), "backup": bak.display().to_string()
    })
    .to_string())
}

/// `POST /api/validate`（body = 编辑后的 YAML 文本）：parse + 校验 → `{ok, errors[], report_html?}`。
/// ok 时附结构图预览 HTML（复用 `generate_report`，所见即所得）。**不写盘**——编辑的是浏览器副本。
/// `?layout=&level=` 让预览与结构工作区一致。
fn run_validate(query: &str, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let file = match parse_str(&text) {
        Ok(f) => f,
        Err(e) => return serde_json::json!({ "ok": false, "errors": [e.to_string()] }).to_string(),
    };
    let files = vec![file];
    if let Err(e) = crate::validator::validate(&files) {
        return serde_json::json!({ "ok": false, "errors": [e.to_string()] }).to_string();
    }
    let report = render_report(&files, parse_layout(query), parse_dag_level(query), parse_color(query)).unwrap_or_default();
    serde_json::json!({ "ok": true, "errors": [], "report_html": report }).to_string()
}

/// 把候选的可调常数 `__c{i}` 代回常数值，得到具体表达式（供 `expr_mathml` 渲染 2D 公式）。
fn candidate_expr(cand: &crate::gp::Candidate) -> crate::ast::Expr {
    let mut e = cand.expr.clone();
    for (i, v) in cand.consts.iter().enumerate() {
        e = e.substitute(&crate::gp::Candidate::const_name(i), &crate::ast::Expr::constant(*v));
    }
    e
}

/// GP「看它长出什么」彩蛋（GA-6b Phase 3）：候选 patch 进目标方程（常数**代回字面值**，
/// 不引入 `__c` 参数节点）→ 与原模型做结构 diff。受约束 GP 只用靶点 inputs（已有变量）→
/// **不长新节点**：diff 的核心是 `added_edges`（GP 新依赖的输入，= 长出的"新枝"）+
/// `changed_equations`（目标方程形式变了，前端给它打脉冲）。本地名对齐（before/after 同 meta.id）。
/// 随 evolve 结果每候选带上，前端零延迟内联播 3D 生长动画。失败 → Null。
fn gp_structure_diff(file: &EquationFile, target: &str, cand: &crate::gp::Candidate) -> serde_json::Value {
    let mut after = file.clone();
    match after.equations.iter_mut().find(|e| e.id == target) {
        Some(eq) => eq.expression = candidate_expr(cand),
        None => return serde_json::Value::Null,
    }
    let diff = crate::graph::diff_models(std::slice::from_ref(file), std::slice::from_ref(&after));
    serde_json::to_value(crate::export::to_graph_diff_json(&diff)).unwrap_or(serde_json::Value::Null)
}

/// 把一个 GP 候选 patch 进目标方程 → 仿真 → 抽 `output` 轨迹（{DAT, value}）。失败 → Null。
fn candidate_trajectory(
    file: &EquationFile,
    target: &str,
    output: &str,
    cand: &crate::gp::Candidate,
    input: &SimInput,
) -> serde_json::Value {
    match crate::gp::patch_model(file, target, cand) {
        Some(m) => match simulate(&m, input) {
            Ok(out) => series_to_traj(&out, output),
            Err(_) => serde_json::Value::Null,
        },
        None => serde_json::Value::Null,
    }
}

/// rmse(传入序列 vs 稀疏观测)，仅在观测日（1-based DAT）上比较。与 GP 候选适应度同口径，
/// 用于 baseline 与候选的并排对比。无可比点/非有限 → None。
fn rmse_on_obs(traj: Option<&[f64]>, obs: &[(usize, f64)]) -> Option<f64> {
    let traj = traj?;
    if obs.is_empty() {
        return None;
    }
    let (mut se, mut n) = (0.0f64, 0usize);
    for &(day, val) in obs {
        let y = *traj.get(day.checked_sub(1)?)?;
        if !y.is_finite() {
            return None;
        }
        se += (y - val).powi(2);
        n += 1;
    }
    (n > 0).then(|| (se / n as f64).sqrt())
}

/// 采纳后可粘贴进模型的 `.eq.yaml` 方程片段：候选 expr（常数已代回字面值）包成
/// `{id,name,output,expression}` 列表项。供科学家复制粘贴替换原方程（S3，不写盘）。
fn candidate_yaml_fragment(target: &str, name: &str, output: &str, cand: &crate::gp::Candidate) -> String {
    let mut m = serde_yaml::Mapping::new();
    m.insert("id".into(), target.into());
    m.insert("name".into(), name.into());
    m.insert("output".into(), output.into());
    m.insert(
        "expression".into(),
        crate::sexpr::to_yaml::to_yaml_value(&candidate_expr(cand)),
    );
    let seq = serde_yaml::Value::Sequence(vec![serde_yaml::Value::Mapping(m)]);
    let body = serde_yaml::to_string(&seq).unwrap_or_default();
    format!("# —— GP 采纳候选（复制进模型替换原方程；可调常数已代回字面值）——\n{body}")
}

/// 仿真输出某变量序列 → {DAT:[1..n], value:[...]}。变量缺失 → Null。
fn series_to_traj(out: &SimOutput, name: &str) -> serde_json::Value {
    match out.series(name) {
        Some(s) => {
            let dat: Vec<usize> = (1..=s.len()).collect();
            serde_json::json!({ "DAT": dat, "value": s })
        }
        None => serde_json::Value::Null,
    }
}

/// 稀疏观测 [(1-based DAT, 值)] → {DAT:[...], value:[...]}（前端叠散点用）。
fn obs_to_traj(obs: &[(usize, f64)]) -> serde_json::Value {
    let dat: Vec<usize> = obs.iter().map(|(d, _)| *d).collect();
    let val: Vec<f64> = obs.iter().map(|(_, v)| *v).collect();
    serde_json::json!({ "DAT": dat, "value": val })
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

/// 通用查询取值：`?key=val` → val（url 解码；空串当缺省）。
fn query_get(query: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix(&prefix) {
            let s = url_decode(v);
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

/// `?key=N` → usize（解析失败/缺省 → None）。
fn query_usize(query: &str, key: &str) -> Option<usize> {
    query_get(query, key).and_then(|s| s.trim().parse().ok())
}

/// `?key=x.y` → f64（解析失败/缺省 → None）。
fn query_f64(query: &str, key: &str) -> Option<f64> {
    query_get(query, key).and_then(|s| s.trim().parse().ok())
}

/// `?key=1|true|on|yes` → true；缺省/其它 → false。
fn query_flag(query: &str, key: &str) -> bool {
    matches!(
        query_get(query, key).as_deref(),
        Some("1") | Some("true") | Some("on") | Some("yes")
    )
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
    fn test_query_helpers() {
        let q = "model=bb&target=BB5-DORM&pop=80&seed=7&parsimony=0.01&memetic=true";
        assert_eq!(query_get(q, "target").as_deref(), Some("BB5-DORM"));
        assert_eq!(query_usize(q, "pop"), Some(80));
        assert_eq!(query_usize(q, "seed"), Some(7));
        assert_eq!(query_f64(q, "parsimony"), Some(0.01));
        assert!(query_flag(q, "memetic"));
        // 缺省 / 空 / 非法
        assert_eq!(query_get(q, "gens"), None);
        assert_eq!(query_usize(q, "gens"), None);
        assert!(!query_flag(q, "pareto"));
        assert!(!query_flag("pareto=off", "pareto"));
        assert_eq!(query_get("target=", "target"), None); // 空串当缺省
    }

    #[test]
    fn test_evolve_traj_helpers() {
        use crate::sim::SimOutput;
        // series_to_traj：DAT 是 1-based。
        let mut traj = IndexMap::new();
        traj.insert("y".to_string(), vec![1.0, 2.0, 3.0]);
        let out = SimOutput { steps: 3, trajectories: traj };
        let j = series_to_traj(&out, "y");
        assert_eq!(j["DAT"], serde_json::json!([1, 2, 3]));
        assert_eq!(j["value"], serde_json::json!([1.0, 2.0, 3.0]));
        assert!(series_to_traj(&out, "missing").is_null());
        // obs_to_traj：稀疏观测 → 两个并列数组。
        let o = obs_to_traj(&[(5, 0.5), (10, 0.9)]);
        assert_eq!(o["DAT"], serde_json::json!([5, 10]));
        assert_eq!(o["value"], serde_json::json!([0.5, 0.9]));
    }

    #[test]
    fn test_evolve_s3_helpers() {
        use crate::ast::Expr;
        use crate::gp::Candidate;
        // rmse_on_obs：仅在观测日（1-based）上比较。
        let traj = [1.0, 2.0, 3.0, 4.0];
        let r = rmse_on_obs(Some(&traj), &[(1, 1.0), (3, 3.5)]).unwrap();
        assert!((r - (0.125f64).sqrt()).abs() < 1e-12); // se=(0+0.25)/2
        assert!(rmse_on_obs(Some(&traj), &[]).is_none()); // 无观测
        assert!(rmse_on_obs(None, &[(1, 1.0)]).is_none()); // 无轨迹
        // candidate_yaml_fragment：常数代回字面值，产出可解析的 .eq.yaml 片段。
        let cand = Candidate { expr: Expr::add(Expr::param("__c0"), Expr::var("d")), consts: vec![2.5] };
        let frag = candidate_yaml_fragment("TGT", "门控", "y", &cand);
        assert!(frag.contains("GP 采纳候选"));
        let v: serde_yaml::Value = serde_yaml::from_str(&frag).expect("片段应是合法 YAML");
        let eq0 = &v.as_sequence().unwrap()[0];
        assert_eq!(eq0["id"].as_str(), Some("TGT"));
        assert_eq!(eq0["output"].as_str(), Some("y"));
        // 常数已代回字面（出现 const: 2.5，不出现 __c 占位）
        assert!(frag.contains("2.5"));
        assert!(!frag.contains("__c"));
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
        // 受约束 GP 面板（S2）：靶点列表 + 进化端点 + 候选详情/拟合叠图
        assert!(STUDIO_HTML.contains("gpPanel"));
        assert!(STUDIO_HTML.contains("buildGpTargets"));
        assert!(STUDIO_HTML.contains("runEvolve"));
        assert!(STUDIO_HTML.contains("showCandidate"));
        assert!(STUDIO_HTML.contains("gpFitSvg"));
        // S3 对比 + 采纳：采纳区 + 溯源草稿/新方程文本复制下载
        assert!(STUDIO_HTML.contains("renderAdopt"));
        assert!(STUDIO_HTML.contains("gp-adopt-btn"));
        assert!(STUDIO_HTML.contains("gpDownload"));
        // S4 异步：start/status 端点 + 轮询 + memetic 勾选 + 实时收敛曲线
        assert!(STUDIO_HTML.contains("/api/evolve/start?"));
        assert!(STUDIO_HTML.contains("/api/evolve/status?id="));
        assert!(STUDIO_HTML.contains("pollEvolve"));
        assert!(STUDIO_HTML.contains("gpMemetic"));
        assert!(STUDIO_HTML.contains("gpConv"));
        // S5 多槽位：靶点多选 + 联合 targets= + 候选块逐槽渲染
        assert!(STUDIO_HTML.contains("toggleGpTarget"));
        assert!(STUDIO_HTML.contains("gpSelectAll"));
        assert!(STUDIO_HTML.contains("targets="));
        assert!(STUDIO_HTML.contains("candidateBlockHtml"));
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
        // 耦合面板（可仿真耦合）：仿真 + 优化
        assert!(STUDIO_HTML.contains("couplePanel"));
        assert!(STUDIO_HTML.contains("/api/couple"));
        assert!(STUDIO_HTML.contains("runCoupleSim"));
        assert!(STUDIO_HTML.contains("/api/couple-optimize"));
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
