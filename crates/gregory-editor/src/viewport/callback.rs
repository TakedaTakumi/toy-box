//! egui の描画の途中で、3D ビューを wgpu で描く。

use std::sync::Arc;

use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};

use super::renderer::{SceneRenderer, Uniforms, Visibility};
use crate::mesh_buffers::SceneGeometry;

pub struct SceneCallback {
    pub geometry: Arc<SceneGeometry>,
    pub revision: u64,
    pub uniforms: Uniforms,
    pub visibility: Visibility,
}

impl CallbackTrait for SceneCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(renderer) = callback_resources.get_mut::<SceneRenderer>() {
            renderer.prepare(device, queue, &self.geometry, self.revision, &self.uniforms);
        }
        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let Some(renderer) = callback_resources.get::<SceneRenderer>() else {
            return;
        };
        let viewport = info.viewport_in_pixels();
        let clip = info.clip_rect_in_pixels();
        let to_u32 = |v: i32| u32::try_from(v).unwrap_or(0);
        if viewport.width_px <= 0 || viewport.height_px <= 0 {
            return;
        }
        if clip.width_px <= 0 || clip.height_px <= 0 {
            return;
        }
        render_pass.set_viewport(
            viewport.left_px as f32,
            viewport.top_px as f32,
            viewport.width_px as f32,
            viewport.height_px as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            to_u32(clip.left_px),
            to_u32(clip.top_px),
            to_u32(clip.width_px),
            to_u32(clip.height_px),
        );
        renderer.paint(render_pass, self.visibility);
    }
}
