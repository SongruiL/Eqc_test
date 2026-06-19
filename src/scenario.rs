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

/// 读**实测数据** CSV（参数标定用）：首列须为 `DAT`（1 起的天数），其余列 = 观测变量。
/// **稀疏**：空单元格表示那天没测、跳过。返回 `名 -> [(天, 值)]`（与 `optimize::ObservedData` 一致）。
pub fn load_observed_csv(path: &Path) -> Result<HashMap<String, Vec<(usize, f64)>>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or_else(|| "实测 CSV 为空".to_string())?;
    let names: Vec<String> = header.split(',').map(|s| s.trim().to_string()).collect();
    let dat_col = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case("DAT"))
        .ok_or_else(|| "实测 CSV 首行须含 DAT 列（1 起的天数）".to_string())?;

    let mut out: HashMap<String, Vec<(usize, f64)>> = HashMap::new();
    for (i, n) in names.iter().enumerate() {
        if i != dat_col {
            out.insert(n.clone(), Vec::new());
        }
    }
    let mut rownum = 1usize;
    for line in lines {
        rownum += 1;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != names.len() {
            return Err(format!("实测 CSV 第 {rownum} 行列数与表头不符"));
        }
        let day: usize = fields[dat_col]
            .trim()
            .parse()
            .map_err(|_| format!("实测 CSV 第 {rownum} 行 DAT 不是整数: '{}'", fields[dat_col].trim()))?;
        for (i, f) in fields.iter().enumerate() {
            if i == dat_col {
                continue;
            }
            let cell = f.trim();
            if cell.is_empty() {
                continue; // 稀疏：那天没测
            }
            let v: f64 = cell
                .parse()
                .map_err(|_| format!("实测 CSV 第 {rownum} 行列 '{}' 无法解析: '{cell}'", names[i]))?;
            out.get_mut(&names[i]).unwrap().push((day, v));
        }
    }
    Ok(out)
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
