//! 3D ビューを囲むパネル。

pub mod menu_bar;
pub mod properties_panel;
pub mod status_bar;
pub mod tool_panel;

/// 左右のパネルの開閉。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelLayout {
    pub tools_open: bool,
    pub properties_open: bool,
}

impl PanelLayout {
    /// これより狭い画面では、3D ビューを広く取るため左右のパネルを閉じて始める。
    pub const NARROW_WIDTH: f32 = 720.0;

    /// 画面幅がまだ取れていない（0 以下・非有限）ときは `None`。
    pub fn for_width(width: f32) -> Option<Self> {
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        let open = width >= Self::NARROW_WIDTH;
        Some(Self {
            tools_open: open,
            properties_open: open,
        })
    }
}

/// 左右のパネルそれぞれの幅の上限を、画面幅のこの割合までにする（3D ビューを画面の半分以上に保つ）。
pub const MAX_PANEL_WIDTH_FRACTION: f32 = 0.25;

/// パネルの幅の範囲 `(最小, 最大)`。最小が最大を超えないようにし、画面幅が不正なら元の範囲を返す。
pub fn panel_width_range(screen_width: f32, min: f32, max: f32) -> (f32, f32) {
    let cap = if screen_width.is_finite() && screen_width > 0.0 {
        screen_width * MAX_PANEL_WIDTH_FRACTION
    } else {
        max
    };
    let hi = max.min(cap);
    (min.min(hi), hi)
}

/// メニューバーと重ならないよう、上に余白を取った控えめな見出し。
pub fn panel_heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).size(15.0).strong());
    ui.add_space(2.0);
}

/// パネルの操作のうち、アプリの状態全体に効くもの。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PanelActions {
    pub reset_camera: bool,
}

impl PanelActions {
    pub fn merge(&mut self, other: Self) {
        self.reset_camera |= other.reset_camera;
    }
}
