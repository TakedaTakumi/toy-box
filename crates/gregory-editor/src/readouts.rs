//! 画面に出す数値と判定の文字列。

use gregory_core::ContinuityCheck;

use crate::scene::Scene;

/// これ未満の法線の角度差（度）を「G1 成立」とする（JS 試作と同じ）。
pub const G1_THRESHOLD_DEG: f64 = 0.5;
/// これ未満の位置の差は「< 1e-9」と表示する（JS 試作と同じ）。
pub const G0_DISPLAY_FLOOR: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum G1Verdict {
    /// 滑らかな共有辺がない。
    NoSharedEdges,
    Satisfied,
    Violated,
}

impl G1Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoSharedEdges => "対象の辺なし",
            Self::Satisfied => "G1 成立",
            Self::Violated => "G1 不成立",
        }
    }
}

/// NaN は、しきい値未満にならないので不成立とする（JS 試作の `c.g1 < 0.5` と同じ）。
pub fn g1_verdict(check: &ContinuityCheck) -> G1Verdict {
    if check.pairs == 0 {
        G1Verdict::NoSharedEdges
    } else if check.g1 < G1_THRESHOLD_DEG {
        G1Verdict::Satisfied
    } else {
        G1Verdict::Violated
    }
}

pub fn format_g0(g0: f64) -> String {
    if g0 < G0_DISPLAY_FLOOR {
        format!("< {G0_DISPLAY_FLOOR:e}")
    } else {
        format!("{g0:.1e}")
    }
}

pub fn format_g1(check: &ContinuityCheck) -> String {
    if check.pairs == 0 {
        G1Verdict::NoSharedEdges.label().to_owned()
    } else {
        format!("{:.3}°", check.g1)
    }
}

/// 頂点数・面数などの数。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneStats {
    pub vertices: usize,
    pub faces: usize,
    pub edges: usize,
    pub triangles: usize,
    pub patches: usize,
    pub quads: usize,
    pub tris: usize,
    pub smooth_edges: usize,
    pub creased_edges: usize,
}

impl SceneStats {
    pub fn of(scene: &Scene) -> Self {
        let model = &scene.model;
        Self {
            vertices: scene.cage.v.len(),
            faces: scene.cage.f.len(),
            edges: model.edges.len(),
            triangles: model.mesh.tris.len(),
            patches: model.patches.len(),
            quads: model.count.q,
            tris: model.count.t,
            smooth_edges: model.check.pairs,
            creased_edges: model.check.creased,
        }
    }
}

/// 使用中の描画バックエンドの名前。wgpu の GL バックエンドは、web では WebGL2 にあたる。
pub fn backend_label(backend: wgpu::Backend, web: bool) -> &'static str {
    match backend {
        wgpu::Backend::BrowserWebGpu => "WebGPU",
        wgpu::Backend::Gl if web => "WebGL2",
        wgpu::Backend::Gl => "OpenGL",
        wgpu::Backend::Vulkan => "Vulkan",
        wgpu::Backend::Metal => "Metal",
        wgpu::Backend::Dx12 => "DirectX 12",
        wgpu::Backend::Noop => "なし",
    }
}

/// フレーム時間の指数移動平均。値が跳ねても読めるようにならす。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTimer {
    average_ms: Option<f32>,
}

impl FrameTimer {
    const SMOOTHING: f32 = 0.1;

    /// 1フレームの所要時間（秒）を足す。負・NaN・無限大は無視する。
    pub fn record(&mut self, seconds: Option<f32>) {
        let Some(ms) = seconds
            .map(|s| s * 1000.0)
            .filter(|ms| ms.is_finite() && *ms >= 0.0)
        else {
            return;
        };
        self.average_ms = Some(match self.average_ms {
            Some(avg) => avg + (ms - avg) * Self::SMOOTHING,
            None => ms,
        });
    }

    pub fn average_ms(&self) -> Option<f32> {
        self.average_ms
    }
}
