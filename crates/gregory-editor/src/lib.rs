//! グレゴリー曲面モデラの表示アプリ。gregory-core の曲面とケージを egui + wgpu で描く。

mod app;
pub mod camera;
pub mod editing;
mod fonts;
pub mod memo;
pub mod mesh_buffers;
pub mod palette;
pub mod panels;
pub mod readouts;
pub mod scene;
pub mod settings;
pub mod shortcuts;
mod viewport;

pub use app::GregoryEditorApp;
pub use viewport::DEPTH_BUFFER_BITS;

/// ウィンドウとページのタイトル。
pub const APP_TITLE: &str = "グレゴリー曲面モデラ（Rust 版）";
