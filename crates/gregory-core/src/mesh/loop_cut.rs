//! ループ分割。

use std::collections::HashMap;

use super::error::MeshError;
use super::model::{CageModel, vertex_at};
use super::ordered::OrderedSet;
use crate::topology::{EdgeKey, EdgeMap};
use crate::vec3::{Vec3, add, mul};

/// `edges` は分割で新しくできた辺（初出の順、重複なし）。
#[derive(Clone, Debug, PartialEq)]
pub struct LoopCut {
    pub model: CageModel,
    pub edges: Vec<EdgeKey>,
}

/// 選んだ辺と平行な辺を四角形の列にそって集め、すべてを中点で分割する（JS の `loopCut`）。
///
/// `mid_of` に辺の中点の位置があれば、端点の平均の代わりにそれを使う。
pub fn loop_cut(
    model: &CageModel,
    edge_keys: &[EdgeKey],
    mid_of: Option<&HashMap<EdgeKey, Vec3>>,
) -> Result<LoopCut, MeshError> {
    let edges = EdgeMap::build(&model.f);
    let ring = collect_ring(&model.f, &edges, edge_keys);
    if ring.is_empty() {
        return Ok(LoopCut {
            model: model.clone(),
            edges: Vec::new(),
        });
    }

    let mut v2 = model.v.clone();
    for &k in ring.iter() {
        let p = match mid_of.and_then(|m| m.get(&k)) {
            Some(&p) => p,
            None => mul(
                add(vertex_at(&model.v, k.a())?, vertex_at(&model.v, k.b())?),
                0.5,
            ),
        };
        v2.push(p);
    }
    let mut splitter = Splitter {
        ring: &ring,
        mid_base: model.v.len(),
        v2,
        f2: Vec::new(),
        new_edges: OrderedSet::default(),
    };
    for (fi, face) in model.f.iter().enumerate() {
        splitter.split(fi, face)?;
    }

    let mut creases = model.creases.clone();
    for &k in ring.iter() {
        if !creases.remove(&k) {
            continue;
        }
        let Some(m) = splitter.mid(k) else { continue };
        creases.insert(EdgeKey::new(k.a(), m));
        creases.insert(EdgeKey::new(m, k.b()));
    }
    Ok(LoopCut {
        model: CageModel {
            v: splitter.v2,
            f: splitter.f2,
            creases,
        },
        edges: splitter.new_edges.into_vec(),
    })
}

/// 指定の辺から、四角形の向かいの辺をたどって集める。後に積んだ辺から先にたどる。
fn collect_ring(f: &[Vec<usize>], edges: &EdgeMap, edge_keys: &[EdgeKey]) -> OrderedSet<EdgeKey> {
    let mut ring = OrderedSet::default();
    let mut stack: Vec<EdgeKey> = edge_keys
        .iter()
        .copied()
        .filter(|&k| edges.contains(k))
        .collect();
    while let Some(k) = stack.pop() {
        if !ring.insert(k) {
            continue;
        }
        let Some(entry) = edges.lookup(k) else {
            continue;
        };
        for c in &entry.faces {
            let face = &f[c.fi];
            if face.len() != 4 {
                continue;
            }
            for i in 0..4 {
                if EdgeKey::new(face[i], face[(i + 1) % 4]) == k {
                    stack.push(EdgeKey::new(face[(i + 2) % 4], face[(i + 3) % 4]));
                }
            }
        }
    }
    ring
}

/// 面を1枚ずつ分割し、新しい頂点・面・辺を積む。中点の番号は、`ring` の順に `mid_base` から振ってある。
struct Splitter<'a> {
    ring: &'a OrderedSet<EdgeKey>,
    mid_base: usize,
    v2: Vec<Vec3>,
    f2: Vec<Vec<usize>>,
    new_edges: OrderedSet<EdgeKey>,
}

impl Splitter<'_> {
    fn mid(&self, k: EdgeKey) -> Option<usize> {
        self.ring.position(&k).map(|p| self.mid_base + p)
    }

    fn link(&mut self, a: usize, b: usize) {
        self.new_edges.insert(EdgeKey::new(a, b));
    }

    fn split(&mut self, fi: usize, f: &[usize]) -> Result<(), MeshError> {
        let n = f.len();
        let marked: Vec<usize> = (0..n)
            .filter(|&i| self.ring.contains(&EdgeKey::new(f[i], f[(i + 1) % n])))
            .collect();
        if marked.is_empty() {
            self.f2.push(f.to_vec());
            return Ok(());
        }
        let undefined = MeshError::UndefinedCorner { face: fi };
        let m = |i: usize| {
            self.mid(EdgeKey::new(f[i % n], f[(i + 1) % n]))
                .ok_or(undefined.clone())
        };
        // 4辺以外の面は、JS と同じく先頭の3角だけを三角形として扱う
        let tri = |s: usize, i: usize| f.get((s + i) % 3).copied().ok_or(undefined.clone());

        if n == 4 && marked.len() == 2 {
            let s = match marked[..] {
                [0, 2] => 0,
                [1, 3] => 1,
                _ => marked[0],
            };
            let v = |i: usize| f[(s + i) % 4];
            let (ma, mb) = (m(s)?, m(s + 2)?);
            self.f2.push(vec![v(0), ma, mb, v(3)]);
            self.f2.push(vec![ma, v(1), v(2), mb]);
            self.link(ma, mb);
        } else if n == 4 {
            let mm = [m(0)?, m(1)?, m(2)?, m(3)?];
            let p = |i: usize| vertex_at(&self.v2, mm[i]);
            let center = mul(add(add(p(0)?, p(1)?), add(p(2)?, p(3)?)), 0.25);
            let c = self.v2.len();
            self.v2.push(center);
            self.f2.push(vec![f[0], mm[0], c, mm[3]]);
            self.f2.push(vec![mm[0], f[1], mm[1], c]);
            self.f2.push(vec![c, mm[1], f[2], mm[2]]);
            self.f2.push(vec![mm[3], c, mm[2], f[3]]);
            for x in mm {
                self.link(x, c);
            }
        } else if marked.len() == 1 {
            let s = marked[0];
            let ma = m(s)?;
            let (v0, v1, v2) = (tri(s, 0)?, tri(s, 1)?, tri(s, 2)?);
            self.f2.push(vec![v0, ma, v2]);
            self.f2.push(vec![ma, v1, v2]);
            self.link(ma, v2);
        } else if marked.len() == 2 {
            let has = |i: usize| marked.contains(&i);
            let s = if has(0) && has(1) {
                0
            } else if has(1) && has(2) {
                1
            } else {
                2
            };
            let (ma, mb) = (m(s)?, m(s + 1)?);
            let (v0, v1, v2) = (tri(s, 0)?, tri(s, 1)?, tri(s, 2)?);
            self.f2.push(vec![ma, v1, mb]);
            self.f2.push(vec![v0, ma, mb, v2]);
            self.link(ma, mb);
        } else {
            let mm = [m(0)?, m(1)?, m(2)?];
            let (f0, f1, f2) = (tri(0, 0)?, tri(0, 1)?, tri(0, 2)?);
            self.f2.push(vec![f0, mm[0], mm[2]]);
            self.f2.push(vec![mm[0], f1, mm[1]]);
            self.f2.push(vec![mm[2], mm[1], f2]);
            self.f2.push(vec![mm[0], mm[1], mm[2]]);
            self.link(mm[0], mm[1]);
            self.link(mm[1], mm[2]);
            self.link(mm[2], mm[0]);
        }
        Ok(())
    }
}
