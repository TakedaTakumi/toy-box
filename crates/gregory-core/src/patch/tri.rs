//! 三角形グレゴリーパッチ（3次の境界曲線を4次に次数上げし、角ごとに2点の内部制御点を持つ）。

use super::{EdgeData, blend};
use crate::vec3::{Vec3, add, mul};

const FACT: [f64; 5] = [1.0, 1.0, 2.0, 6.0, 24.0];

/// 3辺より多い面でも作れ、そのときは先頭の3辺だけで曲面を決める。
#[derive(Clone, Debug, PartialEq)]
pub struct TriPatch {
    ed: Vec<EdgeData>,
    q: Vec<[Vec3; 2]>,
    c: [[Vec3; 5]; 5],
}

impl TriPatch {
    /// 辺が3本未満なら `None`。
    pub(crate) fn new(ed: Vec<EdgeData>) -> Option<Self> {
        if ed.len() < 3 {
            return None;
        }
        let fe: [[Vec3; 5]; 3] = std::array::from_fn(|i| {
            let e = ed[i].e;
            [
                e[0],
                add(mul(e[0], 0.25), mul(e[1], 0.75)),
                add(mul(e[1], 0.5), mul(e[2], 0.5)),
                add(mul(e[2], 0.75), mul(e[3], 0.25)),
                e[3],
            ]
        });
        // 内部の3点は評価時に内部制御点で置き換え、i + j > 4 の欄は使わないため、0 のままでよい
        let mut c = [[[0.0; 3]; 5]; 5];
        for m in 0..=4 {
            c[4 - m][m] = fe[0][m];
            c[0][4 - m] = fe[1][m];
            c[m][0] = fe[2][m];
        }
        let q = ed
            .iter()
            .map(|d| {
                [
                    add(d.e[1], mul(d.c[1], 0.25)),
                    add(d.e[2], mul(d.c[2], 0.25)),
                ]
            })
            .collect();
        Some(Self { ed, q, c })
    }

    pub fn ed(&self) -> &[EdgeData] {
        &self.ed
    }

    pub fn q(&self) -> &[[Vec3; 2]] {
        &self.q
    }

    pub fn at(&self, s: f64, t: f64) -> Vec3 {
        let b0 = 1.0 - s - t;
        let b1 = s;
        let b2 = t;
        let q = &self.q;
        let p0 = blend(b1, q[0][0], b2, q[2][1]);
        let p1 = blend(b0, q[0][1], b2, q[1][0]);
        let p2 = blend(b1, q[1][1], b0, q[2][0]);
        let mut r = [0.0; 3];
        for (i, &fi) in FACT.iter().enumerate().take(5) {
            for (j, &fj) in FACT.iter().enumerate().take(5 - i) {
                let k = 4 - i - j;
                let p = match (i, j) {
                    (2, 1) => p0,
                    (1, 2) => p1,
                    (1, 1) => p2,
                    _ => self.c[i][j],
                };
                let w = (FACT[4] / (fi * fj * FACT[k]))
                    * b0.powf(i as f64)
                    * b1.powf(j as f64)
                    * b2.powf(k as f64);
                r[0] += p[0] * w;
                r[1] += p[1] * w;
                r[2] += p[2] * w;
            }
        }
        r
    }

    pub fn edge_point(&self, k: usize, t: f64, h: f64) -> Option<[Vec3; 2]> {
        let (p, i) = match k {
            0 => ([t, 0.0], [t * (1.0 - h), h]),
            1 => ([1.0 - t, t], [(1.0 - t) * (1.0 - h), t * (1.0 - h)]),
            2 => ([0.0, 1.0 - t], [h, (1.0 - t) * (1.0 - h)]),
            _ => return None,
        };
        Some([self.at(p[0], p[1]), self.at(i[0], i[1])])
    }
}
