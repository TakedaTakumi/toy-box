//! 3D ビューの色。値は sRGB（ガンマ空間）の 0〜1。

use egui::Color32;

/// 3D ビューの背景（egui のダークテーマのパネルより一段暗い色）。
pub const VIEW_BACKGROUND: Color32 = Color32::from_rgb(0x13, 0x13, 0x13);

/// 曲面の地の色（JS 試作のダーク表示と同じ）。
pub const SURFACE_BASE: [f32; 3] = rgb(126, 170, 198);
pub const CAGE_LINE: [f32; 3] = rgb(0x92, 0xa0, 0xac);
pub const CREASE_LINE: [f32; 3] = rgb(0xff, 0xb4, 0x8a);
pub const CAGE_POINT: [f32; 3] = rgb(0xee, 0xf5, 0xf9);

/// 選択中の要素の色（JS 試作のダーク表示の `--warm`）。
pub const SELECTION: [f32; 3] = rgb(0xff, 0x8c, 0x5e);
pub const SELECTION_UI: Color32 = Color32::from_rgb(0xff, 0x8c, 0x5e);
/// 選択中の面で、地の色に選択の色を混ぜる割合（JS 試作と同じ）。
pub const FACE_TINT: f32 = 0.55;

/// 移動ハンドルの軸の色（JS 試作のダーク表示の `--ax-x`・`--ax-y`・`--ax-z`）。
pub const AXIS_X: Color32 = Color32::from_rgb(0xd9, 0x73, 0x6b);
pub const AXIS_Y: Color32 = Color32::from_rgb(0x6f, 0xbd, 0x84);
pub const AXIS_Z: Color32 = Color32::from_rgb(0x70, 0x96, 0xe0);
pub const GIZMO_CENTER: Color32 = Color32::from_rgb(0xee, 0xf5, 0xf9);

/// 判定の表示色。
pub const VERDICT_OK: Color32 = Color32::from_rgb(0x64, 0xd2, 0x64);
pub const VERDICT_BAD: Color32 = Color32::from_rgb(0xff, 0x6e, 0x64);

const fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}
