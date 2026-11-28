//! 移動ハンドルの幾何と、ドラッグ量からの移動量の計算。

use gregory_core::{Vec3, add, dot, mul, sub};

use crate::camera::{NEAR, Ray, ScreenView};

/// 画面上の位置（ビューの左上が原点で右・下が正）。
pub type ScreenPos = [f64; 2];

/// 矢印の画面上の長さ。
pub const GIZMO_PX: f64 = 84.0;
/// 矢印をつかめる、軸からの画面上の距離（これ未満）。
pub const AXIS_PICK_RADIUS: f64 = 9.0;
/// 平面ハンドルの四角が、軸の長さに対して占める範囲。
pub const PLANE_INNER: f64 = 0.22;
pub const PLANE_OUTER: f64 = 0.5;
/// 視線と平面がほぼ平行なときは交点を求めない。
const PARALLEL_EPS: f64 = 1e-6;
/// 軸が画面上でほぼ点になるときは、軸方向に動かさない。
const AXIS_DEGENERATE_EPS: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    pub fn unit(self) -> Vec3 {
        match self {
            Self::X => [1.0, 0.0, 0.0],
            Self::Y => [0.0, 1.0, 0.0],
            Self::Z => [0.0, 0.0, 1.0],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

/// 平面ハンドル。名前の2軸に沿って動かす。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Plane {
    XY,
    YZ,
    ZX,
}

impl Plane {
    /// 判定と描画の順（JS 試作と同じ）。
    pub const ALL: [Self; 3] = [Self::XY, Self::YZ, Self::ZX];

    /// 四角を張る2軸と、法線になる第3軸。
    pub fn axes(self) -> [Axis; 3] {
        match self {
            Self::XY => [Axis::X, Axis::Y, Axis::Z],
            Self::YZ => [Axis::Y, Axis::Z, Axis::X],
            Self::ZX => [Axis::Z, Axis::X, Axis::Y],
        }
    }

    pub fn normal(self) -> Axis {
        self.axes()[2]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handle {
    Axis(Axis),
    Plane(Plane),
}

/// 移動の拘束。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Constraint {
    /// 画面に平行に動かす。
    #[default]
    Free,
    Axis(Axis),
    Plane(Plane),
}

impl From<Handle> for Constraint {
    fn from(handle: Handle) -> Self {
        match handle {
            Handle::Axis(a) => Self::Axis(a),
            Handle::Plane(p) => Self::Plane(p),
        }
    }
}

/// 軸の固定（`None` は自由）から、要素のドラッグの拘束を決める。
pub fn constraint_for_lock(lock: Option<Axis>) -> Constraint {
    lock.map_or(Constraint::Free, Constraint::Axis)
}

/// 画面上の移動ハンドル。`tips` と `planes` は `Axis::ALL`・`Plane::ALL` の順。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gizmo {
    pub center: Vec3,
    pub origin: ScreenPos,
    pub tips: [ScreenPos; 3],
    pub planes: [[ScreenPos; 4]; 3],
}

impl Gizmo {
    pub fn tip(&self, axis: Axis) -> ScreenPos {
        self.tips[axis.index()]
    }
}

/// 頂点の位置の平均。番号が範囲外のものは除き、1つもなければ `None`。
pub fn selection_center(positions: &[Vec3], indices: &[usize]) -> Option<Vec3> {
    let picked: Vec<Vec3> = indices
        .iter()
        .filter_map(|&i| positions.get(i).copied())
        .collect();
    if picked.is_empty() {
        return None;
    }
    let sum = picked.iter().fold([0.0; 3], |acc, &p| add(acc, p));
    let center = mul(sum, 1.0 / picked.len() as f64);
    center.iter().all(|c| c.is_finite()).then_some(center)
}

/// 中心に置いたハンドルの画面上の形。中心がカメラに近すぎるか後ろにあるときは `None`。
pub fn gizmo(view: &ScreenView, center: Vec3) -> Option<Gizmo> {
    let z = view.depth(center);
    if z.is_nan() || z < NEAR {
        return None;
    }
    let world_len = GIZMO_PX * z / view.focal();
    let at = |offset: Vec3| {
        view.project(add(center, mul(offset, world_len)))
            .map(|p| p.xy())
    };
    let origin = view.project(center)?.xy();
    let mut tips = [[0.0; 2]; 3];
    for (slot, axis) in tips.iter_mut().zip(Axis::ALL) {
        *slot = at(axis.unit())?;
    }
    let mut planes = [[[0.0; 2]; 4]; 3];
    for (quad, plane) in planes.iter_mut().zip(Plane::ALL) {
        let [a, b, _] = plane.axes().map(Axis::unit);
        let corner = |u: f64, v: f64| at(add(mul(a, u), mul(b, v)));
        *quad = [
            corner(PLANE_INNER, PLANE_INNER)?,
            corner(PLANE_OUTER, PLANE_INNER)?,
            corner(PLANE_OUTER, PLANE_OUTER)?,
            corner(PLANE_INNER, PLANE_OUTER)?,
        ];
    }
    Some(Gizmo {
        center,
        origin,
        tips,
        planes,
    })
}

/// 画面上の位置にあるハンドル。平面の四角を優先し、次に軸から `AXIS_PICK_RADIUS` 未満で最も近い矢印。
pub fn pick_handle(gizmo: &Gizmo, pos: ScreenPos) -> Option<Handle> {
    if !pos.iter().all(|c| c.is_finite()) {
        return None;
    }
    if let Some(plane) = Plane::ALL
        .into_iter()
        .zip(&gizmo.planes)
        .find(|(_, quad)| point_in_quad(quad, pos))
        .map(|(plane, _)| plane)
    {
        return Some(Handle::Plane(plane));
    }
    let mut best = None;
    let mut best_distance = AXIS_PICK_RADIUS;
    for axis in Axis::ALL {
        let d = segment_distance(gizmo.origin, gizmo.tip(axis), pos);
        if d < best_distance {
            best_distance = d;
            best = Some(Handle::Axis(axis));
        }
    }
    best
}

/// 凸な四角の内側（辺の上を含む）か。辺に対する外積の符号がすべて揃えば内側とする（JS 試作と同じ判定）。
pub fn point_in_quad(quad: &[ScreenPos; 4], pos: ScreenPos) -> bool {
    let [x, y] = pos;
    let mut sign = 0.0;
    for i in 0..4 {
        let a = quad[i];
        let b = quad[(i + 1) % 4];
        let cr = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]);
        if cr.is_nan() {
            return false;
        }
        if cr != 0.0 {
            let s = cr.signum();
            if sign != 0.0 && s != sign {
                return false;
            }
            sign = s;
        }
    }
    true
}

