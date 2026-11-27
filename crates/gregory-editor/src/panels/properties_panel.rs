//! 右のプロパティパネル：連続性の検証と、形の数。

use egui::{CollapsingHeader, Grid, RichText, Ui};
use gregory_core::GregoryError;

use crate::palette;
use crate::readouts::{G1Verdict, SceneStats, format_g0, format_g1, g1_verdict};
use crate::scene::Scene;

pub fn show(ui: &mut Ui, scene: Result<&Scene, &GregoryError>) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("プロパティ");
        let scene = match scene {
            Ok(scene) => scene,
            Err(error) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("曲面を組み立てられませんでした：{error}"),
                );
                return;
            }
        };
        let check = &scene.model.check;
        let stats = SceneStats::of(scene);

        CollapsingHeader::new("連続性の検証")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("continuity")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("G0 位置の最大の隙間");
                        ui.monospace(format_g0(check.g0));
                        ui.end_row();
                        ui.label("G1 法線の最大差");
                        ui.monospace(format_g1(check));
                        ui.end_row();
                        ui.label("判定");
                        let verdict = g1_verdict(check);
                        let text = RichText::new(verdict.label()).strong();
                        match verdict {
                            G1Verdict::Satisfied => ui.colored_label(palette::VERDICT_OK, text),
                            G1Verdict::Violated => ui.colored_label(palette::VERDICT_BAD, text),
                            G1Verdict::NoSharedEdges => ui.label(text),
                        };
                        ui.end_row();
                        ui.label("辺（滑らか / 折れ目）");
                        ui.monospace(format!("{} / {}", stats.smooth_edges, stats.creased_edges));
                        ui.end_row();
                    });
            });

        CollapsingHeader::new("形状の情報")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("stats")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        let rows = [
                            ("頂点", stats.vertices.to_string()),
                            ("辺", stats.edges.to_string()),
                            ("面", stats.faces.to_string()),
                            ("パッチ", stats.patches.to_string()),
                            (
                                "四角形 / 三角形",
                                format!("{} / {}", stats.quads, stats.tris),
                            ),
                        ];
                        for (name, value) in rows {
                            ui.label(name);
                            ui.monospace(value);
                            ui.end_row();
                        }
                    });
            });
    });
}
