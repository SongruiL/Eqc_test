//! 算子导出器
//!
//! 本模块从 ModuleDef 生成 AST JSON（用于动态注册到 lowcode 平台）
//! 和 SQL 模板导入语句。
//!
//! Rust 代码生成已移除——算子通过运行时 AST 解释器动态执行。

use super::workflow::{ModuleDef, OperatorDef};

/// 生成算子的 AST JSON 表示（用于数据库存储 / 动态注册）
///
/// # 参数
/// - `module`: 模块定义
///
/// # 返回
/// 包含所有算子定义的 JSON 字符串
pub fn generate_ast_json(module: &ModuleDef) -> String {
    let operators: Vec<serde_json::Value> = module
        .operators
        .iter()
        .filter_map(|op| operator_to_json(op).ok())
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "module_id": module.id,
        "module_name": module.name,
        "module_description": module.description,
        "operators": operators,
    }))
    .unwrap_or_default()
}

/// 将单个 OperatorDef 转为 JSON（包含 AST）
fn operator_to_json(op: &OperatorDef) -> Result<serde_json::Value, String> {
    let ast_json = op
        .expr
        .as_ref()
        .map(|e| serde_json::to_value(e).map_err(|e| e.to_string()))
        .transpose()?;

    Ok(serde_json::json!({
        "id": op.id,
        "name": op.name,
        "operator_type": op.operator_type.as_str(),
        "category": op.category,
        "description": op.description,
        "latex_formula": op.latex_formula,
        "inputs": op.inputs.iter().map(|i| serde_json::json!({
            "name": i.name,
            "data_type": i.data_type,
            "required": i.required,
            "description": i.description,
            "default_value": i.default_value,
            "latex_name": i.latex_name,
            "paper_ref": i.paper_ref,
        })).collect::<Vec<_>>(),
        "outputs": op.outputs.iter().map(|o| serde_json::json!({
            "name": o.name,
            "data_type": o.data_type,
            "description": o.description,
            "latex_name": o.latex_name,
        })).collect::<Vec<_>>(),
        "ast": ast_json,
    }))
}

// ============================================
// 向后兼容的公开 API（生成空代码的桩函数）
// ============================================

/// 生成 Rust 算子代码 —— 已废弃，返回空字符串
///
/// 算子现在通过 AST 解释器动态执行，不再需要 Rust 代码生成。
/// 此函数保留以兼容 CLI 工具和现有调用链。
pub fn generate_operators(_module: &ModuleDef) -> String {
    String::from("// 算子已迁移为动态注册模式，不再生成 Rust 代码。\n")
}

/// 生成注册代码 —— 已废弃，返回空字符串
///
/// 算子现在通过数据库持久化 + 运行时动态注册。
pub fn generate_register_code(_modules: &[ModuleDef]) -> String {
    String::from("// 算子已迁移为动态注册模式，不再生成注册代码。\n")
}

// ============================================
// SQL 生成函数（保留）
// ============================================

/// 生成 PostgreSQL INSERT 语句用于导入模板到数据库
pub fn generate_template_sql(module: &ModuleDef, workflow_json: &str) -> String {
    let escaped_name = escape_sql_string(&module.name);
    let escaped_desc = escape_sql_string(&module.description);
    let escaped_json = escape_sql_string(workflow_json);

    let template_id = generate_template_id(&module.id);

    format!(
        r#"-- ============================================================================
-- 自动生成 - {name}
-- ============================================================================
-- 由 equation-compiler 从 S表达式 自动生成
-- ============================================================================

INSERT INTO "lowcode-templates" (
    "id",
    "name",
    "description",
    "category",
    "definition",
    "is-active",
    "created-by",
    "created-at",
    "updated-at"
)
VALUES (
    '{id}',
    '{name}',
    '{description}',
    '{category}',
    '{json}'::jsonb,
    true,
    NULL,
    NOW(),
    NOW()
)
ON CONFLICT ("id") DO UPDATE SET
    "name" = EXCLUDED."name",
    "description" = EXCLUDED."description",
    "category" = EXCLUDED."category",
    "definition" = EXCLUDED."definition",
    "updated-at" = NOW();

DO $$
BEGIN
    RAISE NOTICE '模板已导入: {name}';
END $$;
"#,
        id = template_id,
        name = escaped_name,
        description = escaped_desc,
        category = "物候模型",
        json = escaped_json,
    )
}

