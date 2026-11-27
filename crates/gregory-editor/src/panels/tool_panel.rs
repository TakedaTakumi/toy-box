//! 左のツールパネル：形状と表示の設定。

use egui::{CollapsingHeader, ComboBox, Slider, Ui};
use gregory_core::PRESETS;

use super::PanelActions;
use crate::settings::{DisplaySettings, SUBDIVISION_RANGE, Shading};

pub fn show(ui: &mut Ui, settings: &mut DisplaySettings) -> PanelActions {
    let mut actions = PanelActions::default();
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("ツール");

        CollapsingHeader::new("形状")
            .default_open(true)
            .show(ui, |ui| {
                ComboBox::from_id_salt("preset")
                    .selected_text(settings.preset.name())
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for preset in PRESETS {
                            ui.selectable_value(&mut settings.preset, preset, preset.name());
                        }
                    });
            });

        CollapsingHeader::new("表示")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for shading in Shading::ALL {
                        ui.selectable_value(&mut settings.shading, shading, shading.label());
                    }
                });
                ui.checkbox(&mut settings.show_surface, "曲面");
                ui.checkbox(&mut settings.show_cage, "ケージ");
                let mut n = settings.subdivisions();
                ui.add(Slider::new(&mut n, SUBDIVISION_RANGE).text("分割数"));
                settings.set_subdivisions(n);
            });

        CollapsingHeader::new("視点")
            .default_open(true)
            .show(ui, |ui| {
                if ui.button("視点をリセット").clicked() {
                    actions.reset_camera = true;
                }
            });

        CollapsingHeader::new("操作説明")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("controls_help")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        for (action, how) in CONTROLS {
                            ui.label(*action);
                            ui.label(*how);
                            ui.end_row();
                        }
                    });
            });
    });
    actions
}

const CONTROLS: &[(&str, &str)] = &[
    ("回転", "ドラッグ"),
    ("拡大縮小", "ホイール / ピンチ"),
    ("視点をリセット", "ダブルクリック"),
    ("スムース / フラット", "S キー"),
];
