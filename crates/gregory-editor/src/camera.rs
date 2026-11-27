//! 原点を中心に回るカメラ（JS 試作と同じ角度・距離・焦点距離）。座標系は右手系の Y-up。

use gregory_core::{Vec3, cross, dot, mul, norm};

const DEFAULT_YAW: f64 = 0.75;
const DEFAULT_PITCH: f64 = 0.45;
const DEFAULT_DISTANCE: f64 = 8.6;
/// 真上・真下で上向きベクトルが定まらなくなる手前で止める。
pub const PITCH_LIMIT: f64 = 1.45;
pub const MIN_DISTANCE: f64 = 3.0;
pub const MAX_DISTANCE: f64 = 16.0;
/// ドラッグ 1 ポイントあたりの回転角（ラジアン）。
const ORBIT_RADIANS_PER_POINT: f64 = 0.008;
/// 焦点距離を、ビューの短い辺に対する比で表す（JS 試作の `min(W, H) * 1.55`）。
const FOCAL_SCALE: f64 = 1.55;
const NEAR: f64 = 0.1;
const FAR: f64 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    yaw: f64,
    pitch: f64,
    distance: f64,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: DEFAULT_YAW,
            pitch: DEFAULT_PITCH,
            distance: DEFAULT_DISTANCE,
        }
    }
}

/// カメラの位置と、右・上・視線の向きの単位ベクトル。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFrame {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

impl OrbitCamera {
    pub fn yaw(&self) -> f64 {
        self.yaw
    }

    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    pub fn distance(&self) -> f64 {
        self.distance
    }

    /// 画面上のドラッグ量（ポイント）で回す。有限でない量は無視する。
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        let (dx, dy) = (f64::from(dx), f64::from(dy));
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.yaw = (self.yaw - dx * ORBIT_RADIANS_PER_POINT).rem_euclid(std::f64::consts::TAU);
        self.pitch = (self.pitch + dy * ORBIT_RADIANS_PER_POINT).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// 距離を `factor` 倍にする（1 未満で近づく）。正の有限値以外は無視する。
    pub fn zoom(&mut self, factor: f64) {
        if factor.is_finite() && factor > 0.0 {
            self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn frame(&self) -> CameraFrame {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        let d = self.distance;
        let eye = [d * cp * cy, d * sp, -d * cp * sy];
        let forward = norm(mul(eye, -1.0));
        let right = norm(cross(forward, [0.0, 1.0, 0.0]));
        let up = cross(right, forward);
        CameraFrame {
            eye,
            right,
            up,
            forward,
        }
    }

    /// ワールド座標からクリップ座標への行列（列優先、深度は 0〜1）。`width`・`height` はビューの大きさ（比だけが効く）。
    pub fn view_projection(&self, width: f32, height: f32) -> [[f32; 4]; 4] {
        view_projection(&self.frame(), width, height)
    }
}

pub fn view_projection(frame: &CameraFrame, width: f32, height: f32) -> [[f32; 4]; 4] {
    let positive = |x: f32| {
        if x.is_finite() && x > 0.0 {
            f64::from(x)
        } else {
            1.0
        }
    };
    let (w, h) = (positive(width), positive(height));
    let focal = 2.0 * FOCAL_SCALE * w.min(h);
    let (sx, sy) = (focal / w, focal / h);
    let CameraFrame {
        eye,
        right: r,
        up: u,
        forward: f,
    } = *frame;
    let a = FAR / (NEAR - FAR);
    let b = FAR * NEAR / (NEAR - FAR);
    let rows = [
        [sx * r[0], sx * r[1], sx * r[2], -sx * dot(r, eye)],
        [sy * u[0], sy * u[1], sy * u[2], -sy * dot(u, eye)],
        [-a * f[0], -a * f[1], -a * f[2], a * dot(f, eye) + b],
        [f[0], f[1], f[2], -dot(f, eye)],
    ];
    std::array::from_fn(|col| std::array::from_fn(|row| rows[row][col] as f32))
}