/// 点から線分までの画面上の距離。両端が同じ点なら、その点までの距離。
pub fn segment_distance(a: ScreenPos, b: ScreenPos, p: ScreenPos) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let l2 = if l2 == 0.0 { 1.0 } else { l2 };
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
    (p[0] - (a[0] + t * dx)).hypot(p[1] - (a[1] + t * dy))
}

/// 視線と、`point` を通り法線が `normal` の平面の交点。平行に近いときと、カメラの後ろで交わるときは `None`。
pub fn ray_plane(ray: &Ray, normal: Vec3, point: Vec3) -> Option<Vec3> {
    let den = dot(normal, ray.dir);
    if den.is_nan() || den.abs() < PARALLEL_EPS {
        return None;
    }
    let t = dot(normal, sub(point, ray.origin)) / den;
    (t > 0.0)
        .then(|| ray.at(t))
        .filter(|p| p.iter().all(|c| c.is_finite()))
}

/// 移動ハンドルの平面上で、画面上の位置の真下にある点（平面ドラッグの基準）。
pub fn plane_point(view: &ScreenView, plane: Plane, center: Vec3, pos: ScreenPos) -> Option<Vec3> {
    let ray = view.ray(pos[0], pos[1])?;
    ray_plane(&ray, plane.normal().unit(), center)
}

/// ドラッグによる移動量。
///
/// - 自由：画面上の累計の移動量 `total` を、中心の奥行きで換算してカメラの右・上へ動かす
/// - 軸：`total` を、軸の画面上の向きへ射影した分だけ軸に沿って動かす
/// - 平面：いまの位置 `pos` の視線と平面の交点から、押した位置の交点 `plane_origin` を引く
///
/// 求められないとき（交点なし、軸が画面上で点になる、結果が有限でない）は 0 を返す。
pub fn move_delta(
    view: &ScreenView,
    constraint: Constraint,
    center: Vec3,
    total: ScreenPos,
    pos: ScreenPos,
    plane_origin: Option<Vec3>,
) -> Vec3 {
    let [dx, dy] = total;
    let d = match constraint {
        Constraint::Free => {
            let k = view.depth(center) / view.focal();
            let frame = view.frame();
            add(mul(frame.right, dx * k), mul(frame.up, -dy * k))
        }
        Constraint::Axis(axis) => {
            let unit = axis.unit();
            match (view.project(center), view.project(add(center, unit))) {
                (Some(p0), Some(p1)) => {
                    let sv = [p1.x - p0.x, p1.y - p0.y];
                    let l2 = sv[0] * sv[0] + sv[1] * sv[1];
                    let s = if l2 < AXIS_DEGENERATE_EPS {
                        0.0
                    } else {
                        (dx * sv[0] + dy * sv[1]) / l2
                    };
                    mul(unit, s)
                }
                _ => [0.0; 3],
            }
        }
        Constraint::Plane(plane) => match (plane_point(view, plane, center, pos), plane_origin) {
            (Some(hit), Some(origin)) => sub(hit, origin),
            _ => [0.0; 3],
        },
    };
    if d.iter().all(|c| c.is_finite()) {
        d
    } else {
        [0.0; 3]
    }
}
