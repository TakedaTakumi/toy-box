//! 四角形グレゴリーパッチ（3次の境界曲線と、角ごとに2点の内部制御点）。

use super::{EdgeData, blend};
use crate::bezier::bern3;
use crate::vec3::{Vec3, add, mul};

#[derive(Clone, Debug, PartialEq)]
pub struct QuadPatch {
    ed: [EdgeData; 4],
    q: [[Vec3; 2]; 4],
    b: [[Vec3; 4]; 4],
}

impl QuadPatch {
    pub(crate) fn new(ed: [EdgeData; 4]) -> Self {
        // 内部の4点は評価時に内部制御点で置き換えるため、0 のままでよい
        let mut b = [[[0.0; 3]; 4]; 4];
        for m in 0..4 {
            b[m][0] = ed[0].e[m];
            b[3][m] = ed[1].e[m];
            b[3 - m][3] = ed[2].e[m];
            b[0][3 - m] = ed[3].e[m];
        }
        let q = ed.map(|d| [add(d.e[1], mul(d.c[1], 1.0 / 3.0)), add(d.e[2], mul(d.c[2], 1.0 / 3.0))]);
        Self { ed, q, b }
    }

    pub fn ed(&self) -> &[EdgeData] {
        &self.ed
    }

    pub fn q(&self) -> &[[Vec3; 2]] {
        &self.q
    }

    pub fn at(&self, u: f64, v: f64) -> Vec3 {
        let bu = bern3(u);
        let bv = bern3(v);
        let q = &self.q;
        let p11 = blend(u, q[0][0], v, q[3][1]);
        let p21 = blend(1.0 - u, q[0][1], v, q[1][0]);
        let p22 = blend(1.0 - u, q[2][0], 1.0 - v, q[1][1]);
        let p12 = blend(u, q[2][1], 1.0 - v, q[3][0]);
        let mut r = [0.0; 3];
        for (i, &wu) in bu.iter().enumerate() {
            for (j, &wv) in bv.iter().enumerate() {
                let p = match (i, j) {
                    (1, 1) => p11,
                    (2, 1) => p21,
                    (2, 2) => p22,
                    (1, 2) => p12,
                    _ => self.b[i][j],
                };
                let w = wu * wv;
                r[0] += p[0] * w;
                r[1] += p[1] * w;
                r[2] += p[2] * w;
            }
        }
        r
    }

    pub fn edge_point(&self, k: usize, t: f64, h: f64) -> Option<[Vec3; 2]> {
        let (p, i) = match k {
            0 => ([t, 0.0], [t, h]),
            1 => ([1.0, t], [1.0 - h, t]),
            2 => ([1.0 - t, 1.0], [1.0 - t, 1.0 - h]),
            3 => ([0.0, 1.0 - t], [h, 1.0 - t]),
            _ => return None,
        };
        Some([self.at(p[0], p[1]), self.at(i[0], i[1])])
    }
}
