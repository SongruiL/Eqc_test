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
