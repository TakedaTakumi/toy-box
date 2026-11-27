//! アプリの状態と、1フレームの処理。

use std::sync::Arc;

use egui::{CentralPanel, Key, Panel, Ui};
use gregory_core::GregoryError;

use crate::camera::OrbitCamera;
use crate::fonts;
use crate::memo::Memo;
use crate::mesh_buffers::{SceneGeometry, scene_geometry};
use crate::panels::{self, PanelActions, PanelLayout};
use crate::readouts::{FrameTimer, SceneStats};
use crate::scene::{BuildKey, Scene, build_scene};
use crate::settings::{DisplaySettings, Shading};
use crate::viewport::{self, GpuStatus, ViewportContent};

pub struct GregoryEditorApp {
    settings: DisplaySettings,
    /// 画面幅が初めて有効になったフレームで1回だけ決める。
    layout: Option<PanelLayout>,
    camera: OrbitCamera,
    scene: Memo<BuildKey, Result<Scene, GregoryError>>,
    /// キーは、組み立て直した曲面の `revision` と、シェーディング。
    geometry: Memo<(u64, Shading), Arc<SceneGeometry>>,
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
        if ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.is_none()) {
            self.settings.shading = self.settings.shading.toggled();
        }
    }

    /// 設定が変わったときだけ、曲面の組み立てと GPU 用の配列の作成をやり直す。
    fn refresh_scene(&mut self) {
        let previous_revision = self.scene.revision();
        self.scene
            .get_or_compute(self.settings.build_key(), build_scene);
        let scene_revision = self.scene.revision();
        if scene_revision != previous_revision
            && let Some(Err(error)) = self.scene.value()
        {
            log::warn!("曲面を組み立てられませんでした：{error}");
        }
        let shading = self.settings.shading;
        let scene = self.scene.value();
        self.geometry
            .get_or_compute((scene_revision, shading), |_| match scene {
                Some(Ok(scene)) => Arc::new(scene_geometry(&scene.cage, &scene.model, shading)),
                _ => Arc::new(SceneGeometry::default()),
            });
    }
}

impl eframe::App for GregoryEditorApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.frame_timer.record(frame.info().cpu_usage);
        self.handle_shortcuts(ui);
        let settings_before = self.settings;
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

        Panel::top("menu_bar").show(ui, |ui| {
            actions.merge(panels::menu_bar::show(ui, &mut self.settings, &mut layout));
        });

        self.refresh_scene();
        let stats = match self.scene.value() {
            Some(Ok(scene)) => Some(SceneStats::of(scene)),
            _ => None,
        };
        Panel::bottom("status_bar").show(ui, |ui| {
            panels::status_bar::show(ui, stats, self.frame_timer.average_ms(), &self.gpu);
        });

        Panel::left("tools")
            .resizable(true)
            .default_size(200.0)
            .size_range(tools_min..=tools_max)
            .show_collapsible(ui, &mut layout.tools_open, |ui| {
                actions.merge(panels::tool_panel::show(ui, &mut self.settings));
            });

        self.refresh_scene();
        Panel::right("properties")
            .resizable(true)
            .default_size(210.0)
            .size_range(props_min..=props_max)
            .show_collapsible(ui, &mut layout.properties_open, |ui| {
                if let Some(scene) = self.scene.value() {
                    panels::properties_panel::show(ui, scene.as_ref());
                }
            });

        if self.layout.is_some() {
            self.layout = Some(layout);
        }
        if actions.reset_camera {
            self.camera.reset();
        }

        let message = match self.scene.value() {
            Some(Err(error)) => Some(format!("曲面を組み立てられませんでした：{error}")),
            _ => None,
        };
        let empty = Arc::new(SceneGeometry::default());
        let geometry = self.geometry.value().unwrap_or(&empty);
        let content = ViewportContent {
            geometry,
            revision: self.geometry.revision(),
            show_surface: self.settings.show_surface,
            show_cage: self.settings.show_cage,
            message: message.as_deref(),
        };
        CentralPanel::no_frame().show(ui, |ui| {
            viewport::show(ui, &mut self.camera, &content, &self.gpu);
        });

        // 左パネルでの変更は、このフレームのステータスバーにまだ出ていないため、もう1フレーム描く
        if self.settings != settings_before {
            ui.ctx().request_repaint();
        }
    }
}
