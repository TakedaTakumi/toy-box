//! 中央の 3D ビュー：入力でカメラを動かし、wgpu の描画を egui に差し込む。

mod callback;
mod renderer;

use std::sync::Arc;

use egui::{Align2, FontId, Sense, Ui};

use crate::camera::OrbitCamera;
use crate::mesh_buffers::SceneGeometry;
use crate::palette;
use callback::SceneCallback;
use renderer::{SceneRenderer, Uniforms, Visibility};

/// 深度バッファのビット数。eframe の起動設定と、自前のパイプラインの両方で使う。
pub const DEPTH_BUFFER_BITS: u8 = 24;

/// ケージの点の一辺（ポイント）。
const POINT_SIZE: f32 = 6.0;
/// ホイール 1 ポイントあたりの拡大縮小の指数。
const WHEEL_ZOOM_PER_POINT: f64 = 0.0024;

/// 描画の準備ができたか。できなければ理由を持つ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuStatus {
    Ready { backend: &'static str },
    Unavailable(String),
}

/// 自前の描画資源を egui_wgpu に登録する。
pub fn install_renderer(render_state: Option<&egui_wgpu::RenderState>) -> GpuStatus {
    let Some(state) = render_state else {
        return GpuStatus::Unavailable("wgpu の描画環境を取得できませんでした".to_owned());
    };
    let depth_format = egui_wgpu::depth_format_from_bits(DEPTH_BUFFER_BITS, 0);
    let scene = SceneRenderer::new(&state.device, state.target_format, depth_format);
    state.renderer.write().callback_resources.insert(scene);
    let backend = crate::readouts::backend_label(
        state.adapter.get_info().backend,
        cfg!(target_arch = "wasm32"),
    );
    GpuStatus::Ready { backend }
}

/// 3D ビューに描くもの。
pub struct ViewportContent<'a> {
    pub geometry: &'a Arc<SceneGeometry>,
    pub revision: u64,
    pub show_surface: bool,
    pub show_cage: bool,
    /// 曲面を組み立てられなかったときなど、ビューに重ねて出す文。
    pub message: Option<&'a str>,
}

pub fn show(ui: &mut Ui, camera: &mut OrbitCamera, content: &ViewportContent<'_>, gpu: &GpuStatus) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

    if response.dragged() {
        let delta = response.drag_delta();
        camera.orbit(delta.x, delta.y);
    }
    if response.double_clicked() {
        camera.reset();
    }
    if response.hovered() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        if scroll != 0.0 {
            camera.zoom((-f64::from(scroll) * WHEEL_ZOOM_PER_POINT).exp());
        }
        if (pinch - 1.0).abs() > f32::EPSILON {
            camera.zoom(1.0 / f64::from(pinch));
        }
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, palette::VIEW_BACKGROUND);

    match gpu {
        GpuStatus::Ready { .. } => {
            let pixels_per_point = ui.ctx().pixels_per_point();
            let uniforms = scene_uniforms(
                camera,
                [
                    rect.width() * pixels_per_point,
                    rect.height() * pixels_per_point,
                ],
                POINT_SIZE * pixels_per_point,
            );
            painter.add(egui_wgpu::Callback::new_paint_callback(
                rect,
                SceneCallback {
                    geometry: Arc::clone(content.geometry),
                    revision: content.revision,
                    uniforms,
                    visibility: Visibility {
                        surface: content.show_surface,
                        cage: content.show_cage,
                    },
                },
            ));
        }
        GpuStatus::Unavailable(reason) => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                reason,
                FontId::proportional(14.0),
                ui.visuals().error_fg_color,
            );
        }
    }

    let margin = egui::vec2(8.0, 8.0);
    painter.text(
        rect.left_top() + margin,
        Align2::LEFT_TOP,
        "ドラッグで回転 / ホイール・ピンチで拡大縮小 / ダブルクリックで視点をリセット",
        FontId::proportional(12.0),
        ui.visuals().weak_text_color(),
    );
    if let Some(message) = content.message {
        painter.text(
            rect.left_bottom() + egui::vec2(margin.x, -margin.y),
            Align2::LEFT_BOTTOM,
            message,
            FontId::proportional(13.0),
            ui.visuals().error_fg_color,
        );
    }
}

fn scene_uniforms(camera: &OrbitCamera, size_px: [f32; 2], point_size_px: f32) -> Uniforms {
    let frame = camera.frame();
    let v4 = |v: gregory_core::Vec3| [v[0] as f32, v[1] as f32, v[2] as f32, 0.0];
    let [r, g, b] = palette::SURFACE_BASE;
    Uniforms {
        view_proj: camera.view_projection(size_px[0], size_px[1]),
        eye: v4(frame.eye),
        right: v4(frame.right),
        up: v4(frame.up),
        forward: v4(frame.forward),
        surface_color: [r, g, b, 1.0],
        params: [size_px[0], size_px[1], point_size_px, 0.0],
    }
}
