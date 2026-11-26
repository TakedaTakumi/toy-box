//! 面の押し出し。

use std::collections::{HashMap, HashSet};

use super::compact::compact_parts;
use super::error::MeshError;
use super::face_normal::face_normal;
use super::model::{CageModel, vertex_at};
use super::ordered::{OrderedMap, OrderedSet};
use crate::js_math::{js_max, js_min};
use crate::topology::EdgeMap;
use crate::union_find::UnionFind;
use crate::vec3::{Vec3, add, mul, norm};

const MIN_EXTRUDE: f64 = 0.02;
const MAX_EXTRUDE: f64 = 100.0;

/// `faces` は押し出した面の番号（重複と範囲外を除いた、指定の順）。
#[derive(Clone, Debug, PartialEq)]
pub struct Extruded {
    pub model: CageModel,
    pub faces: Vec<usize>,
}

/// 選んだ面を法線の向きに押し出し、境界に側面を張る（JS の `extrudeFaces`）。
pub fn extrude_faces(
    model: &CageModel,
    face_idxs: &[usize],
    raw_dist: f64,
) -> Result<Extruded, MeshError> {
    let dist = clamp_distance(raw_dist);
    let sel: OrderedSet<usize> = face_idxs
        .iter()
        .copied()
        .filter(|&i| i < model.f.len())
        .collect();
    if sel.is_empty() {
        return Ok(Extruded {
            model: model.clone(),
            faces: Vec::new(),
        });
    }
    let (v, f) = (&model.v, &model.f);
    let fans = fan_keys(f, &sel);

    // 扇ごとに、属する選択面の法線を足し合わせる。扇の番号が、複製した頂点の番号の順になる
    let mut acc: OrderedMap<(usize, usize), Vec3> = OrderedMap::default();
    let mut slots: Vec<Vec<usize>> = Vec::with_capacity(sel.len());
    for (s, &fi) in sel.iter().enumerate() {
        let n = face_normal(v, &f[fi])?;
        let corner_slots = fans[s]
            .iter()
            .map(|&fan| {
                let (slot, sum) = acc.slot_mut(fan, || [0.0; 3]);
                *sum = add(*sum, n);
                slot
            })
            .collect();
        slots.push(corner_slots);
    }

    let base = v.len();
    let mut v2 = v.clone();
    for &((vertex, _), n) in acc.iter() {
        v2.push(add(vertex_at(v, vertex)?, mul(norm(n), dist)));
    }
    let dup = |s: usize, k: usize| base + slots[s][k];

    let mut f2: Vec<Vec<usize>> = f
        .iter()
        .enumerate()
        .map(|(fi, face)| match sel.position(&fi) {
            Some(s) => (0..face.len()).map(|k| dup(s, k)).collect(),
            None => face.clone(),
        })
        .collect();
    let directed: HashSet<(usize, usize)> =
        sel.iter().flat_map(|&fi| directed_edges(&f[fi])).collect();
    for (s, &fi) in sel.iter().enumerate() {
        let face = &f[fi];
        let m = face.len();
        for (i, &a) in face.iter().enumerate() {
            let j = (i + 1) % m;
            let b = face[j];
            if !directed.contains(&(b, a)) {
                f2.push(vec![a, b, dup(s, j), dup(s, i)]);
            }
        }
    }

    let out = compact_parts(&v2, &f2, &model.creases)?.model;
    Ok(Extruded {
        model: out,
        faces: sel.into_vec(),
    })
}

/// 頂点が重なると面が潰れて曲面が作れず、巨大だと座標が溢れるため、押し出し量を範囲に収める。
fn clamp_distance(raw: f64) -> f64 {
    let sign = if raw < 0.0 { -1.0 } else { 1.0 };
    let mag = if raw.is_finite() {
        js_min(MAX_EXTRUDE, js_max(MIN_EXTRUDE, raw.abs()))
    } else {
        MIN_EXTRUDE
    };
    sign * mag
}

fn directed_edges(face: &[usize]) -> impl Iterator<Item = (usize, usize)> + '_ {
    let m = face.len();
    face.iter()
        .enumerate()
        .map(move |(i, &a)| (a, face[(i + 1) % m]))
}

/// 選択面の角ごとの扇の名前 `(頂点, 代表)`。`fans[s][k]` は `sel` の `s` 番目の面の角 `k`。
///
/// 頂点ごとに、辺でつながった選択面の角を1つの扇にまとめる。頂点だけを共有する面どうしは、別の扇になる。
fn fan_keys(f: &[Vec<usize>], sel: &OrderedSet<usize>) -> Vec<Vec<(usize, usize)>> {
    let mut node: HashMap<(usize, usize), usize> = HashMap::new();
    let corner_nodes: Vec<Vec<usize>> = sel
        .iter()
        .map(|&fi| {
            f[fi]
                .iter()
                .map(|&v| {
                    let next = node.len();
                    *node.entry((fi, v)).or_insert(next)
                })
                .collect()
        })
        .collect();
    let mut uf = UnionFind::new(node.len());
    for e in EdgeMap::build(f).entries() {
        let fs: Vec<usize> = e
            .faces
            .iter()
            .map(|c| c.fi)
            .filter(|fi| sel.contains(fi))
            .collect();
        let Some((&first, rest)) = fs.split_first() else {
            continue;
        };
        for &other in rest {
            for v in [e.a, e.b] {
                if let (Some(&i), Some(&j)) = (node.get(&(other, v)), node.get(&(first, v))) {
                    uf.union(i, j);
                }
            }
        }
    }
    sel.iter()
        .zip(&corner_nodes)
        .map(|(&fi, nodes)| {
            f[fi]
                .iter()
                .zip(nodes)
                .map(|(&v, &n)| (v, uf.find(n)))
                .collect()
        })
        .collect()
}
