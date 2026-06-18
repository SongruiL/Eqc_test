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

        /// 输出轨迹 CSV
        #[arg(short, long, default_value = "sim_output.csv")]
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
    },

    /// 导出模型的 JSON 契约（前端/工具消费用，可检视）
    Export {
        /// 模型文件（.eq.yaml）或目录
        input: PathBuf,

        /// 输出 JSON 文件（缺省打印到 stdout）
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
        Commands::Report { input, output } => run_report(&input, &output),
        Commands::Simulate { input, drivers, params, steps, output } => {
            run_simulate(&input, &drivers, params.as_ref(), steps, &output)
        }
        Commands::Serve { input, port, drivers, params } => {
            equation_compiler::serve::serve(&input, port, drivers.as_ref(), params.as_ref())
        }
        Commands::Export { input, output } => run_export(&input, output.as_ref()),
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

    let compiler = Compiler::new().load_directory(input)?.validate()?;

    println!("✅ 验证通过");
    println!("   - 模块数: {}", compiler.files().len());
    println!("   - 方程数: {}", compiler.equation_ids().len());

    Ok(())
}

#[cfg(feature = "cli")]
fn run_report(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let compiler = Compiler::new().load_directory(input)?.validate()?.build_dag()?;
    let dag = compiler.dag().ok_or("DAG 未构建")?;
    let html = equation_compiler::report::generate_report(compiler.files(), dag);
    std::fs::write(output, html)?;
    println!("✅ 报告已生成: {}", output.display());
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

    // 读参数覆盖 JSON（可选）
    if let Some(pjson) = params {
        sim_in.param_overrides = load_params_json(pjson)?;
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
