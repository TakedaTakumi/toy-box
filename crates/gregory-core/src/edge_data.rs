//! 辺をまたぐ接線場と、そこから求める内部制御点の元（JS の `edgeData`）。

use crate::bezier::bezier3d;
use crate::cage::Cage;
use crate::curves::EdgeCurves;
use crate::model::Mode;
use crate::patch::EdgeData;
use crate::topology::FaceCorner;
use crate::vec3::{Vec3, add, cross, dot, mul, sub};

/// 辺の上の位置 `t` で、辺をまたぐ向きの接線を返す場。
trait CrossTangentField {
    fn at(&self, t: f64) -> Vec3;
}

/// 両側の面で共有でき、辺の接線と頂点の法線から決まる G1 用の場。
struct G1Field {
    e: [Vec3; 4],
    c0: Vec3,
    c1: Vec3,
    a0: f64,
    b0: f64,
    a1: f64,
    b1: f64,
}

impl G1Field {
    fn new(e: [Vec3; 4], nx: Vec3, ny: Vec3, ax: Vec3, ay: Vec3) -> Self {
        let tx = mul(sub(e[1], e[0]), 3.0);
        let ty = mul(sub(e[3], e[2]), 3.0);
        let c0 = cross(nx, tx);
        let c1 = cross(ny, ty);
        let inv = |x: f64| if x > 1e-18 { 1.0 / x } else { 0.0 };
        Self {
            e,
            c0,
            c1,
            a0: dot(ax, c0) * inv(dot(c0, c0)),
            b0: dot(ax, tx) * inv(dot(tx, tx)),
            a1: dot(ay, c1) * inv(dot(c1, c1)),
            b1: dot(ay, ty) * inv(dot(ty, ty)),
        }
    }
}

impl CrossTangentField for G1Field {
    fn at(&self, t: f64) -> Vec3 {
        let s = 1.0 - t;
        let c = add(mul(self.c0, s), mul(self.c1, t));
        let tangent = bezier3d(&self.e, t);
        add(mul(c, s * self.a0 + t * self.a1), mul(tangent, s * self.b0 + t * self.b1))
    }
}

/// 両端の隣の辺の接線を、線形に補間するだけの場。
struct LinearField {
    ax: Vec3,
    ay: Vec3,
}

impl CrossTangentField for LinearField {
    fn at(&self, t: f64) -> Vec3 {
        add(mul(self.ax, 1.0 - t), mul(self.ay, t))
    }
}

/// 場を 1/3 と 2/3 で通る3次式に当てはめ、その係数を返す。
fn fit_cubic(field: &impl CrossTangentField) -> [Vec3; 4] {
    let c0 = field.at(0.0);
    let c3 = field.at(1.0);
    let d1 = field.at(1.0 / 3.0);
    let d2 = field.at(2.0 / 3.0);
    let a = sub(mul(d1, 27.0), add(mul(c0, 8.0), c3));
    let b = sub(mul(d2, 27.0), add(c0, mul(c3, 8.0)));
    let c1 = mul(sub(mul(a, 2.0), b), 1.0 / 18.0);
    let c2 = mul(sub(mul(b, 2.0), a), 1.0 / 18.0);
    [c0, c1, c2, c3]
}

/// 面の角 `at` から始まる辺の、曲線と接線場の係数。
pub(crate) fn edge_data(
    cage: Cage<'_>,
    corner: &[Vec<Vec3>],
    curves: &mut EdgeCurves<'_>,
    mode: Mode,
    at: FaceCorner,
) -> EdgeData {
    let face = &cage.f[at.fi];
    let m = face.len();
    let k = at.k;
    let nx = corner[at.fi][k];
    let ny = corner[at.fi][(k + 1) % m];
    let (x, y, w, z) = (face[k], face[(k + 1) % m], face[(k + m - 1) % m], face[(k + 2) % m]);
    let e = curves.curve(x, y);
    let ax = mul(sub(curves.curve(x, w)[1], cage.v[x]), 3.0);
    let ay = mul(sub(curves.curve(y, z)[1], cage.v[y]), 3.0);
    let c = match mode {
        Mode::G1 => fit_cubic(&G1Field::new(e, nx, ny, ax, ay)),
        Mode::Linear => fit_cubic(&LinearField { ax, ay }),
    };
    EdgeData { e, c }
}
