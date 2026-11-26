//! 頂点と面の参照と、面の角の幾何。

use crate::topology::FaceCorner;
use crate::vec3::{Vec3, cross, dot, len, mul, norm, sub};

#[derive(Clone, Copy)]
pub(crate) struct Cage<'a> {
    pub v: &'a [Vec3],
    pub f: &'a [Vec<usize>],
}

impl<'a> Cage<'a> {
    pub fn new(v: &'a [Vec3], f: &'a [Vec<usize>]) -> Self {
        Self { v, f }
    }

    pub fn vertex(&self, c: FaceCorner) -> usize {
        self.f[c.fi][c.k]
    }

    pub fn next(&self, c: FaceCorner) -> usize {
        let face = &self.f[c.fi];
        face[(c.k + 1) % face.len()]
    }

    pub fn prev(&self, c: FaceCorner) -> usize {
        let face = &self.f[c.fi];
        let m = face.len();
        face[(c.k + m - 1) % m]
    }

    /// 角から次の頂点へのベクトルと、前の頂点へのベクトル。
    pub fn corner_edges(&self, c: FaceCorner) -> (Vec3, Vec3) {
        let p = self.v[self.vertex(c)];
        (sub(self.v[self.next(c)], p), sub(self.v[self.prev(c)], p))
    }

    /// 角の内角で重み付けした面法線（JS の `cornerWeighted`）。
    pub fn corner_weighted(&self, c: FaceCorner) -> Vec3 {
        let (ea, eb) = self.corner_edges(c);
        let theta = len(cross(ea, eb)).atan2(dot(ea, eb));
        mul(norm(cross(ea, eb)), theta)
    }

    /// 角での面法線（JS の `flat`）。
    pub fn flat(&self, c: FaceCorner) -> Vec3 {
        let (e1, e2) = self.corner_edges(c);
        norm(cross(e1, e2))
    }
}
