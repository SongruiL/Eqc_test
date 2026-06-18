//! 情景数据加载：驱动量 CSV（逐日时间序列）+ 参数覆盖 JSON。
//!
//! 「模型结构 与 情景数据 分离」：模型文件只写结构与方程，某一季的天气/观测从这里进来。
//! `eqc simulate` 与 `eqc serve` 共用本模块，避免重复。

use std::collections::HashMap;
use std::path::Path;

/// 读驱动量 CSV：首行变量名，每行一天。返回（行数, 变量名 -> 列向量）。
pub fn load_drivers_csv(path: &Path) -> Result<(usize, HashMap<String, Vec<f64>>), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or_else(|| "驱动量 CSV 为空".to_string())?;
    let names: Vec<String> = header.split(',').map(|s| s.trim().to_string()).collect();
    let mut cols: Vec<Vec<f64>> = vec![Vec::new(); names.len()];
    let mut rows = 0usize;
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != names.len() {
            return Err(format!(
                "驱动量 CSV 第 {} 行列数({})与表头({})不符",
                rows + 2,
                fields.len(),
                names.len()
            ));
        }
        for (i, f) in fields.iter().enumerate() {
            let v: f64 = f
                .trim()
                .parse()
                .map_err(|_| format!("驱动量 CSV 第 {} 行无法解析数值: '{}'", rows + 2, f.trim()))?;
            cols[i].push(v);
        }
        rows += 1;
    }
    Ok((rows, names.into_iter().zip(cols).collect()))
}

/// 读参数覆盖 JSON：`{"name": value, ...}`。
pub fn load_params_json(path: &Path) -> Result<HashMap<String, f64>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let v: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("参数 JSON 解析失败: {e}"))?;
    let obj = v.as_object().ok_or_else(|| "参数 JSON 应为对象 {name: value}".to_string())?;
    let mut out = HashMap::new();
    for (k, val) in obj {
        let f = val.as_f64().ok_or_else(|| format!("参数 {k} 不是数值"))?;
        out.insert(k.clone(), f);
    }
    Ok(out)
}
