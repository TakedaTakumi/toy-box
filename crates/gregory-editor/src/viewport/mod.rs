//! 中央の 3D ビュー：入力で選択・移動・視点を操作し、wgpu の描画を egui に差し込む。

mod callback;
mod input;
mod overlay;
mod renderer;

use std::sync::Arc;

use egui::{Align2, CursorIcon, FontId, Rect, Response, Sense, Ui};

use crate::camera::{OrbitCamera, ScreenView};
use crate::editing::Editor;
use crate::editing::gizmo::{self, Gizmo, Handle, ScreenPos};
use crate::editing::pick::pick_element;
use crate::editing::selection::Element;
use crate::mesh_buffers::SceneGeometry;
use crate::palette;
use crate::scene::Scene;
use callback::SceneCallback;
use input::PointerInput;
use renderer::{SceneRenderer, Uniforms, Visibility};

/// 深度バッファのビット数。eframe の起動設定と、自前のパイプラインの両方で使う。
pub const DEPTH_BUFFER_BITS: u8 = 24;

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
    pub show_curves: bool,
    /// 曲面を組み立てられなかったときなど、ビューに重ねて出す文。
    pub message: Option<&'a str>,
}

/// 画面上の位置にあるもの。
#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    Handle(Handle),
    Element(Element),
}

fn target_at(
    view: &ScreenView,
    editor: &Editor,
    scene: Option<&Scene>,
    pos: ScreenPos,
) -> Option<Target> {
    let gizmo = editor.gizmo_center().and_then(|c| gizmo::gizmo(view, c));
    if let Some(handle) = gizmo.as_ref().and_then(|g| gizmo::pick_handle(g, pos)) {
        return Some(Target::Handle(handle));
    }
    let scene = scene?;
    let mode = editor.selection().mode();
    pick_element(view, &scene.cage, &scene.model, mode, pos).map(Target::Element)
}

fn to_screen(rect: Rect, p: egui::Pos2) -> ScreenPos {
    [f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y)]
}

/// ポインタ操作を、選択・移動・視点の回転に振り分ける。
fn handle_pointer(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    camera: &mut OrbitCamera,
    editor: &mut Editor,
    scene: Option<&Scene>,
) {
    let accept_press = response.hovered() || response.is_pointer_button_down_on();
    let inputs = ui.input(|i| input::collect(i, rect, accept_press, editor.is_dragging()));
    for event in inputs {
        let view = ScreenView::of(camera, rect.width(), rect.height());
        match event {
            PointerInput::Press {
                pos,
                force_orbit,
                additive,
            } => {
                let target = if force_orbit {
                    None
                } else {
                    target_at(&view, editor, scene, pos)
                };
                match target {
                    Some(Target::Handle(handle)) => editor.press_handle(&view, handle, pos),
                    Some(Target::Element(element)) => editor.press_element(element, additive, pos),
                    None => editor.press_empty(pos, !force_orbit && !additive),
                }
            }
            PointerInput::Move(pos) => {
                if let Some([dx, dy]) = editor.pointer_moved(&view, pos) {
                    camera.orbit(dx as f32, dy as f32);
                }
            }
            PointerInput::Release => editor.release(),
            PointerInput::Cancel => editor.cancel_drag(),
        }
    }

    if response.double_clicked() {
        let view = ScreenView::of(camera, rect.width(), rect.height());
        let on_target = response
            .interact_pointer_pos()
            .and_then(|p| target_at(&view, editor, scene, to_screen(rect, p)))
            .is_some();
        if !on_target {
            camera.reset();
        }
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
}

/// ホバー中のもの。カーソルの形を変え、ハンドルなら選択の色で描くために返す。
fn hover_feedback(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    view: &ScreenView,
    editor: &Editor,
    scene: Option<&Scene>,
) -> Option<Handle> {
    if editor.drag().is_moving() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        return None;
    }
    if editor.is_dragging() || !response.hovered() {
        return None;
    }
    let pos = response.hover_pos()?;
    let alt = ui.input(|i| i.modifiers.alt);
    if alt {
        return None;
    }
    let target = target_at(view, editor, scene, to_screen(rect, pos))?;
    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    match target {
        Target::Handle(handle) => Some(handle),
        Target::Element(_) => None,
    }
}

/// 入力を処理した後の、3D ビューの領域とホバー中のハンドル。
pub struct Interaction {
    rect: Rect,
    hovered_handle: Option<Handle>,
}

/// 3D ビューの領域を取り、入力で選択・移動・視点を変える。`scene` はピックに使う、いまのケージから組み立てた曲面。
pub fn interact(
    ui: &mut Ui,
    camera: &mut OrbitCamera,
    editor: &mut Editor,
    scene: Option<&Scene>,
) -> Interaction {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    handle_pointer(ui, rect, &response, camera, editor, scene);
    let view = ScreenView::of(camera, rect.width(), rect.height());
    let hovered_handle = hover_feedback(ui, rect, &response, &view, editor, scene);
    Interaction {
        rect,
        hovered_handle,
    }
}

/// 曲面・ケージと、移動ハンドル・案内の文を描く。
pub fn paint(
    ui: &Ui,
    interaction: &Interaction,
    camera: &OrbitCamera,
    editor: &Editor,
    content: &ViewportContent<'_>,
    gpu: &GpuStatus,
) {
    let rect = interaction.rect;
    let view = ScreenView::of(camera, rect.width(), rect.height());
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
                pixels_per_point,
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
                        curves: content.show_curves,
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

    let gizmo: Option<Gizmo> = editor.gizmo_center().and_then(|c| gizmo::gizmo(&view, c));
    if let Some(gizmo) = &gizmo {
        let lit = editor.active_handle().or(interaction.hovered_handle);
        overlay::paint_gizmo(&painter, rect.min, gizmo, lit);
    }

    let margin = egui::vec2(8.0, 8.0);
    let hint = painter.layout(
        "クリックで選択 / 選択した要素・ハンドルをドラッグで移動 / 空所のドラッグで回転 / ホイール・ピンチで拡大縮小".to_owned(),
        FontId::proportional(12.0),
        ui.visuals().weak_text_color(),
        (rect.width() - margin.x * 2.0).max(0.0),
    );
    painter.galley(
        rect.left_top() + margin,
        hint,
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

fn scene_uniforms(camera: &OrbitCamera, size_px: [f32; 2], pixels_per_point: f32) -> Uniforms {
    let frame = camera.frame();
    let v4 = |v: gregory_core::Vec3| [v[0] as f32, v[1] as f32, v[2] as f32, 0.0];
    let [r, g, b] = palette::SURFACE_BASE;
    let [hr, hg, hb] = palette::SELECTION;
    Uniforms {
        view_proj: camera.view_projection(size_px[0], size_px[1]),
        eye: v4(frame.eye),
        right: v4(frame.right),
        up: v4(frame.up),
        forward: v4(frame.forward),
        surface_color: [r, g, b, 1.0],
        highlight_color: [hr, hg, hb, palette::FACE_TINT],
        params: [size_px[0], size_px[1], pixels_per_point, 0.0],
    }
}
