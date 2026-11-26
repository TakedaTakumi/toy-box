//! グレゴリーパッチ。

mod quad;
mod tri;

pub use quad::QuadPatch;
pub use tri::TriPatch;

use crate::vec3::{Vec3, add, cross, mul, norm, sub};

/// 面の1辺の情報。`e` は辺の曲線、`c` は辺をまたぐ接線場を3次式にした係数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeData {
    pub e: [Vec3; 4],
    pub c: [Vec3; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatchKind {
    Quad,
    Tri,
}

// ほとんどのパッチは四角形なので、Box化しても節約は小さく、四角形ごとのヒープ確保が増えるだけ
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum Patch {
    Quad(QuadPatch),
    Tri(TriPatch),
}

impl Patch {
    pub fn kind(&self) -> PatchKind {
        match self {
            Patch::Quad(_) => PatchKind::Quad,
            Patch::Tri(_) => PatchKind::Tri,
        }
    }

    /// 面の全辺の情報（5辺以上の面でも全辺を持つ）。
    pub fn ed(&self) -> &[EdgeData] {
        match self {
            Patch::Quad(p) => p.ed(),
            Patch::Tri(p) => p.ed(),
        }
    }

    /// 辺ごとの内部制御点。`q()[k][0]` が JS の `q[k][1]`、`q()[k][1]` が `q[k][2]`。
    pub fn q(&self) -> &[[Vec3; 2]] {
        match self {
            Patch::Quad(p) => p.q(),
            Patch::Tri(p) => p.q(),
        }
    }

    /// 曲面上の点。四角形は `(u, v)`、三角形は重心座標の `(s, t)`。
    pub fn at(&self, a: f64, b: f64) -> Vec3 {
        match self {
            Patch::Quad(p) => p.at(a, b),
            Patch::Tri(p) => p.at(a, b),
        }
    }

    pub fn normal(&self, a: f64, b: f64) -> Vec3 {
        normal_at(|u, v| self.at(u, v), a, b, self.kind() == PatchKind::Tri)
    }

    /// 辺 `k` の位置 `t` の点と、そこから内側へ `h` だけ入った点。パッチにない辺の番号では `None`。
    pub fn edge_point(&self, k: usize, t: f64, h: f64) -> Option<[Vec3; 2]> {
        match self {
            Patch::Quad(p) => p.edge_point(k, t, h),
            Patch::Tri(p) => p.edge_point(k, t, h),
        }
    }
}

/// 角で重みが 0/0 になる点は、2つの候補の平均にする。
pub(crate) fn blend(wa: f64, a: Vec3, wb: f64, b: Vec3) -> Vec3 {
    let s = wa + wb;
    if s < 1e-12 {
        return mul(add(a, b), 0.5);
    }
    add(mul(a, wa / s), mul(b, wb / s))
}

/// 角や斜辺で差分が取れるよう、内側へ少し寄せてから差分する。
fn normal_at(at: impl Fn(f64, f64) -> Vec3, a: f64, b: f64, tri: bool) -> Vec3 {
    let e = 2e-3;
    let h = 3e-4;
    let inset = if tri { e / 3.0 } else { e / 2.0 };
    let a2 = a * (1.0 - e) + inset;
    let b2 = b * (1.0 - e) + inset;
    let da = sub(at(a2 + h, b2), at(a2 - h, b2));
    let db = sub(at(a2, b2 + h), at(a2, b2 - h));
    norm(cross(da, db))
}
