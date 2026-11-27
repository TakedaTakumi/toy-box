//! アプリの状態と、1フレームの処理。

use std::sync::Arc;

use egui::{CentralPanel, Event, Panel, Ui};
use gregory_core::{GregoryError, Selection};

use crate::camera::OrbitCamera;
use crate::editing::{EditAction, Editor};
use crate::fonts;
use crate::memo::Memo;
use crate::mesh_buffers::{Highlight, SceneGeometry, scene_geometry};
use crate::panels::{self, PanelActions, PanelLayout};
use crate::readouts::{FrameTimer, SceneStats};
use crate::scene::{BuildKey, Scene, build_scene};
use crate::settings::{DisplaySettings, Shading};
use crate::shortcuts::{Shortcut, shortcut};
use crate::viewport::{self, GpuStatus, ViewportContent};

pub struct GregoryEditorApp {
    settings: DisplaySettings,
    /// 画面幅が初めて有効になったフレームで1回だけ決める。
    layout: Option<PanelLayout>,
    camera: OrbitCamera,
    editor: Editor,
    scene: Memo<BuildKey, Result<Scene, GregoryError>>,
    /// キーは、組み立て直した曲面の `revision`、シェーディング、選択。
    geometry: Memo<(u64, Shading, Selection), Arc<SceneGeometry>>,
    frame_timer: FrameTimer,
    gpu: GpuStatus,
}

impl GregoryEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        fonts::install(&cc.egui_ctx);
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let gpu = viewport::install_renderer(cc.wgpu_render_state.as_ref());
        if let GpuStatus::Unavailable(reason) = &gpu {
            log::error!("{reason}");
        }
        Self {
            settings: DisplaySettings::default(),
            layout: None,
            camera: OrbitCamera::default(),
            editor: Editor::default(),
            scene: Memo::default(),
            geometry: Memo::default(),
            frame_timer: FrameTimer::default(),
            gpu,
        }
    }

    fn handle_shortcuts(&mut self, ui: &Ui) {
        let ctx = ui.ctx();
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let pressed: Vec<Shortcut> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| match event {
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => shortcut(*key, *modifiers),
                    _ => None,
                })
                .collect()
        });
        for s in pressed {
            match s {
                Shortcut::ToggleShading => self.settings.shading = self.settings.shading.toggled(),
                // ドラッグ中の編集は Editor が受け付けない
                Shortcut::Edit(action) => self.apply_edit(Some(action)),
            }
        }
    }

    fn apply_edit(&mut self, action: Option<EditAction>) {
        if let Some(action) = action {
            self.editor.apply(action);
        }
    }

    /// ケージか設定が変わったときだけ、曲面の組み立てと GPU 用の配列の作成をやり直す。
    fn refresh_scene(&mut self) {
        let previous_revision = self.scene.revision();
        let key = BuildKey {
            cage_revision: self.editor.revision(),
            subdivisions: self.settings.subdivisions(),
        };
        let cage = self.editor.cage();
        self.scene
            .get_or_compute(key, |key| build_scene(cage, key.subdivisions));
        let scene_revision = self.scene.revision();
        if scene_revision != previous_revision
            && let Some(Err(error)) = self.scene.value()
        {
            log::warn!("曲面を組み立てられませんでした：{error}");
        }
        let shading = self.settings.shading;
        let selection = self.editor.selection().items().clone();
        let scene = self.scene.value();
        self.geometry
            .get_or_compute(
                (scene_revision, shading, selection),
                |(_, _, selection)| match scene {
                    Some(Ok(scene)) => Arc::new(scene_geometry(
                        &scene.cage,
                        &scene.model,
                        shading,
                        &Highlight::of(selection),
                    )),
                    _ => Arc::new(SceneGeometry::default()),
                },
            );
    }
}

impl eframe::App for GregoryEditorApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.frame_timer.record(frame.info().cpu_usage);
        self.handle_shortcuts(ui);
        let settings_before = self.settings;
        let status_before = self.editor.status();
        let ctx = ui.ctx().clone();
        // 最初のフレームは画面の大きさが未確定で、egui が仮の大きな値を返すため使わない
        if self.layout.is_none() && ctx.input(|i| i.raw.screen_rect.is_some()) {
            self.layout = PanelLayout::for_width(ctx.content_rect().width());
        }
        if self.layout.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        let mut layout = self.layout.unwrap_or(PanelLayout {
            tools_open: false,
            properties_open: false,
        });
        let screen_width = ctx.content_rect().width();
        let (tools_min, tools_max) = panels::panel_width_range(screen_width, 150.0, 260.0);
        let (props_min, props_max) = panels::panel_width_range(screen_width, 150.0, 280.0);
        let mut actions = PanelActions::default();

        let status = self.editor.status();
        Panel::top("menu_bar").show(ui, |ui| {
            actions.merge(panels::menu_bar::show(
                ui,
                &mut self.settings,
                &mut layout,
                &status,
            ));
        });
        self.apply_edit(actions.edit.take());

        self.refresh_scene();
        let stats = match self.scene.value() {
            Some(Ok(scene)) => Some(SceneStats::of(scene)),
            _ => None,
        };
        let status = self.editor.status();
        Panel::bottom("status_bar").show(ui, |ui| {
            panels::status_bar::show(ui, &status, stats, self.frame_timer.average_ms(), &self.gpu);
        });

        Panel::left("tools")
            .resizable(true)
            .default_size(200.0)
            .size_range(tools_min..=tools_max)
            .show_collapsible(ui, &mut layout.tools_open, |ui| {
                actions.merge(panels::tool_panel::show(ui, &mut self.settings, &status));
            });
        self.apply_edit(actions.edit.take());

        self.refresh_scene();
        let status = self.editor.status();
        Panel::right("properties")
            .resizable(true)
            .default_size(210.0)
            .size_range(props_min..=props_max)
            .show_collapsible(ui, &mut layout.properties_open, |ui| {
                if let Some(scene) = self.scene.value() {
                    panels::properties_panel::show(ui, scene.as_ref(), &status);
                }
            });

        if self.layout.is_some() {
            self.layout = Some(layout);
        }
        if actions.reset_camera {
            self.camera.reset();
        }

        let revision_before_view = self.editor.revision();
        let selection_before_view = self.editor.selection().clone();
        CentralPanel::no_frame().show(ui, |ui| {
            let scene = match self.scene.value() {
                Some(Ok(scene)) => Some(scene),
                _ => None,
            };
            let interaction = viewport::interact(ui, &mut self.camera, &mut self.editor, scene);
            // 入力で変わったケージと選択を、このフレームの描画に間に合わせる
            self.refresh_scene();
            let message = match self.scene.value() {
                Some(Err(error)) => Some(format!("曲面を組み立てられませんでした：{error}")),
                _ => None,
            };
            let empty = Arc::new(SceneGeometry::default());
            let content = ViewportContent {
                geometry: self.geometry.value().unwrap_or(&empty),
                revision: self.geometry.revision(),
                show_surface: self.settings.show_surface,
                show_cage: self.settings.show_cage,
                message: message.as_deref(),
            };
            viewport::paint(
                ui,
                &interaction,
                &self.camera,
                &self.editor,
                &content,
                &self.gpu,
            );
        });

        // ビューやパネルでの変更は、このフレームの他のパネルにまだ出ていないため、もう1フレーム描く
        if self.settings != settings_before
            || self.editor.status() != status_before
            || self.editor.revision() != revision_before_view
            || *self.editor.selection() != selection_before_view
        {
            ui.ctx().request_repaint();
        }
    }
}
