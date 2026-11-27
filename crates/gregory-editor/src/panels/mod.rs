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

    pub fn for_width(width: f32) -> Self {
        let narrow = width < Self::NARROW_WIDTH;
        let open = !narrow;
        Self {
            tools_open: open,
            properties_open: open,
        }
    }
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