/// 生成算子种子 SQL（将算子 AST 存入数据库）
pub fn generate_operator_seed_sql(module: &ModuleDef) -> String {
    let mut sql = String::new();

    sql.push_str("-- ============================================================================\n");
    sql.push_str(&format!(
        "-- 算子种子数据 - {}\n",
        module.name
    ));
    sql.push_str("-- ============================================================================\n\n");

    for op in &module.operators {
        if let Ok(json_val) = operator_to_json(op) {
            let json_str = serde_json::to_string(&json_val).unwrap_or_default();
            let escaped_json = escape_sql_string(&json_str);
            let escaped_name = escape_sql_string(&op.name);
            let escaped_desc = escape_sql_string(&op.description);
            let escaped_category = escape_sql_string(&op.category);
            let escaped_id = escape_sql_string(&op.id);

            sql.push_str(&format!(
                r#"INSERT INTO "lowcode-operator-sources" (
    "operator-id", "module-id", "name", "description", "category",
    "operator-def", "version"
)
VALUES (
    '{id}', '{module_id}', '{name}', '{desc}', '{category}',
    '{json}'::jsonb, 1
)
ON CONFLICT ("operator-id") DO UPDATE SET
    "name" = EXCLUDED."name",
    "description" = EXCLUDED."description",
    "category" = EXCLUDED."category",
    "operator-def" = EXCLUDED."operator-def",
    "version" = "lowcode-operator-sources"."version" + 1,
    "updated-at" = NOW();

"#,
                id = escaped_id,
                module_id = escape_sql_string(&module.id),
                name = escaped_name,
                desc = escaped_desc,
                category = escaped_category,
                json = escaped_json,
            ));
        }
    }

    sql
}

// ============================================
// 辅助函数
// ============================================

/// 转义 SQL 字符串
fn escape_sql_string(s: &str) -> String {
    s.replace('\'', "''")
}

/// 生成模板 ID（基于模块 ID 生成稳定的 UUID）
fn generate_template_id(module_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    module_id.hash(&mut hasher);
    let hash = hasher.finish();

    format!(
        "01950000-{:04x}-7000-8000-{:012x}",
        (hash >> 48) as u16,
        hash & 0xFFFFFFFFFFFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::workflow::parse_annotated_sexpr;

    const SAMPLE_SEXPR: &str = r#"
;; @module: phenoflex.chill
;; @name: PhenoFlex冷量模型
;; @description: 基于Dynamic Model的冷量累积计算

;; @operator: phenoflex.temp_kelvin
;; @name: 温度转开尔文
;; @category: 物理转换
;; @description: 将摄氏度转换为开尔文温度
;; @input: T, Number, required, 温度(摄氏度)
;; @input: offset, Number, optional, 偏移量, 273
;; @output: TK, Number, 开尔文温度
(add T offset)
"#;

    #[test]
    fn test_generate_ast_json() {
        let module = parse_annotated_sexpr(SAMPLE_SEXPR).unwrap();
        let json = generate_ast_json(&module);

        assert!(json.contains("phenoflex.temp_kelvin"));
        assert!(json.contains("温度转开尔文"));
        assert!(json.contains("\"ast\""));
    }

    #[test]
    fn test_generate_operators_stub() {
        let module = parse_annotated_sexpr(SAMPLE_SEXPR).unwrap();
        let code = generate_operators(&module);
        assert!(code.contains("动态注册模式"));
    }

    #[test]
    fn test_generate_register_code_stub() {
        let module = parse_annotated_sexpr(SAMPLE_SEXPR).unwrap();
        let code = generate_register_code(&[module]);
        assert!(code.contains("动态注册模式"));
    }
}
