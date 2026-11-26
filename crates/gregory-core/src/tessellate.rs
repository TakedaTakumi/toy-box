//! パッチを、描画用の細かい三角形に分割する。

use crate::patch::{Patch, PatchKind};
use crate::vec3::Vec3;

/// `tris` は `pos` の番号の3つ組、`tri_face` は三角形ごとの元の面の番号。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriMesh {
    pub pos: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
    pub tris: Vec<[usize; 3]>,
    pub tri_face: Vec<usize>,
}

impl TriMesh {
    fn push_point(&mut self, patch: &Patch, a: f64, b: f64) {
        self.pos.push(patch.at(a, b));
        self.nrm.push(patch.normal(a, b));
    }
}

pub(crate) fn tessellate(patches: &[Patch], n: usize) -> TriMesh {
    let mut mesh = TriMesh::default();
    // n = 0 では JS と同じく 0/0 の NaN で評価する
    let nf = n as f64;
    for (pi, patch) in patches.iter().enumerate() {
        let base = mesh.pos.len();
        let t0 = mesh.tris.len();
        match patch.kind() {
            PatchKind::Quad => {
                for j in 0..=n {
                    for i in 0..=n {
                        mesh.push_point(patch, i as f64 / nf, j as f64 / nf);
                    }
                }
                for j in 0..n {
                    for i in 0..n {
                        let a = base + j * (n + 1) + i;
                        mesh.tris.push([a, a + 1, a + n + 1]);
                        mesh.tris.push([a + 1, a + n + 2, a + n + 1]);
                    }
                }
            }
            PatchKind::Tri => {
                let mut row_start = Vec::with_capacity(n + 1);
                let mut cnt = 0;
                for t in 0..=n {
                    row_start.push(cnt);
                    for s in 0..=(n - t) {
                        mesh.push_point(patch, s as f64 / nf, t as f64 / nf);
                        cnt += 1;
                    }
                }
                for t in 0..n {
                    for s in 0..(n - t) {
                        let a = base + row_start[t] + s;
                        let b = a + 1;
                        let c = base + row_start[t + 1] + s;
                        mesh.tris.push([a, b, c]);
                        if s < n - t - 1 {
                            mesh.tris.push([b, base + row_start[t + 1] + s + 1, c]);
                        }
                    }
                }
            }
        }
        let added = mesh.tris.len() - t0;
        mesh.tri_face.extend(std::iter::repeat_n(pi, added));
    }
    mesh
}
