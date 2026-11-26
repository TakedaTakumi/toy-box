//! 辺の3次ベジェ曲線と、その両端の接線。

use std::collections::{HashMap, HashSet};

use crate::cage::Cage;
use crate::topology::{EdgeKey, EdgeMap, FaceCorner};
use crate::vec3::{Vec3, add, cross, dot, len, mul, norm, sub};

/// 有向辺ごとの曲線を、逆向きの辺と共有しながら作る。
pub(crate) struct EdgeCurves<'a> {
    cage: Cage<'a>,
    edges: &'a EdgeMap,
    corner: &'a [Vec<Vec3>],
    creases: &'a HashSet<EdgeKey>,
    tau: f64,
    cache: HashMap<(usize, usize), [Vec3; 4]>,
}

impl<'a> EdgeCurves<'a> {
    pub fn new(
        cage: Cage<'a>,
        edges: &'a EdgeMap,
        corner: &'a [Vec<Vec3>],
        creases: &'a HashSet<EdgeKey>,
        tau: f64,
    ) -> Self {
        Self {
            cage,
            edges,
            corner,
            creases,
            tau,
            cache: HashMap::new(),
        }
    }

    /// `x` から `y` への曲線。逆向きの曲線も同時に記録する（`x == y` では、2回目以降は逆向きが返る）。
    pub fn curve(&mut self, x: usize, y: usize) -> [Vec3; 4] {
        if let Some(&e) = self.cache.get(&(x, y)) {
            return e;
        }
        let px = self.cage.v[x];
        let py = self.cage.v[y];
        let e = [
            px,
            add(px, mul(self.tangent(x, y), 1.0 / 3.0)),
            add(py, mul(self.tangent(y, x), 1.0 / 3.0)),
            py,
        ];
        self.cache.insert((x, y), e);
        self.cache.insert((y, x), [e[3], e[2], e[1], e[0]]);
        e
    }

    /// 頂点 `x` での、`y` へ向かう接線（長さは辺長 × tau）。
    fn tangent(&self, x: usize, y: usize) -> Vec3 {
        let d = sub(self.cage.v[y], self.cage.v[x]);
        let l = len(d);
        // 面の辺にしか呼ばれないため、辺の表には必ずある
        let Some(entry) = self.edges.get(x, y) else {
            return [f64::NAN; 3];
        };
        let fs = &entry.faces;
        let n1 = self.corner_at(fs[0], x);
        if self.creases.contains(&EdgeKey::new(x, y)) && fs.len() == 2 {
            let n2 = self.corner_at(fs[1], x);
            let line = cross(n1, n2);
            if len(line) > 1e-3 {
                let dir = norm(line);
                return mul(
                    if dot(dir, d) < 0.0 {
                        mul(dir, -1.0)
                    } else {
                        dir
                    },
                    l * self.tau,
                );
            }
        }
        let pr = sub(d, mul(n1, dot(d, n1)));
        let pl = len(pr);
        if pl < 1e-9 {
            mul(d, self.tau)
        } else {
            mul(pr, (l * self.tau) / pl)
        }
    }

    /// 面の中で最初に現れる頂点 `v` の角の法線（JS の `indexOf` と同じく、重複した頂点では先頭を使う）。
    fn corner_at(&self, at: FaceCorner, v: usize) -> Vec3 {
        let k = self.cage.f[at.fi]
            .iter()
            .position(|&x| x == v)
            .unwrap_or(at.k);
        self.corner[at.fi][k]
    }
}
