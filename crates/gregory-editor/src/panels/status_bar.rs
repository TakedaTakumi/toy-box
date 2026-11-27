//! 下のステータスバー。

use egui::Ui;

use crate::readouts::SceneStats;
use crate::viewport::GpuStatus;

pub fn show(ui: &mut Ui, stats: Option<SceneStats>, frame_ms: Option<f32>, gpu: &GpuStatus) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        if let Some(stats) = stats {
            ui.label(format!("頂点 {}", stats.vertices));
            ui.label(format!("面 {}", stats.faces));
            ui.label(format!("三角形 {}", stats.triangles));
        }
        match frame_ms {
            Some(ms) => ui.label(format!("フレーム {ms:.1} ms")),
            None => ui.label("フレーム - ms"),
        };
        match gpu {
            GpuStatus::Ready { backend } => ui.label(*backend),
            GpuStatus::Unavailable(_) => ui.colored_label(ui.visuals().error_fg_color, "描画不可"),
        };
    });
}
