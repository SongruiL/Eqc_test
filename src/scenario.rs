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

/// 读**多处理**实测 CSV（多处理标定用）：须含 `treatment`（1 起处理号，与 spec `treatments:` 顺序对应）
/// + `DAT`（1 起天/步）两列，其余列 = 观测变量。按处理号拆成**逐处理** ObservedData，返回 Vec 按处理号
/// 1..N 排（缺号=空表·其误差贡献为 0）。稀疏规则同 [`load_observed_csv`]。
pub fn load_observed_by_treatment(path: &Path) -> Result<Vec<HashMap<String, Vec<(usize, f64)>>>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or_else(|| "多处理实测 CSV 为空".to_string())?;
    let names: Vec<String> = header.split(',').map(|s| s.trim().to_string()).collect();
    let dat_col = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case("DAT"))
        .ok_or_else(|| "多处理实测 CSV 首行须含 DAT 列（1 起的天/步）".to_string())?;
    let tr_col = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case("treatment"))
        .ok_or_else(|| "多处理实测 CSV 首行须含 treatment 列（1 起处理号）".to_string())?;
    let var_cols: Vec<usize> = (0..names.len()).filter(|&i| i != dat_col && i != tr_col).collect();

    let mut per: Vec<HashMap<String, Vec<(usize, f64)>>> = Vec::new();
    let mut rownum = 1usize;
    for line in lines {
        rownum += 1;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != names.len() {
            return Err(format!("多处理实测 CSV 第 {rownum} 行列数与表头不符"));
        }
        let tr: usize = fields[tr_col]
            .trim()
            .parse()
            .map_err(|_| format!("多处理实测 CSV 第 {rownum} 行 treatment 不是整数: '{}'", fields[tr_col].trim()))?;
        if tr == 0 {
            return Err(format!("多处理实测 CSV 第 {rownum} 行 treatment 须 ≥1"));
        }
        let day: usize = fields[dat_col]
            .trim()
            .parse()
            .map_err(|_| format!("多处理实测 CSV 第 {rownum} 行 DAT 不是整数: '{}'", fields[dat_col].trim()))?;
        // 按需扩到 tr 个处理表（每表含全部观测变量键、初值空）。
        while per.len() < tr {
            let mut m = HashMap::new();
            for &i in &var_cols {
                m.insert(names[i].clone(), Vec::new());
            }
            per.push(m);
        }
        for &i in &var_cols {
            let cell = fields[i].trim();
            if cell.is_empty() {
                continue; // 稀疏：那天没测
            }
            let v: f64 = cell
                .parse()
                .map_err(|_| format!("多处理实测 CSV 第 {rownum} 行列 '{}' 无法解析: '{cell}'", names[i]))?;
            per[tr - 1].get_mut(&names[i]).unwrap().push((day, v));
        }
    }
    if per.is_empty() {
        return Err("多处理实测 CSV 无数据行".into());
    }
    Ok(per)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_observed_by_treatment_routes_and_gaps() {
        // treatment 1、3 有数据、2 缺（跳号）：应得 3 个槽、槽2 为空（供 run_calibrate 守卫捕获）。
        let tmp = std::env::temp_dir().join("eqc_test_obs_by_treatment.csv");
        std::fs::write(&tmp, "treatment,DAT,obs_X\n1,10,1.5\n1,20,2.5\n3,10,9.0\n").unwrap();
        let per = load_observed_by_treatment(&tmp).unwrap();
        assert_eq!(per.len(), 3, "处理号最大 3 → 3 个槽");
        assert_eq!(per[0].get("obs_X").unwrap(), &vec![(10, 1.5), (20, 2.5)], "处理1 两点、按 DAT 归");
        assert!(per[1].get("obs_X").unwrap().is_empty(), "处理2 跳号 → 空槽（守卫据此报错）");
        assert_eq!(per[2].get("obs_X").unwrap(), &vec![(10, 9.0)], "处理3 一点");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_load_observed_by_treatment_missing_column() {
        // 缺 treatment 列 → 明确报错（非 panic）。
        let tmp = std::env::temp_dir().join("eqc_test_obs_no_treatment.csv");
        std::fs::write(&tmp, "DAT,obs_X\n10,1.5\n").unwrap();
        assert!(load_observed_by_treatment(&tmp).is_err(), "缺 treatment 列须报错");
        let _ = std::fs::remove_file(&tmp);
    }
}
