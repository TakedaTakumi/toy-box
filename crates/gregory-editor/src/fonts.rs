//! UI のフォントの登録。

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

const UI_FONT_NAME: &str = "gregory-ui-sans";
static UI_FONT: &[u8] = include_bytes!("../assets/fonts/gregory-ui-sans.ttf");

/// 日本語を表示するため、同梱のフォントを等幅・プロポーショナルの両方の先頭に置く。
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        UI_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(UI_FONT)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, UI_FONT_NAME.to_owned());
    }
    ctx.set_fonts(fonts);
}
