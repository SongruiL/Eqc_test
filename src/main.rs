//! Equation Compiler CLI
//!
//! 命令行工具，用于编译方程文件。
//!
//! ## 使用方法
//!
//! ```bash
//! # 编译所有方程
//! eqc build --input ./equations --output ./generated
//!
//! # 仅验证
//! eqc validate ./equations
//!
//! # 输出 DAG
//! eqc graph ./equations
//! ```

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};

#[cfg(feature = "cli")]
use equation_compiler::{Compiler, GeneratorKind};

#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "eqc")]
#[command(author = "Boshenaware")]
#[command(version = "0.1.0")]
#[command(about = "方程编译器 - 将 YAML 方程定义编译为多种格式")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// 编译方程文件
    Build {
        /// 输入目录（包含 .eq.yaml 文件）
        #[arg(short, long)]
        input: PathBuf,

        /// 输出目录
        #[arg(short, long)]
        output: PathBuf,

        /// 输出格式：python, rust, json, markdown, latex, all
        #[arg(short, long, default_value = "all")]
        format: String,
    },

    /// 验证方程文件
    Validate {
        /// 输入目录
        input: PathBuf,
    },

    /// 输出依赖图
    Graph {
        /// 输入目录
        input: PathBuf,

        /// 输出格式：mermaid, dot
        #[arg(short, long, default_value = "mermaid")]
        format: String,
    },

    /// 列出所有方程
    List {
        /// 输入目录
        input: PathBuf,
    },

    /// 转换S表达式为YAML
    Convert {
        /// 输入S表达式（文件路径或直接表达式字符串）
        input: String,

        /// 输出YAML文件（可选，不指定则输出到stdout）
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 输出格式：yaml, json
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },

    /// 从带注解的S表达式生成workflow和算子
    Workflow {
        /// 输入S表达式文件或目录
        input: PathBuf,

        /// 输出目录（Rust算子代码）
        #[arg(short, long)]
        output: PathBuf,

        /// 同时生成Rust算子代码
        #[arg(long)]
        operators: bool,

        /// SQL模板输出目录（默认与output相同）
        #[arg(long)]
        sql_output: Option<PathBuf>,
    },

    /// 验证带注解的S表达式文件
    ValidateSexpr {
        /// 输入S表达式文件或目录
        input: PathBuf,

        /// 输出详细信息
        #[arg(short, long)]
        verbose: bool,

        /// 将错误视为警告（不返回错误码）
        #[arg(long)]
        warn_only: bool,
    },

    /// 生成多模块 L2 级 Mermaid DAG（通过 Connector 耦合）
    GraphL2 {
        /// 输入 S-expression 文件列表
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
    },

    /// 输出S表达式书写规范
    SexprSpec,

    /// 检查量纲一致性与跨模块耦合单位
    CheckDims {
        /// 输入目录（包含 .eq.yaml 文件）
        input: PathBuf,

        /// 有错误时返回非零退出码
        #[arg(long)]
        strict: bool,
    },

    /// 生成自包含 HTML 模型报告（DAG 图 + 二维公式，离线可看）
    Report {
        /// 输入目录（包含 .eq.yaml 文件）
        input: PathBuf,

        /// 输出 HTML 文件
        #[arg(short, long, default_value = "report.html")]
        output: PathBuf,

        /// 结构图布局：layered（分层，默认）, force（力导向）, forrester（学术风，暂回退分层）
        #[arg(short, long, default_value = "layered")]
        layout: String,
    },

    /// 逐日仿真一个动态模型：按驱动量时间序列做显式 Euler 时间步进，输出轨迹 CSV
    Simulate {
        /// 模型文件（单个 .eq.yaml）
        input: PathBuf,

        /// 驱动量 CSV（首行为变量名，每行一天；列名须匹配模型里的驱动量）
        #[arg(short, long)]
        drivers: PathBuf,

        /// 参数覆盖 JSON（如各 cohort 开花日 {"anthesis__1": 55, ...}），可选
        #[arg(short, long)]
        params: Option<PathBuf>,

        /// 步数（默认取驱动量 CSV 的行数）
        #[arg(short, long)]
        steps: Option<usize>,

        /// 时间步长 dt（覆盖模型 meta.dt；缺省用 meta.dt，日步长模型=1）。亚日动态模型（温室气候）设小步长
        #[arg(long)]
        dt: Option<f64>,

        /// 状态初值覆盖 `name=val,name=val,...`（覆盖模型里状态/延迟寄存器的 init:；多年生跨年编排用：
        /// 携带木质池/储备作次年 init、物候清零）。例：`--init W_cane=420,C_reserve=66,ChillAccum=0`
        #[arg(long)]
        init: Option<String>,

        /// 输出轨迹 CSV
        #[arg(short, long, default_value = "sim_output.csv")]
        output: PathBuf,
    },

    /// 耦合仿真（C1：多速率、单向）：快模型（温室，小 dt）↔ 慢模型（作物，大 dt）一次集成运行。
    /// 每慢步跑 R=dt_slow秒/dt_fast秒 个快步、把温室气候聚合喂作物。见 docs/spec-coupled-simulation.md。
    Couple {
        /// 快模型（小 dt，如温室；须有 meta.dt_seconds）
        #[arg(long)]
        fast: PathBuf,

        /// 慢模型（大 dt，如作物；须有 meta.dt_seconds）
        #[arg(long)]
        slow: PathBuf,

        /// 快模型室外驱动 CSV（快分辨率，全程；行数 ≥ 慢步数·R）
        #[arg(short, long)]
        weather: PathBuf,

        /// 快→慢链接，可重复：`to=from[:agg[:scale]]`（agg=mean|integral|last，缺省 mean；scale 缺省 1）。
        /// 例：`--link T=T_air:mean --link Sr=Q_sun:integral:1e-6`
        #[arg(long = "link")]
        links: Vec<String>,

        /// 慢→快反馈（C2 双向，滞后一慢步），可重复：`to=from[:scale[:init]]`（scale 缺省 1、init 缺省 0）。
        /// 例：`--feedback phi_ass=assim_flux_inst:1.0:0`（温室 phi_ass ← 作物瞬时光合通量）
        #[arg(long = "feedback")]
        feedback: Vec<String>,

        /// 快模型（温室）参数覆盖 JSON（如环控设定点；C3 优化的旋钮即在此）
        #[arg(long)]
        fast_params: Option<PathBuf>,

        /// 慢模型（作物）参数覆盖 JSON
        #[arg(long)]
        slow_params: Option<PathBuf>,

        /// 慢步数（作物天数；缺省 = 室外驱动行数 / R）
        #[arg(short, long)]
        steps: Option<usize>,

        /// 输出慢模型（作物）轨迹 CSV
        #[arg(short, long, default_value = "couple_output.csv")]
        output: PathBuf,

        /// 另存喂给慢模型的聚合驱动 CSV（= 等效的离线 aggregate；便于核对）
        #[arg(long)]
        fed_out: Option<PathBuf>,

        /// 另存快模型（温室）日均轨迹 CSV（看反馈对温室气候如 CO₂ 的影响）
        #[arg(long)]
        fast_out: Option<PathBuf>,
    },

    /// 参数敏感性扫描：把一个标量参数在区间内取 N 点各跑一次仿真，输出对某变量的响应 CSV
    Sweep {
        /// 模型文件（单个 .eq.yaml）
        input: PathBuf,

        /// 驱动量 CSV
        #[arg(short, long)]
        drivers: PathBuf,

        /// 【单参数模式】要扫描的标量参数名（与 --sensitivity 二选一）
        #[arg(long)]
        param: Option<String>,

        /// 【单参数模式】扫描区间 a:b:n —— 从 a 到 b 取 n 个点（如 1.0:5.0:9）
        #[arg(long)]
        range: Option<String>,

        /// 【敏感性模式】对所有标量参数各 ±percent% 各跑一遍，按对 --var 的影响排序
        #[arg(long)]
        sensitivity: bool,

        /// 敏感性模式的扰动幅度（百分比，默认 10）
        #[arg(long, default_value_t = 10.0)]
        percent: f64,

        /// 关注的输出变量名（轨迹键；向量变量用 “名[1]” 形式）
        #[arg(long)]
        var: String,

        /// 对该输出的归约：final（末值，默认）/ max / mean / min
        #[arg(long, default_value = "final")]
        reduce: String,

        /// 基准参数覆盖 JSON（扫描参数以外的其它覆盖），可选
        #[arg(long)]
        params: Option<PathBuf>,

        /// 步数（默认取驱动量 CSV 行数）
        #[arg(long)]
        steps: Option<usize>,

        /// 输出扫描结果 CSV
        #[arg(short, long, default_value = "sweep.csv")]
        output: PathBuf,
    },

    /// 本地预览服务（EQC Studio）：监听模型文件，存盘即刷新；可跑仿真画轨迹
    Serve {
        /// 模型文件（.eq.yaml）或目录
        input: PathBuf,

        /// 监听端口
        #[arg(short, long, default_value_t = 7878)]
        port: u16,

        /// 驱动量 CSV（提供后 Studio 可跑仿真、画整季轨迹）
        #[arg(short, long)]
        drivers: Option<PathBuf>,

        /// 参数覆盖 JSON
        #[arg(long)]
        params: Option<PathBuf>,

        /// 实测数据存放目录（园区录入的 observed CSV 写到这里，每处理区一文件 <zone>.csv；
        /// 缺省=模型同级的 observations/）。正是 `eqc calibrate --observed` 的输入。
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// 导出模型的 JSON 契约（前端/工具消费用，可检视）
    Export {
        /// 模型文件（.eq.yaml）或目录
        input: PathBuf,

        /// 输出 JSON 文件（缺省打印到 stdout）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 仿真优化：读模型 + 决策 spec，用差分进化 DE 搜旋钮空间，输出最优旋钮 + 目标值
    Optimize {
        /// 模型文件（单个 .eq.yaml）
        input: PathBuf,

        /// 决策 spec（YAML：目标/旋钮/约束/优化器，见 docs/spec-optimization.md §4）
        #[arg(short, long)]
        spec: PathBuf,

        /// 环境驱动量 CSV（覆盖 spec 里的 environment:；二者至少有一个）
        #[arg(short, long)]
        drivers: Option<PathBuf>,

        /// 步数（默认取驱动量 CSV 行数）
        #[arg(long)]
        steps: Option<usize>,

        /// 优化前做敏感性预筛：把对目标几乎无影响的旋钮固定在基线、只搜索敏感旋钮（单目标）
        #[arg(long)]
        prescreen: bool,

        /// 输出结果 JSON（最优旋钮 + 目标值 + 收敛轨迹），缺省只打印到控制台
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 参数标定：用实测数据反推模型参数（旋钮=参数、目标=预测 vs 实测误差），见 docs/spec-calibration.md
    Calibrate {
        /// 模型文件（单个 .eq.yaml）
        input: PathBuf,

        /// 标定 spec（YAML：误差目标/参数旋钮/observed/environment）
        #[arg(short, long)]
        spec: PathBuf,

        /// 同期天气驱动量 CSV（覆盖 spec 的 environment:）
        #[arg(short, long)]
        drivers: Option<PathBuf>,

        /// 实测数据 CSV（覆盖 spec 的 observed:；首列 DAT + 各观测变量列，空格=未测）
        #[arg(long)]
        observed: Option<PathBuf>,

        /// 步数（默认取驱动量 CSV 行数）
        #[arg(long)]
        steps: Option<usize>,

        /// 输出结果 JSON（拟合参数 + 误差 + 收敛轨迹），缺省只打印到控制台
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 可辨识性分析（服务实验设计）：标定前看「要定准哪个参数、最该测哪个变量」，见 docs/spec-calibration.md §5
    Identify {
        /// 模型文件（单个 .eq.yaml）
        input: PathBuf,

        /// 标定 spec（候选参数 = 其 knobs；可含 observables: 候选可观测变量）
        #[arg(short, long)]
        spec: PathBuf,

        /// 同期天气驱动量 CSV（覆盖 spec 的 environment:）
        #[arg(short, long)]
        drivers: Option<PathBuf>,

        /// 候选可观测变量（逗号分隔，覆盖 spec 的 observables:；缺省=模型所有 output 标量）
        #[arg(long)]
        observables: Option<String>,

        /// 步数（默认取驱动量 CSV 行数）
        #[arg(long)]
        steps: Option<usize>,

        /// 输出报告 JSON，缺省只打印到控制台
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[cfg(feature = "cli")]
fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Build {
            input,
            output,
            format,
        } => run_build(&input, &output, &format),
        Commands::Validate { input } => run_validate(&input),
        Commands::Graph { input, format } => run_graph(&input, &format),
        Commands::List { input } => run_list(&input),
        Commands::Convert { input, output, format } => run_convert(&input, output.as_ref(), &format),
        Commands::Workflow { input, output, operators, sql_output } => run_workflow(&input, &output, operators, sql_output.as_ref()),
        Commands::ValidateSexpr { input, verbose, warn_only } => run_validate_sexpr(&input, verbose, warn_only),
        Commands::GraphL2 { inputs } => run_graph_l2(&inputs),
        Commands::SexprSpec => run_sexpr_spec(),
        Commands::CheckDims { input, strict } => run_check_dims(&input, strict),
        Commands::Report { input, output, layout } => run_report(&input, &output, &layout),
        Commands::Couple { fast, slow, weather, links, feedback, fast_params, slow_params, steps, output, fed_out, fast_out } => {
            run_couple(&fast, &slow, &weather, &links, &feedback, fast_params.as_ref(), slow_params.as_ref(), steps, &output, fed_out.as_ref(), fast_out.as_ref())
        }
        Commands::Simulate { input, drivers, params, steps, output, dt, init } => {
            run_simulate(&input, &drivers, params.as_ref(), steps, &output, dt, init.as_deref())
        }
        Commands::Sweep { input, drivers, param, range, sensitivity, percent, var, reduce, params, steps, output } => {
            run_sweep(&input, &drivers, param.as_deref(), range.as_deref(), sensitivity, percent, &var, &reduce, params.as_ref(), steps, &output)
        }
        Commands::Serve { input, port, drivers, params, data_dir } => {
            equation_compiler::serve::serve(&input, port, drivers.as_ref(), params.as_ref(), data_dir.as_ref())
        }
        Commands::Export { input, output } => run_export(&input, output.as_ref()),
        Commands::Optimize { input, spec, drivers, steps, prescreen, output } => {
            run_optimize(&input, &spec, drivers.as_ref(), steps, prescreen, output.as_ref())
        }
        Commands::Calibrate { input, spec, drivers, observed, steps, output } => {
            run_calibrate(&input, &spec, drivers.as_ref(), observed.as_ref(), steps, output.as_ref())
        }
        Commands::Identify { input, spec, drivers, observables, steps, output } => {
            run_identify(&input, &spec, drivers.as_ref(), observables.as_deref(), steps, output.as_ref())
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        // 如果是多个验证错误，打印详细信息
        if let Some(equation_compiler::error::CompileError::MultipleValidationErrors(errors)) =
            e.downcast_ref::<equation_compiler::error::CompileError>()
        {
            for err in errors {
                eprintln!("  - {}", err);
            }
        }
        std::process::exit(1);
    }
}

#[cfg(feature = "cli")]
fn run_build(
    input: &PathBuf,
    output: &PathBuf,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📂 加载方程文件: {}", input.display());

    let kind = match format {
        "python" => GeneratorKind::Python,
        "rust" => GeneratorKind::RustOperator,
        "json" => GeneratorKind::WorkflowJson,
        "markdown" => GeneratorKind::Markdown,
        "latex" => GeneratorKind::Latex,
        "all" => GeneratorKind::All,
        _ => {
            return Err(format!("未知格式: {}", format).into());
        }
    };

    Compiler::new()
        .load_directory(input)?
        .validate()?
        .build_dag()?
        .generate(kind, output)?;

    println!("✅ 生成完成: {}", output.display());
    Ok(())
}

#[cfg(feature = "cli")]
fn run_validate(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 验证方程文件: {}", input.display());

    // 单文件用 load_file，目录用 load_directory（否则单文件会撞 read_dir 的「目录名无效」os-267）。
    let loaded = if input.is_file() {
        Compiler::new().load_file(input)?
    } else {
        Compiler::new().load_directory(input)?
    };
    let compiler = loaded.validate()?;

    println!("✅ 验证通过");
    println!("   - 模块数: {}", compiler.files().len());
    println!("   - 方程数: {}", compiler.equation_ids().len());

    Ok(())
}

#[cfg(feature = "cli")]
fn run_report(
    input: &PathBuf,
    output: &PathBuf,
    layout: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let loaded = if input.is_file() {
        Compiler::new().load_file(input)?
    } else {
        Compiler::new().load_directory(input)?
    };
    let compiler = loaded.validate()?.build_dag()?;
    let dag = compiler.dag().ok_or("DAG 未构建")?;
    let kind = equation_compiler::report::LayoutKind::parse(layout);
    let html = equation_compiler::report::generate_report_with(compiler.files(), dag, kind);
    std::fs::write(output, html)?;
    println!("✅ 报告已生成: {}（布局：{}）", output.display(), kind.as_str());
    println!("   用浏览器（Edge/Chrome/Firefox）打开即可查看 DAG 与二维公式。");
    Ok(())
}

#[cfg(feature = "cli")]
fn run_simulate(
    input: &PathBuf,
    drivers: &PathBuf,
    params: Option<&PathBuf>,
    steps: Option<usize>,
    output: &PathBuf,
    dt: Option<f64>,
    init: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::scenario::{load_drivers_csv, load_params_json};
    use equation_compiler::{parse_file, simulate, SimInput};

    println!("🌱 仿真模型: {}", input.display());
    let file = parse_file(input)?;

    // 读驱动量 CSV
    let (rows, driver_map) = load_drivers_csv(drivers)?;
    let steps = steps.unwrap_or(rows);

    let mut sim_in = SimInput::new(steps);
    sim_in.drivers = driver_map;
    sim_in.dt = dt; // None → 用模型 meta.dt

    // 读参数覆盖 JSON（可选）
    if let Some(pjson) = params {
        sim_in.param_overrides = load_params_json(pjson)?;
    }

    // 状态初值覆盖 `name=val,...`（覆盖状态/延迟寄存器 init；多年生跨年编排用）
    if let Some(s) = init {
        sim_in.init_overrides = parse_init_overrides(s)?;
    }

    let out = simulate(&file, &sim_in).map_err(|e| format!("仿真失败: {e}"))?;

    // 写轨迹 CSV（首列 DAT）
    let mut csv = String::from("DAT");
    for name in out.trajectories.keys() {
        csv.push(',');
        csv.push_str(name);
    }
    csv.push('\n');
    for n in 0..out.steps {
        csv.push_str(&(n + 1).to_string());
        for series in out.trajectories.values() {
            csv.push(',');
            csv.push_str(&format!("{}", series[n]));
        }
        csv.push('\n');
    }
    std::fs::write(output, csv)?;

    println!("✅ 仿真完成：{} 步，轨迹已写入 {}", out.steps, output.display());
    // 打印输出变量末值
    let outputs = file.output_variables();
    if !outputs.is_empty() {
        println!("   输出变量末值（第 {} 天）：", out.steps);
        for (name, _) in outputs {
            if let Some(v) = out.final_value(name) {
                println!("     {name} = {v}");
            }
        }
    }
    Ok(())
}

/// 把轨迹/聚合驱动写成 CSV（首列 DAT，列序保 IndexMap 声明序）。
#[cfg(feature = "cli")]
fn write_traj_csv(
    traj: &indexmap::IndexMap<String, Vec<f64>>,
    steps: usize,
    path: &PathBuf,
) -> std::io::Result<()> {
    let mut csv = String::from("DAT");
    for name in traj.keys() {
        csv.push(',');
        csv.push_str(name);
    }
    csv.push('\n');
    for n in 0..steps {
        csv.push_str(&(n + 1).to_string());
        for series in traj.values() {
            csv.push(',');
            csv.push_str(&format!("{}", series[n]));
        }
        csv.push('\n');
    }
    std::fs::write(path, csv)
}

/// `eqc couple`：多速率耦合仿真（C1 单向）。
#[cfg(feature = "cli")]
#[allow(clippy::too_many_arguments)]
fn run_couple(
    fast: &PathBuf,
    slow: &PathBuf,
    weather: &PathBuf,
    links: &[String],
    feedback: &[String],
    fast_params: Option<&PathBuf>,
    slow_params: Option<&PathBuf>,
    steps: Option<usize>,
    output: &PathBuf,
    fed_out: Option<&PathBuf>,
    fast_out: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::scenario::{load_drivers_csv, load_params_json};
    use equation_compiler::{
        parse_file, simulate_coupled, Agg, CoupledInput, CoupledLink, FeedbackLink,
    };

    let fast_file = parse_file(fast)?;
    let slow_file = parse_file(slow)?;
    println!("🔗 耦合仿真: 快 {} ↔ 慢 {}", fast_file.meta.id, slow_file.meta.id);

    // 解析链接 to=from[:agg[:scale]]
    let mut parsed: Vec<CoupledLink> = Vec::new();
    for l in links {
        let (to, rest) = l
            .split_once('=')
            .ok_or_else(|| format!("链接格式应为 to=from[:agg[:scale]]，得到: {l}"))?;
        let parts: Vec<&str> = rest.split(':').collect();
        let from = parts[0].trim();
        let agg = match parts.get(1) {
            Some(s) => Agg::parse(s).ok_or_else(|| format!("未知聚合 '{s}'（mean|integral|last）"))?,
            None => Agg::Mean,
        };
        let scale = match parts.get(2) {
            Some(s) => s.parse::<f64>().map_err(|e| format!("scale 非数值: {e}"))?,
            None => 1.0,
        };
        parsed.push(CoupledLink { to: to.trim().to_string(), from: from.to_string(), agg, scale });
    }
    if parsed.is_empty() {
        return Err("至少需要一条 --link（如 --link T=T_air:mean）".into());
    }

    // 解析反馈 to=from[:scale[:init]]（慢→快，C2 双向）
    let mut fb: Vec<FeedbackLink> = Vec::new();
    for l in feedback {
        let (to, rest) = l
            .split_once('=')
            .ok_or_else(|| format!("反馈格式应为 to=from[:scale[:init]]，得到: {l}"))?;
        let parts: Vec<&str> = rest.split(':').collect();
        let from = parts[0].trim();
        let scale = match parts.get(1) {
            Some(s) => s.parse::<f64>().map_err(|e| format!("scale 非数值: {e}"))?,
            None => 1.0,
        };
        let init = match parts.get(2) {
            Some(s) => s.parse::<f64>().map_err(|e| format!("init 非数值: {e}"))?,
            None => 0.0,
        };
        fb.push(FeedbackLink { to: to.trim().to_string(), from: from.to_string(), scale, init });
    }

    // R = dt_slow秒 / dt_fast秒（定每慢步的快步数、默认慢步数）
    let dtf = fast_file
        .meta
        .dt_seconds
        .ok_or_else(|| format!("快模型 {} 缺 meta.dt_seconds", fast_file.meta.id))?;
    let dts = slow_file
        .meta
        .dt_seconds
        .ok_or_else(|| format!("慢模型 {} 缺 meta.dt_seconds", slow_file.meta.id))?;
    let r = (dts / dtf).round().max(1.0) as usize;

    let (rows, weather_map) = load_drivers_csv(weather)?;
    let slow_steps = steps.unwrap_or(rows / r);
    if slow_steps == 0 {
        return Err(format!("室外驱动 {rows} 行不足一慢步（R={r}）").into());
    }
    let need = slow_steps * r;
    if rows < need {
        return Err(format!("室外驱动 {rows} 行 < 慢步数·R = {need}").into());
    }
    // 截到精确长度（多余的整步尾巴丢弃）
    let weather_trunc: std::collections::HashMap<String, Vec<f64>> = weather_map
        .into_iter()
        .map(|(k, v)| (k, v[..need.min(v.len())].to_vec()))
        .collect();

    let mut inp = CoupledInput::new(&fast_file, &slow_file, parsed, weather_trunc, slow_steps);
    inp.feedback = fb;
    if let Some(fp) = fast_params {
        inp.fast_params = load_params_json(fp)?;
    }
    if let Some(sp) = slow_params {
        inp.slow_params = load_params_json(sp)?;
    }
    let out = simulate_coupled(&inp).map_err(|e| format!("耦合仿真失败: {e}"))?;

    write_traj_csv(&out.slow.trajectories, out.slow_steps, output)?;
    if let Some(fo) = fed_out {
        write_traj_csv(&out.fed_drivers, out.slow_steps, fo)?;
    }
    if let Some(fo) = fast_out {
        write_traj_csv(&out.fast.trajectories, out.slow_steps, fo)?;
    }

    println!(
        "✅ 耦合完成：{} 慢步 × R={} 快步/步（共 {} 快步）；作物轨迹 → {}",
        out.slow_steps,
        out.r,
        out.slow_steps * out.r,
        output.display()
    );
    let outputs = slow_file.output_variables();
    if !outputs.is_empty() {
        println!("   作物输出末值（第 {} 步）：", out.slow_steps);
        for (name, _) in outputs.iter().take(8) {
            if let Some(v) = out.slow.final_value(name) {
                println!("     {name} = {v}");
            }
        }
    }
    Ok(())
}

/// 解析状态初值覆盖 `name=val,name=val,...` → map。空串/空段忽略。
#[cfg(feature = "cli")]
fn parse_init_overrides(s: &str) -> Result<std::collections::HashMap<String, f64>, String> {
    let mut m = std::collections::HashMap::new();
    for pair in s.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (name, val) = pair
            .split_once('=')
            .ok_or_else(|| format!("--init 段 '{pair}' 须为 name=val 形式"))?;
        let name = name.trim();
        let v: f64 = val
            .trim()
            .parse()
            .map_err(|_| format!("--init '{name}' 的值 '{}' 不是数值", val.trim()))?;
        if name.is_empty() {
            return Err(format!("--init 段 '{pair}' 变量名为空"));
        }
        m.insert(name.to_string(), v);
    }
    Ok(m)
}

/// 解析扫描区间 `a:b:n` → (起, 止, 点数)。
#[cfg(feature = "cli")]
fn parse_range(s: &str) -> Result<(f64, f64, usize), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(format!("--range 须为 a:b:n（如 1.0:5.0:9），收到 '{s}'"));
    }
    let a: f64 = parts[0].trim().parse().map_err(|_| "range 起点不是数值".to_string())?;
    let b: f64 = parts[1].trim().parse().map_err(|_| "range 终点不是数值".to_string())?;
    let n: usize = parts[2].trim().parse().map_err(|_| "range 点数不是整数".to_string())?;
    if n == 0 {
        return Err("range 点数须 ≥ 1".to_string());
    }
    Ok((a, b, n))
}

/// 对一条轨迹做归约。
#[cfg(feature = "cli")]
fn reduce_series(s: &[f64], how: &str) -> Result<f64, String> {
    if s.is_empty() {
        return Err("空轨迹".to_string());
    }
    Ok(match how {
        "final" => *s.last().unwrap(),
        "max" => s.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "min" => s.iter().copied().fold(f64::INFINITY, f64::min),
        "mean" => s.iter().sum::<f64>() / s.len() as f64,
        other => return Err(format!("未知 --reduce '{other}'（应为 final/max/mean/min）")),
    })
}

#[cfg(feature = "cli")]
#[allow(clippy::too_many_arguments)]
fn run_sweep(
    input: &PathBuf,
    drivers: &PathBuf,
    param: Option<&str>,
    range: Option<&str>,
    sensitivity: bool,
    percent: f64,
    var: &str,
    reduce: &str,
    params: Option<&PathBuf>,
    steps: Option<usize>,
    output: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::scenario::{load_drivers_csv, load_params_json};
    use equation_compiler::{parse_file, simulate, SimInput};
    use std::collections::HashMap;

    let file = parse_file(input)?;
    let (rows, driver_map) = load_drivers_csv(drivers)?;
    let steps = steps.unwrap_or(rows);
    let base: HashMap<String, f64> = match params {
        Some(p) => load_params_json(p)?,
        None => HashMap::new(),
    };

    // 用给定覆盖跑一次仿真、取 --var 的归约值
    let metric = |overrides: &HashMap<String, f64>| -> Result<f64, String> {
        let mut sim_in = SimInput::new(steps);
        sim_in.drivers = driver_map.clone();
        sim_in.param_overrides = overrides.clone();
        let out = simulate(&file, &sim_in).map_err(|e| format!("仿真失败: {e}"))?;
        let series = out
            .trajectories
            .get(var)
            .ok_or_else(|| format!("输出 '{var}' 不在轨迹里（向量变量请用 “{var}[1]” 形式）"))?;
        reduce_series(series, reduce)
    };

    if sensitivity {
        // —— OAT 全局敏感性：每个标量参数各 ±percent%，按对 var 的影响排序 ——
        let y0 = metric(&base)?;
        let pct = percent / 100.0;
        // (param, default, low, high, dVar, elasticity)
        let mut rows_out: Vec<(String, f64, f64, f64, f64, f64)> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        for (pname, p) in &file.parameters {
            if p.values.is_some() {
                continue; // 向量参数不参与
            }
            let d = base.get(pname).copied().unwrap_or(p.default);
            if d == 0.0 {
                skipped.push(pname.clone()); // 默认 0 无法相对扰动
                continue;
            }
            let mut lo = base.clone();
            lo.insert(pname.clone(), d * (1.0 - pct));
            let mut hi = base.clone();
            hi.insert(pname.clone(), d * (1.0 + pct));
            let ylo = metric(&lo).map_err(|e| format!("{pname}-: {e}"))?;
            let yhi = metric(&hi).map_err(|e| format!("{pname}+: {e}"))?;
            let dvar = yhi - ylo;
            let elasticity = if y0 != 0.0 { (dvar / y0) / (2.0 * pct) } else { f64::NAN };
            rows_out.push((pname.clone(), d, ylo, yhi, dvar, elasticity));
        }
        // 按对 var 的绝对影响从大到小
        rows_out.sort_by(|a, b| b.4.abs().partial_cmp(&a.4.abs()).unwrap_or(std::cmp::Ordering::Equal));

        let mut csv = format!("param,default,{var}_low,{var}_high,d{var},elasticity\n");
        for (p, d, ylo, yhi, dvar, el) in &rows_out {
            csv.push_str(&format!("{p},{d},{ylo},{yhi},{dvar},{el}\n"));
        }
        std::fs::write(output, csv)?;

        println!("✅ 敏感性扫描（每参数 ±{percent}%，基线 {var}({reduce})={y0:.6}）→ {}", output.display());
        println!("   对 {var} 的影响从大到小：");
        for (p, _, _, _, dvar, el) in rows_out.iter().take(12) {
            println!("     {p:<14} Δ{var}={dvar:+.6}   弹性={el:+.4}");
        }
        if !skipped.is_empty() {
            println!("   （默认值为 0、无法相对扰动而跳过：{}）", skipped.join(", "));
        }
        return Ok(());
    }

    // —— 单参数扫描 ——
    let param = param.ok_or("非 --sensitivity 模式须提供 --param（或加 --sensitivity 做全局敏感性）")?;
    let range = range.ok_or("非 --sensitivity 模式须提供 --range a:b:n")?;
    match file.parameters.get(param) {
        None => return Err(format!("参数 '{param}' 不在模型的 parameters 中").into()),
        Some(p) if p.values.is_some() => {
            return Err(format!("'{param}' 是向量参数（cohort 种子），不能用标量扫描").into())
        }
        _ => {}
    }
    let (a, b, npts) = parse_range(range)?;

    println!("🔬 扫描 {param} ∈ [{a}, {b}]（{npts} 点），输出 {var}（{reduce}）……");
    let mut csv = format!("{param},{var}_{reduce}\n");
    let mut results: Vec<(f64, f64)> = Vec::with_capacity(npts);
    for i in 0..npts {
        let v = if npts <= 1 { a } else { a + (b - a) * (i as f64) / ((npts - 1) as f64) };
        let mut ov = base.clone();
        ov.insert(param.to_string(), v);
        let r = metric(&ov).map_err(|e| format!("{param}={v}: {e}"))?;
        csv.push_str(&format!("{v},{r}\n"));
        results.push((v, r));
    }
    std::fs::write(output, csv)?;

    let lo = results.iter().copied().reduce(|x, y| if x.1 <= y.1 { x } else { y });
    let hi = results.iter().copied().reduce(|x, y| if x.1 >= y.1 { x } else { y });
    println!("✅ 扫描完成，结果写入 {}", output.display());
    if let (Some(lo), Some(hi)) = (lo, hi) {
        println!(
            "   {var}（{reduce}）范围 [{:.6}, {:.6}]；最小 @ {param}={}，最大 @ {param}={}",
            lo.1, hi.1, lo.0, hi.0
        );
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn run_export(input: &PathBuf, output: Option<&PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::{parse_directory, parse_file};

    let files = if input.is_dir() {
        parse_directory(input)?
    } else {
        vec![parse_file(input)?]
    };
    let json = equation_compiler::export::to_json_pretty(&files);
    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            println!("✅ 模型 JSON 契约已写入 {}", path.display());
        }
        None => println!("{json}"),
    }
    Ok(())
}

#[cfg(feature = "cli")]
#[allow(clippy::too_many_arguments)]
fn run_optimize(
    input: &PathBuf,
    spec: &PathBuf,
    drivers: Option<&PathBuf>,
    steps: Option<usize>,
    prescreen: bool,
    output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::optimize::{self, load_problem, Sense};
    use equation_compiler::parse_file;
    use equation_compiler::scenario::load_drivers_csv;

    let mut problem = load_problem(spec)?;

    // —— 耦合优化（C3）：spec 有 coupling 块 → 前向模型 = 多速率耦合仿真（input 忽略，模型在 coupling 里）——
    if problem.coupling.is_some() {
        return run_optimize_coupled(spec, &problem, output);
    }

    println!("🎯 优化模型: {}", input.display());
    let file = parse_file(input)?;

    // —— 解析环境驱动量：--drivers 优先，否则用 spec 里的 environment（相对 spec 目录解析）——
    let driver_path: PathBuf = match drivers {
        Some(p) => p.clone(),
        None => match &problem.environment {
            Some(env) => spec.parent().unwrap_or_else(|| std::path::Path::new(".")).join(env),
            None => {
                return Err("未提供环境驱动量：请加 --drivers，或在决策 spec 里写 environment:".into())
            }
        },
    };
    let (rows, driver_map) = load_drivers_csv(&driver_path)?;
    let steps = steps.unwrap_or(rows);

    let sense_of = |o: &equation_compiler::optimize::Objective| match o.sense {
        Sense::Max => "max",
        Sense::Min => "min",
    };

    // —— 敏感性预筛（单目标）：把对目标几乎无影响的旋钮固定在基线、缩小搜索 ——
    if prescreen {
        if problem.is_multi() {
            println!("   ⚠️ 预筛仅用于单目标，本 spec 为多目标 → 跳过预筛");
        } else {
            let pr = optimize::prescreen(&file, &problem, &driver_map, steps, 10.0, 0.01)?;
            let maxd = pr.deltas.iter().cloned().fold(0.0_f64, f64::max);
            println!("   🔬 敏感性预筛（±10%，目标 {}）：", problem.objective.expr);
            // 按敏感性降序打印
            let mut idx: Vec<usize> = (0..problem.knobs.len()).collect();
            idx.sort_by(|&a, &b| pr.deltas[b].partial_cmp(&pr.deltas[a]).unwrap_or(std::cmp::Ordering::Equal));
            for i in idx {
                let mark = if pr.kept.contains(&i) { "保留" } else { "固定" };
                let rel = if maxd > 0.0 { pr.deltas[i] / maxd } else { 0.0 };
                println!(
                    "     [{mark}] {:<16} |Δ目标|={:.6}（相对 {:.3}）",
                    problem.knobs[i].var, pr.deltas[i], rel
                );
            }
            // 把被剔除的旋钮边界收拢到基线（固定）→ 仅搜索保留旋钮
            for &i in &pr.dropped {
                problem.knobs[i].bounds = [pr.baseline[i], pr.baseline[i]];
            }
            if !pr.dropped.is_empty() {
                let names: Vec<&str> =
                    pr.dropped.iter().map(|&i| problem.knobs[i].var.as_str()).collect();
                println!("     → 固定 {} 个低敏感旋钮于基线：{}", pr.dropped.len(), names.join(", "));
            }
        }
    }

    // —— 多目标模式（提供了 objective2）：MO-DE 一次跑出 Pareto 权衡前沿 ——
    if problem.is_multi() {
        let o2 = problem.objective2.as_ref().unwrap();
        println!(
            "   旋钮 {} 个 | 环境 {} ({} 步) | MO-DE pop={} iters={} seed={}",
            problem.knobs.len(),
            driver_path.display(),
            steps,
            problem.optimizer.pop,
            problem.optimizer.iters,
            problem.optimizer.seed,
        );
        println!("   目标1 {} {}", sense_of(&problem.objective), problem.objective.expr);
        println!("   目标2 {} {}", sense_of(o2), o2.expr);

        let mr = optimize::run_mo(&file, &problem, &driver_map, steps)?;
        println!("\n✅ 多目标优化完成：Pareto 前沿 {} 点", mr.front.len());
        let names: Vec<&str> = problem.knobs.iter().map(|k| k.var.as_str()).collect();
        println!("   {:>13} {:>13}   旋钮({})", "目标1", "目标2", names.join(", "));
        for p in &mr.front {
            let objs = p
                .objectives
                .iter()
                .map(|v| format!("{v:>13.4}"))
                .collect::<Vec<_>>()
                .join(" ");
            let knobs = p.knobs.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>().join(", ");
            let feas = if p.feasible { "" } else { "  (违反约束)" };
            println!("   {objs}   [{knobs}]{feas}");
        }
        if let Some(path) = output {
            let json = optimize::mo_result_json(&file, &problem, &mr);
            std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
            println!("   结果已写入 {}", path.display());
        }
        return Ok(());
    }

    let sense_str = sense_of(&problem.objective);
    println!(
        "   旋钮 {} 个 | 环境 {} ({} 步) | DE pop={} iters={} seed={} | 目标 {sense_str} {}",
        problem.knobs.len(),
        driver_path.display(),
        steps,
        problem.optimizer.pop,
        problem.optimizer.iters,
        problem.optimizer.seed,
        problem.objective.expr,
    );

    // —— 校验 + 跑优化（与 serve 的 /api/optimize 共用 optimize::run）——
    let res = optimize::run(&file, &problem, &driver_map, steps)?;
    let best = &res.outcome;

    println!("\n✅ 优化完成");
    println!("   最优旋钮：");
    for (k, v) in problem.knobs.iter().zip(&res.best_knobs) {
        let unit = k.unit.as_deref().map(|u| format!(" {u}")).unwrap_or_default();
        println!("     {:<16} = {:.6}{unit}   [{}]", k.var, v, k.kind.as_str());
    }
    match best.objective {
        Some(obj) => println!("   目标值（{sense_str}）：{obj:.6}"),
        None => println!(
            "   目标值：⚠️ 最优候选仍无法求值（{}）",
            best.note.clone().unwrap_or_default()
        ),
    }
    if !problem.constraints.is_empty() {
        println!(
            "   约束（{}，惩罚 {:.6}）：",
            if best.feasible { "全部满足 ✓" } else { "存在违反 ✗" },
            best.penalty
        );
        for cs in &best.constraints {
            let mark = if cs.violation > 0.0 { "✗" } else { "✓" };
            let viol = if cs.violation > 0.0 {
                format!("   违反 {:.6}", cs.violation)
            } else {
                String::new()
            };
            println!("     {mark} {} = {:.6} ≤ {:.6}{viol}", cs.expr, cs.value, cs.max);
        }
    }
    if let (Some(first), Some(last)) = (res.history.first(), res.history.last()) {
        println!("   收敛：初代代价 {first:.6} → 末代 {last:.6}（共 {} 代）", res.history.len() - 1);
    }

    // —— 写结果 JSON（与 serve 同一份结构）——
    if let Some(path) = output {
        let json = optimize::result_json(&file, &problem, &res);
        std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
        println!("   结果已写入 {}", path.display());
    }

    Ok(())
}

/// `eqc optimize <任意> --spec coupled.yaml`（spec 含 coupling 块）：耦合优化（C3）。
/// 前向模型 = 多速率耦合仿真（温室↔作物，双向）；旋钮 = 温室/作物参数；目标归约作物轨迹。
#[cfg(feature = "cli")]
fn run_optimize_coupled(
    spec: &PathBuf,
    problem: &equation_compiler::optimize::Problem,
    output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::optimize::{run_coupled, CoupledModel};
    use equation_compiler::parse_file;
    use equation_compiler::scenario::load_drivers_csv;
    use equation_compiler::sim::{Agg, CoupledLink, FeedbackLink};

    let c = problem.coupling.as_ref().unwrap();
    let spec_dir = spec.parent().unwrap_or_else(|| std::path::Path::new("."));
    let rel = |p: &str| spec_dir.join(p);

    let fast = parse_file(&rel(&c.fast))?;
    let slow = parse_file(&rel(&c.slow))?;
    println!("🎯🔗 耦合优化: 温室 {} ↔ 作物 {}", fast.meta.id, slow.meta.id);

    let weather_path = c
        .weather
        .as_ref()
        .ok_or("耦合优化需在 coupling 块写 weather: 室外驱动 CSV")?;
    let (rows, weather_map) = load_drivers_csv(&rel(weather_path))?;

    // R = dt_slow秒/dt_fast秒；慢步数缺省 = 室外行数/R
    let dtf = fast.meta.dt_seconds.ok_or("温室模型缺 meta.dt_seconds")?;
    let dts = slow.meta.dt_seconds.ok_or("作物模型缺 meta.dt_seconds")?;
    let r = (dts / dtf).round().max(1.0) as usize;
    let slow_steps = c.steps.unwrap_or(rows / r);
    let need = slow_steps * r;
    if rows < need {
        return Err(format!("室外驱动 {rows} 行 < 慢步数·R = {need}").into());
    }
    let weather: std::collections::HashMap<String, Vec<f64>> = weather_map
        .into_iter()
        .map(|(k, v)| (k, v[..need.min(v.len())].to_vec()))
        .collect();

    let links: Vec<CoupledLink> = c
        .links
        .iter()
        .map(|l| {
            Ok(CoupledLink {
                to: l.to.clone(),
                from: l.from.clone(),
                agg: Agg::parse(&l.agg).ok_or_else(|| format!("未知聚合 '{}'", l.agg))?,
                scale: l.scale,
            })
        })
        .collect::<Result<_, String>>()?;
    let feedback: Vec<FeedbackLink> = c
        .feedback
        .iter()
        .map(|f| FeedbackLink { to: f.to.clone(), from: f.from.clone(), scale: f.scale, init: f.init })
        .collect();

    let base_fast_params: std::collections::HashMap<String, f64> =
        c.fast_params.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let base_slow_params: std::collections::HashMap<String, f64> =
        c.slow_params.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let model = CoupledModel {
        fast: &fast,
        slow: &slow,
        links,
        feedback,
        weather,
        slow_steps,
        base_fast_params,
        base_slow_params,
    };
    println!(
        "   旋钮 {} 个 | {} 慢步 × R={} | DE pop={} iters={} seed={}",
        problem.knobs.len(), slow_steps, r,
        problem.optimizer.pop, problem.optimizer.iters, problem.optimizer.seed
    );
    println!("   目标: {} ({})", problem.objective.expr, problem.objective.sense.as_str());

    let res = run_coupled(&model, problem)?;

    println!("\n✅ 耦合优化完成");
    println!("   最优目标值 = {:.6}", res.best_objective);
    println!("   最优旋钮:");
    for (k, v) in problem.knobs.iter().zip(&res.best_knobs) {
        println!("     {:<16} = {:.6}{}", k.var, v, k.unit.as_deref().map(|u| format!(" {u}")).unwrap_or_default());
    }
    let hist = &res.history;
    if hist.len() >= 2 {
        println!("   收敛: 代价 {:.6} → {:.6}（{} 代）", hist[0], hist[hist.len() - 1], hist.len() - 1);
    }

    if let Some(path) = output {
        let knobs: serde_json::Map<String, serde_json::Value> = problem
            .knobs
            .iter()
            .zip(&res.best_knobs)
            .map(|(k, v)| (k.var.clone(), serde_json::json!(v)))
            .collect();
        let j = serde_json::json!({
            "coupled": true,
            "fast": fast.meta.id, "slow": slow.meta.id,
            "best_objective": res.best_objective,
            "best_knobs": knobs,
            "objective": problem.objective.expr,
            "sense": problem.objective.sense.as_str(),
            "history": res.history,
        });
        std::fs::write(path, serde_json::to_string_pretty(&j)?)?;
        println!("   结果已写入 {}", path.display());
    }
    Ok(())
}

#[cfg(feature = "cli")]
#[allow(clippy::too_many_arguments)]
fn run_calibrate(
    input: &PathBuf,
    spec: &PathBuf,
    drivers: Option<&PathBuf>,
    observed: Option<&PathBuf>,
    steps: Option<usize>,
    output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::optimize::{self, load_problem, Sense};
    use equation_compiler::parse_file;
    use equation_compiler::scenario::{load_drivers_csv, load_observed_csv};

    println!("🔧 标定模型: {}", input.display());
    let file = parse_file(input)?;
    let problem = load_problem(spec)?;
    if problem.is_multi() {
        return Err("标定暂为单目标（误差最小化）：请用单个 objective".into());
    }
    let spec_dir = || spec.parent().unwrap_or_else(|| std::path::Path::new("."));

    // —— 同期天气驱动量：--drivers 优先，否则 spec 的 environment ——
    let driver_path: PathBuf = match drivers {
        Some(p) => p.clone(),
        None => match &problem.environment {
            Some(env) => spec_dir().join(env),
            None => return Err("缺同期天气：请加 --drivers，或在 spec 写 environment:".into()),
        },
    };
    let (rows, driver_map) = load_drivers_csv(&driver_path)?;
    let steps = steps.unwrap_or(rows);

    // —— 实测数据：--observed 优先，否则 spec 的 observed ——
    let obs_path: PathBuf = match observed {
        Some(p) => p.clone(),
        None => match &problem.observed {
            Some(o) => spec_dir().join(o),
            None => return Err("缺实测数据：请加 --observed，或在 spec 写 observed:".into()),
        },
    };
    let observed_data = load_observed_csv(&obs_path)?;
    let n_obs: usize = observed_data.values().map(|v| v.len()).sum();

    let sense_str = match problem.objective.sense {
        Sense::Max => "max",
        Sense::Min => "min",
    };
    println!(
        "   参数 {} 个 | 环境 {} ({} 步) | 实测 {} ({} 观测点 / {} 变量) | 目标 {sense_str} {}",
        problem.knobs.len(),
        driver_path.display(),
        steps,
        obs_path.display(),
        n_obs,
        observed_data.len(),
        problem.objective.expr,
    );

    // —— 跑标定（旋钮=参数、目标=误差；与决策优化共用 run_obs）——
    let res = optimize::run_obs(&file, &problem, &driver_map, steps, &observed_data)?;
    let best = &res.outcome;

    println!("\n✅ 标定完成");
    println!("   拟合参数：");
    for (k, v) in problem.knobs.iter().zip(&res.best_knobs) {
        let unit = k.unit.as_deref().map(|u| format!(" {u}")).unwrap_or_default();
        println!("     {:<16} = {:.6}{unit}", k.var, v);
    }
    match best.objective {
        Some(e) => println!("   拟合误差（{sense_str} {}）：{e:.6}", problem.objective.expr),
        None => println!("   ⚠️ 最优候选无法求值（{}）", best.note.clone().unwrap_or_default()),
    }
    if let (Some(first), Some(last)) = (res.history.first(), res.history.last()) {
        println!("   收敛：初代 {first:.6} → 末代 {last:.6}（共 {} 代）", res.history.len() - 1);
    }

    if let Some(path) = output {
        let json = optimize::result_json(&file, &problem, &res);
        std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
        println!("   结果已写入 {}", path.display());
    }
    Ok(())
}

#[cfg(feature = "cli")]
#[allow(clippy::too_many_arguments)]
fn run_identify(
    input: &PathBuf,
    spec: &PathBuf,
    drivers: Option<&PathBuf>,
    observables_arg: Option<&str>,
    steps: Option<usize>,
    output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::optimize::{self, load_problem, simulate_candidate, validate_problem};
    use equation_compiler::parse_file;
    use equation_compiler::scenario::load_drivers_csv;

    println!("🔬 可辨识性分析: {}", input.display());
    let file = parse_file(input)?;
    let problem = load_problem(spec)?;
    validate_problem(&file, &problem).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let driver_path: PathBuf = match drivers {
        Some(p) => p.clone(),
        None => match &problem.environment {
            Some(env) => spec.parent().unwrap_or_else(|| std::path::Path::new(".")).join(env),
            None => return Err("缺天气：请加 --drivers，或在 spec 写 environment:".into()),
        },
    };
    let (rows, driver_map) = load_drivers_csv(&driver_path)?;
    let steps = steps.unwrap_or(rows);

    // —— 候选可观测变量：--observables > spec.observables > 默认（所有 output 标量轨迹键）——
    let observables: Vec<String> = if let Some(s) = observables_arg {
        s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
    } else if let Some(list) = &problem.observables {
        list.clone()
    } else {
        // 默认：跑一次基线仿真，取所有 output 变量里直接成轨迹键（标量）的
        let baseline: Vec<f64> =
            problem.knobs.iter().map(|k| 0.5 * (k.bounds[0] + k.bounds[1])).collect();
        let out = simulate_candidate(&file, &problem, &baseline, &driver_map, steps)?;
        file.output_variables()
            .iter()
            .map(|(n, _)| n.to_string())
            .filter(|n| out.series(n).is_some())
            .collect()
    };
    if observables.is_empty() {
        return Err("无候选可观测变量（用 --observables 指定，或在 spec 写 observables:）".into());
    }

    println!(
        "   候选参数 {} 个 | 候选观测 {} 个 [{}] | 环境 {} ({} 步)",
        problem.knobs.len(),
        observables.len(),
        observables.join(", "),
        driver_path.display(),
        steps,
    );

    let rep = optimize::identifiability(&file, &problem, &driver_map, steps, &observables, 10.0, 0.01)?;

    println!("\n   参数 → 最该测的观测（相对敏感度，±10% 扰动引起的轨迹相对 RMS 变化）：");
    for p in &rep.params {
        if p.identifiable {
            let top = &p.per_observable[0];
            let others: Vec<String> = p
                .per_observable
                .iter()
                .skip(1)
                .filter(|(_, s)| *s > 0.0)
                .map(|(v, s)| format!("{v}={s:.4}"))
                .collect();
            let more = if others.is_empty() { String::new() } else { format!("（其它: {}）", others.join(", ")) };
            println!("     {:<16} → 测 {} (敏感度 {:.4}){more}", p.param, top.0, top.1);
        } else {
            println!("     {:<16} → ⚠️ 不可辨识：候选观测都约束不住它（需补测别的变量，或先固定它）", p.param);
        }
    }
    if !rep.confounded.is_empty() {
        println!("\n   ⚠️ 可能异参同效（敏感模式高度相关、难分辨，建议加处理梯度/多变量观测核实）：");
        for (a, b, r) in &rep.confounded {
            println!("     {a} ↔ {b}（相关 {r:.3}）");
        }
    }
    // 测量清单建议：可辨识参数所需观测的并集
    let mut need: Vec<String> = Vec::new();
    for p in &rep.params {
        if p.identifiable {
            let top = p.per_observable[0].0.clone();
            if !need.contains(&top) {
                need.push(top);
            }
        }
    }
    let unident: Vec<&str> = rep.params.iter().filter(|p| !p.identifiable).map(|p| p.param.as_str()).collect();
    println!("\n   📋 测量建议：至少测 [{}] 可约束 {} 个可辨识参数。", need.join(", "), rep.params.len() - unident.len());
    if !unident.is_empty() {
        println!("      不可辨识（这组观测下）：{} —— 需补测能反映它们的变量，或标定时固定。", unident.join(", "));
    }

    if let Some(path) = output {
        let params_json: Vec<serde_json::Value> = rep
            .params
            .iter()
            .map(|p| {
                serde_json::json!({
                    "param": p.param,
                    "identifiable": p.identifiable,
                    "sensitivities": p.per_observable.iter().map(|(v, s)| serde_json::json!({"observable": v, "sensitivity": s})).collect::<Vec<_>>(),
                })
            })
            .collect();
        let json = serde_json::json!({
            "model": file.meta.id,
            "observables": rep.observables,
            "params": params_json,
            "confounded": rep.confounded.iter().map(|(a, b, r)| serde_json::json!({"a": a, "b": b, "corr": r})).collect::<Vec<_>>(),
            "measure": need,
            "unidentifiable": unident,
        });
        std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
        println!("   报告已写入 {}", path.display());
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn run_check_dims(input: &PathBuf, strict: bool) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::units::{self, CouplingIssue};

    println!("🔬 量纲检查: {}", input.display());
    let compiler = Compiler::new().load_directory(input)?;
    let files = compiler.files();

    let mut errors = 0usize;
    let mut infos = 0usize;

    // 1) 每个模块内部的量纲一致性
    for file in files {
        let diags = units::check_equation_file(file);
        if !diags.is_empty() {
            println!("\n📄 模块 {}", file.meta.id);
            for d in &diags {
                println!("   ⚠️  [{}] {}", d.equation_id, d.message);
                errors += 1;
            }
        }
    }

    // 2) 跨模块耦合接口
    let couplings = units::check_coupling(files);
    if !couplings.is_empty() {
        println!("\n🔗 跨模块耦合");
        for c in &couplings {
            match &c.issue {
                // 量纲相同、仅单位不同：可自动换算，属提示而非错误
                CouplingIssue::ConversionNeeded { .. } => {
                    println!("   ℹ️  {} → {}: {}", c.from, c.to, c.message);
                    infos += 1;
                }
                _ => {
                    println!("   ❌ {} → {}: {}", c.from, c.to, c.message);
                    errors += 1;
                }
            }
        }
    }

    println!("\n────────────────────────────────────────");
    println!(
        "📊 模块数: {}，错误: {}，需换算提示: {}",
        files.len(),
        errors,
        infos
    );
    if errors == 0 {
        println!("✅ 未发现量纲错误");
    }

    if strict && errors > 0 {
        return Err(format!("量纲检查发现 {errors} 处错误").into());
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn run_graph(input: &PathBuf, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let compiler = Compiler::new()
        .load_directory(input)?
        .validate()?
        .build_dag()?;

    let dag = compiler.dag().ok_or("DAG 未构建")?;

    match format {
        "mermaid" => {
            println!("```mermaid");
            println!("graph TD");
            for edge in &dag.edges {
                println!("    {} --> {}", edge.from, edge.to);
            }
            println!("```");
        }
        "dot" => {
            println!("digraph equations {{");
            for edge in &dag.edges {
                println!("    \"{}\" -> \"{}\";", edge.from, edge.to);
            }
            println!("}}");
        }
        _ => {
            return Err(format!("未知图格式: {}", format).into());
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn run_list(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let compiler = Compiler::new().load_directory(input)?;

    println!("📋 方程列表\n");

    for file in compiler.files() {
        println!("## {} ({})", file.meta.name_cn, file.meta.id);
        println!("   模型: {}", file.meta.model);
        println!("   方程数: {}", file.equations.len());
        println!();

        for eq in &file.equations {
            println!("   - [{}] {}", eq.id, eq.name);
            if let Some(ref formula) = eq.formula_display {
                println!("     公式: {}", formula);
            }
        }
        println!();
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn run_convert(
    input: &str,
    output: Option<&PathBuf>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::sexpr;
    use std::fs;
    
    // 判断输入是文件还是表达式字符串
    let sexpr_content = if std::path::Path::new(input).exists() {
        println!("📄 读取S表达式文件: {}", input);
        fs::read_to_string(input)?
    } else {
        input.to_string()
    };
    
    // 解析S表达式
    let expr = sexpr::parse_to_expr(&sexpr_content).map_err(|e| format!("解析错误: {}", e))?;
    
    // 转换为输出格式
    let output_content = match format {
        "yaml" => {
            let yaml_value = sexpr::to_yaml_value(&expr);
            serde_yaml::to_string(&yaml_value)?
        }
        "json" => {
            let yaml_value = sexpr::to_yaml_value(&expr);
            serde_json::to_string_pretty(&yaml_value)?
        }
        _ => {
            return Err(format!("未知输出格式: {}", format).into());
        }
    };
    
    // 输出
    if let Some(output_path) = output {
        fs::write(output_path, &output_content)?;
        println!("✅ 转换完成: {}", output_path.display());
    } else {
        println!("{}", output_content);
    }
    
    Ok(())
}

#[cfg(feature = "cli")]
fn run_workflow(
    input: &PathBuf,
    output: &PathBuf,
    generate_operators: bool,
    sql_output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::sexpr;
    use std::fs;
    
    println!("📂 加载带注解的S表达式文件: {}", input.display());
    
    // SQL 输出目录（默认与 output 相同）
    let sql_dir = sql_output.unwrap_or(output);
    
    // 收集所有要处理的文件
    let files: Vec<PathBuf> = if input.is_dir() {
        // 递归查找 .sexpr 文件
        walkdir::WalkDir::new(input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().is_some_and(|ext| ext == "sexpr")
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    } else {
        vec![input.clone()]
    };
    
    if files.is_empty() {
        return Err("未找到 .sexpr 文件".into());
    }
    
    println!("   找到 {} 个文件", files.len());
    
    // 创建输出目录
    fs::create_dir_all(output)?;
    fs::create_dir_all(sql_dir)?;
    
    let mut all_modules = Vec::new();
    
    for file_path in &files {
        println!("📄 处理: {}", file_path.display());
        
        let content = fs::read_to_string(file_path)?;
        let module = sexpr::parse_annotated_sexpr(&content)
            .map_err(|e| format!("解析错误 {}: {}", file_path.display(), e))?;
        
        // 生成模块名
        let module_name = module.id.replace('.', "_").to_lowercase();
        
        // 守恒律验证
        let conservation_warnings = sexpr::workflow::verify_conservation_laws(&module);
        if !conservation_warnings.is_empty() {
            println!("   ⚠️  守恒律检查发现 {} 条警告:", conservation_warnings.len());
            for w in &conservation_warnings {
                println!("      [{:?}] {}: {}", w.level, w.node, w.message);
            }
        }

        // 生成 workflow.json（内部使用，用于 SQL 生成）
        let workflow_json = sexpr::generate_workflow_json(&module);
        let workflow_content = serde_json::to_string_pretty(&workflow_json)?;
        
        // 生成 SQL 导入语句（输出到 sql_dir）
        let sql_content = sexpr::generate_template_sql(&module, &workflow_content);
        let sql_path = sql_dir.join(format!("{}_template.sql", module_name));
        fs::write(&sql_path, &sql_content)?;
        println!("   ✅ 生成: {}", sql_path.display());
        
        // 生成算子代码（如果需要，输出到 output）
        if generate_operators {
            let operators_code = sexpr::generate_operators(&module);
            let operators_path = output.join(format!("{}_operators.rs", module_name));
            fs::write(&operators_path, &operators_code)?;
            println!("   ✅ 生成: {}", operators_path.display());
        }
        
        all_modules.push(module);
    }
    
    // 生成统一的注册代码（如果生成算子）
    if generate_operators && !all_modules.is_empty() {
        let register_code = sexpr::generate_register_code(&all_modules);
        let register_path = output.join("register.rs");
        fs::write(&register_path, &register_code)?;
        println!("   ✅ 生成: {}", register_path.display());
        
        // 生成 mod.rs
        let mut mod_code = String::new();
        mod_code.push_str("//! 自动生成的算子模块\n\n");
        for module in &all_modules {
            let module_name = module.id.replace('.', "_").to_lowercase();
            mod_code.push_str(&format!("pub mod {}_operators;\n", module_name));
        }
        mod_code.push_str("pub mod register;\n\n");
        mod_code.push_str("pub use register::register_generated_operators;\n");
        
        let mod_path = output.join("mod.rs");
        fs::write(&mod_path, &mod_code)?;
        println!("   ✅ 生成: {}", mod_path.display());
    }
    
    println!("\n✅ 生成完成!");
    println!("   模块数: {}", all_modules.len());
    println!("   算子数: {}", all_modules.iter().map(|m| m.operators.len()).sum::<usize>());
    println!("   输出目录: {}", output.display());
    
    println!("\n📋 使用说明:");
    println!("   SQL模板已生成到: {}", sql_dir.display());
    println!("   服务启动时会自动同步到数据库");
    
    if generate_operators {
        println!("\n   Rust算子代码已生成到: {}", output.display());
        println!("   在 registry/builder.rs 中引入注册函数:");
        println!("      use crate::lowcode::operators::generated::register_generated_operators;");
        println!("      register_generated_operators(&mut registry);");
        println!("   重新编译后端: cargo build");
    }
    
    Ok(())
}

/// 验证带注解的 S-expression 文件
#[cfg(feature = "cli")]
fn run_validate_sexpr(
    input: &PathBuf,
    verbose: bool,
    warn_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::sexpr;
    use std::fs;

    println!("🔍 验证 S-expression 文件: {}", input.display());
    println!();

    // 收集所有要处理的文件
    let files: Vec<PathBuf> = if input.is_dir() {
        walkdir::WalkDir::new(input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sexpr"))
            .map(|e| e.path().to_path_buf())
            .collect()
    } else {
        vec![input.clone()]
    };

    if files.is_empty() {
        return Err("未找到 .sexpr 文件".into());
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut all_valid = true;

    for file_path in &files {
        println!("📄 {}", file_path.display());
        
        let content = fs::read_to_string(file_path)?;
        
        // 解析文件
        let module = match sexpr::parse_annotated_sexpr(&content) {
            Ok(m) => m,
            Err(e) => {
                println!("   ❌ 解析错误: {}", e);
                all_valid = false;
                total_errors += 1;
                continue;
            }
        };

        // 验证模块
        let mut validator = sexpr::SExprValidator::new();
        let result = validator.validate(&module);

        if verbose {
            println!("{}", sexpr::format_validation_result(&result));
        } else {
            // 简洁输出
            if result.is_valid {
                println!("   ✅ 验证通过 (算子: {}, 警告: {})",
                    result.stats.operator_count,
                    result.warnings.len()
                );
            } else {
                println!("   ❌ 验证失败 (错误: {}, 警告: {})",
                    result.errors.len(),
                    result.warnings.len()
                );
                for err in &result.errors {
                    println!("      - {}", err.message);
                }
            }
        }

        total_errors += result.errors.len();
        total_warnings += result.warnings.len();
        if !result.is_valid {
            all_valid = false;
        }
    }

    println!();
    println!("───────────────────────────────────────────────────────────────────────");
    println!("📊 总计: {} 个文件, {} 个错误, {} 个警告",
        files.len(), total_errors, total_warnings
    );

    if all_valid {
        println!("✅ 所有文件验证通过");
        Ok(())
    } else if warn_only {
        println!("⚠️  存在错误但 --warn-only 已启用，继续执行");
        Ok(())
    } else {
        Err(format!("验证失败: {} 个错误", total_errors).into())
    }
}

/// 生成多模块 L2 级 Mermaid DAG
#[cfg(feature = "cli")]
fn run_graph_l2(inputs: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::sexpr;

    if inputs.len() < 2 {
        eprintln!("L2 图需要至少 2 个 S-expression 文件");
        std::process::exit(1);
    }

    let mut modules = Vec::new();
    for path in inputs {
        let source = std::fs::read_to_string(path)?;
        let module = sexpr::workflow::parse_annotated_sexpr(&source)?;
        println!("  已解析模块: {} ({})", module.name, module.id);
        modules.push(module);
    }

    let mermaid = sexpr::workflow::generate_l2_mermaid(&modules);
    println!("\n{}", mermaid);

    Ok(())
}

/// 输出 S-expression 书写规范
#[cfg(feature = "cli")]
fn run_sexpr_spec() -> Result<(), Box<dyn std::error::Error>> {
    use equation_compiler::sexpr;
    
    println!("{}", sexpr::generate_spec_doc());
    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI 功能未启用。请使用 --features cli 编译。");
    eprintln!("示例: cargo run --features cli -- build --input ./equations --output ./generated");
    std::process::exit(1);
}

#[cfg(all(test, feature = "cli"))]
mod cli_tests {
    use super::*;

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_range("1.0:5.0:9").unwrap(), (1.0, 5.0, 9));
        assert_eq!(parse_range("0:10:1").unwrap(), (0.0, 10.0, 1));
        assert!(parse_range("1:5").is_err()); // 缺点数
        assert!(parse_range("a:5:3").is_err()); // 非数值
        assert!(parse_range("1:5:0").is_err()); // 点数为 0
    }

    #[test]
    fn test_reduce_series() {
        let s = [1.0, 3.0, 2.0];
        assert_eq!(reduce_series(&s, "final").unwrap(), 2.0);
        assert_eq!(reduce_series(&s, "max").unwrap(), 3.0);
        assert_eq!(reduce_series(&s, "min").unwrap(), 1.0);
        assert!((reduce_series(&s, "mean").unwrap() - 2.0).abs() < 1e-9);
        assert!(reduce_series(&s, "bogus").is_err());
        assert!(reduce_series(&[], "final").is_err());
    }

    #[test]
    fn test_parse_init_overrides() {
        let m = parse_init_overrides("W_cane=420, C_reserve=66.5 , ChillAccum=0").unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!(m["W_cane"], 420.0);
        assert_eq!(m["C_reserve"], 66.5);
        assert_eq!(m["ChillAccum"], 0.0);
        assert!(parse_init_overrides("").unwrap().is_empty()); // 空串 → 空 map
        assert!(parse_init_overrides("W_cane").is_err()); // 缺 =
        assert!(parse_init_overrides("W_cane=abc").is_err()); // 非数值
        assert!(parse_init_overrides("=5").is_err()); // 名为空
    }
}
