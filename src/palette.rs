//! 配色**单一真相源**（2D 报告 + 3D 拓扑共用）。
//!
//! 设计（用户 2026-06-26 拍板）：
//! - **Rust 拥有**子系统配色：一组有序鲜色 `MODULE_VIVID`（= 已认可的 3D 深底色）作基准，
//!   2D 浅底色由鲜色**向白混合**派生（保留色相 + 相对饱和；比纯色相派生更能保住手调区分，
//!   如棕 `#b08968` 与橙 `#ff8c42` 色相相近但混白后仍可分）。
//! - **槽位 = 作者 `meta.modules` 声明顺序**（光合在前…），2D 与 3D 都从这里取 →
//!   **同一子系统两边同色相**（3D 鲜、2D 浅）。耦合多文件按文件序拼接、同名子系统并一槽。
//! - 暴露给前端：3D 经 `Layout3dJson.module_color`（鲜调）读取；2D 报告 Rust 直接烤进 SVG。

use indexmap::IndexMap;

use crate::schema::EquationFile;

/// 子系统基准鲜色（深底 3D 用）。16 色覆盖到耦合视图（温室×番茄 14 子系统）不绕回重色。
/// 这是配色的**唯一真相源**；2D 浅调由此派生（[`lighten`]）。
pub const MODULE_VIVID: [&str; 16] = [
    "#4f9dff", "#ff8c42", "#3ddc97", "#c77dff", "#ff5d8f", "#ffd23f", "#2ec4b6", "#ff6b6b",
    "#a3e635", "#bdb2ff", "#f0a6ca", "#7dd3fc", "#e879f9", "#b08968", "#6366f1", "#5eead4",
];

/// 2D 浅底填充的"向白混合"系数（0=原色，1=纯白）。浅色报告底上要够浅、衬深色文字。
const LIGHT_MIX: f64 = 0.66;

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("0"), 16).unwrap_or(0);
    (p(0), p(2), p(4))
}

/// 把一个 `#rrggbb` 按系数向白混合 → 浅色（保留色相，提亮）。
pub fn lighten(hex: &str, t: f64) -> String {
    let (r, g, b) = parse_hex(hex);
    let mix = |c: u8| ((c as f64) + (255.0 - c as f64) * t).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", mix(r), mix(g), mix(b))
}

/// 槽位 → 鲜调（3D）。超过调色板长度则循环。
pub fn vivid_for_slot(slot: usize) -> &'static str {
    MODULE_VIVID[slot % MODULE_VIVID.len()]
}

/// 槽位 → 浅调（2D）。
pub fn light_for_slot(slot: usize) -> String {
    lighten(vivid_for_slot(slot), LIGHT_MIX)
}

/// 模型里作者声明的子系统 → 槽位（按 `meta.modules` 声明顺序；耦合多文件按序拼接、同名并一槽）。
/// 只含**显式命名**的子系统（与 [`crate::graph`] 的 `node_modules` 口径一致）；自动桶不在内。
pub fn module_slots(files: &[EquationFile]) -> IndexMap<String, usize> {
    let mut slots: IndexMap<String, usize> = IndexMap::new();
    for f in files {
        for name in f.meta.modules.keys() {
            if !slots.contains_key(name) {
                let n = slots.len();
                slots.insert(name.clone(), n);
            }
        }
    }
    slots
}

/// 子系统名 → (鲜调 3D, 浅调 2D)，声明顺序定色。2D/3D 共用此分配 → 同子系统同色相。
pub fn module_colors(files: &[EquationFile]) -> IndexMap<String, (String, String)> {
    module_slots(files)
        .into_iter()
        .map(|(name, slot)| (name, (vivid_for_slot(slot).to_string(), light_for_slot(slot))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighten_moves_toward_white() {
        // 向白混合：每通道都应 ≥ 原值，且 t=1 得纯白。
        assert_eq!(lighten("#000000", 1.0), "#ffffff");
        assert_eq!(lighten("#4f9dff", 0.0), "#4f9dff");
        let l = lighten("#4f9dff", 0.66);
        let (r, g, b) = parse_hex(&l);
        assert!(r >= 0x4f && g >= 0x9d && b >= 0xff - 1, "浅调各通道不应变暗：{l}");
    }

    #[test]
    fn slots_follow_declared_order_and_dedup() {
        use crate::graph::bipartite::tests::toy;
        let mut a = toy(vec![("e1", "y", vec!["x"])]);
        a.meta.id = "A".into();
        a.meta.modules.insert("光合".into(), vec!["e1".into()]);
        a.meta.modules.insert("水".into(), vec![]);
        let mut b = toy(vec![("e2", "z", vec!["y"])]);
        b.meta.id = "B".into();
        b.meta.modules.insert("水".into(), vec!["e2".into()]); // 同名 → 并一槽
        b.meta.modules.insert("氮".into(), vec![]);
        let s = module_slots(&[a, b]);
        assert_eq!(s.get("光合"), Some(&0)); // 声明顺序：光合→0
        assert_eq!(s.get("水"), Some(&1)); //              水→1
        assert_eq!(s.get("氮"), Some(&2)); // B 的氮→2（水已占 1，不重分）
        assert_eq!(s.len(), 3);
    }
}
