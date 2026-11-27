//! 3D ビューの色。値は sRGB（ガンマ空間）の 0〜1。

use egui::Color32;

/// 3D ビューの背景（egui のダークテーマのパネルより一段暗い色）。
pub const VIEW_BACKGROUND: Color32 = Color32::from_rgb(0x13, 0x13, 0x13);

/// 曲面の地の色（JS 試作のダーク表示と同じ）。
pub const SURFACE_BASE: [f32; 3] = rgb(126, 170, 198);
pub const CAGE_LINE: [f32; 3] = rgb(0x92, 0xa0, 0xac);
pub const CREASE_LINE: [f32; 3] = rgb(0xff, 0xb4, 0x8a);
pub const CAGE_POINT: [f32; 3] = rgb(0xee, 0xf5, 0xf9);

/// 判定の表示色。
pub const VERDICT_OK: Color32 = Color32::from_rgb(0x64, 0xd2, 0x64);
pub const VERDICT_BAD: Color32 = Color32::from_rgb(0xff, 0x6e, 0x64);

const fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}
