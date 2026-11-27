//! 左のツールパネル：形状・選択・編集・移動・表示の設定。

use egui::{Button, CollapsingHeader, Slider, Ui};
use gregory_core::{PRESETS, SelectMode};

use super::PanelActions;
use crate::editing::gizmo::Axis;
use crate::editing::selection::mode_label;
use crate::editing::{EditAction, EditStatus};
use crate::settings::{DisplaySettings, SUBDIVISION_RANGE, Shading};

pub fn show(ui: &mut Ui, settings: &mut DisplaySettings, status: &EditStatus) -> PanelActions {
    let mut chosen: Option<EditAction> = None;
    let mut reset_camera = false;
    let mut edit = |action: EditAction| {
        chosen.get_or_insert(action);
    };
    let editable = !status.dragging;
    egui::ScrollArea::vertical().show(ui, |ui| {
        super::panel_heading(ui, "ツール");

        CollapsingHeader::new("形状")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for preset in PRESETS {
                        if ui
                            .add_enabled(editable, Button::new(preset.name()))
                            .clicked()
                        {
                            edit(EditAction::NewShape(preset));
                        }
                    }
                });
                ui.weak("いまの形を置き換えます（元に戻せます）");
            });

        CollapsingHeader::new("選択")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for mode in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
                        let button = Button::selectable(status.mode == mode, mode_label(mode));
                        if ui.add_enabled(editable, button).clicked() {
                            edit(EditAction::SetMode(mode));
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.add_enabled(editable, Button::new("全選択")).clicked() {
                        edit(EditAction::SelectAll);
                    }
                    if ui.add_enabled(editable, Button::new("選択解除")).clicked() {
                        edit(EditAction::ClearSelection);
                    }
                });
            });

        CollapsingHeader::new("編集")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let undo = Button::new("元に戻す");
                    if ui.add_enabled(editable && status.can_undo, undo).clicked() {
                        edit(EditAction::Undo);
                    }
                    let redo = Button::new("やり直し");
                    if ui.add_enabled(editable && status.can_redo, redo).clicked() {
                        edit(EditAction::Redo);
                    }
                });
            });

        CollapsingHeader::new("移動")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let options = [None, Some(Axis::X), Some(Axis::Y), Some(Axis::Z)];
                    for lock in options {
                        let label = lock.map_or("自由", Axis::label);
                        let button = Button::selectable(status.axis_lock == lock, label);
                        if ui.add_enabled(editable, button).clicked() {
                            edit(EditAction::SetAxisLock(lock));
                        }
                    }
                });
                ui.weak("要素のドラッグを、選んだ軸の向きに限ります");
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
                    reset_camera = true;
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
    PanelActions {
        reset_camera,
        edit: chosen,
    }
}

const CONTROLS: &[(&str, &str)] = &[
    ("選択", "クリック"),
    ("選択に追加・解除", "Shift / Ctrl + クリック"),
    ("移動", "選択した要素かハンドルをドラッグ"),
    ("軸に固定して移動", "X / Y / Z キー（同じキーで自由に戻す）"),
    ("点 / 辺 / 面", "1 / 2 / 3 キー"),
    ("全選択", "Ctrl + A"),
    ("選択解除", "Esc / 空所をクリック"),
    ("元に戻す", "Ctrl + Z"),
    ("やり直し", "Ctrl + Shift + Z / Ctrl + Y"),
    ("回転", "空所をドラッグ / 右・中ボタン / Alt + ドラッグ"),
    ("拡大縮小", "ホイール / ピンチ"),
    ("視点をリセット", "空所をダブルクリック"),
    ("スムース / フラット", "S キー"),
];
