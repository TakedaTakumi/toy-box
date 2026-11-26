//! 頂点の法線と、面の角ごとの法線（折れ目で区切った滑らかな群ごとの法線）。

use std::collections::HashSet;

use crate::cage::Cage;
use crate::topology::{EdgeKey, FaceCorner};
use crate::union_find::UnionFind;
use crate::vec3::{Vec3, add, cross, dot, len, mul, norm, sub};

/// 頂点の全周の法線。可視判定の目安にだけ使う。
pub(crate) fn vertex_normals(cage: Cage<'_>) -> Vec<Vec3> {
    let mut acc = vec![[0.0; 3]; cage.v.len()];
    for (fi, face) in cage.f.iter().enumerate() {
        for (k, &v) in face.iter().enumerate() {
            acc[v] = add(acc[v], cage.corner_weighted(FaceCorner { fi, k }));
        }
    }
    acc.into_iter().map(norm).collect()
}

/// `corner[fi][k]` が面 `fi` の角 `k` の法線になる表を返す。
pub(crate) fn corner_normals(cage: Cage<'_>, creases: &HashSet<EdgeKey>) -> Vec<Vec<Vec3>> {
    let mut corner: Vec<Vec<Vec3>> = cage
        .f
        .iter()
        .map(|face| vec![[0.0; 3]; face.len()])
        .collect();
    let mut incident: Vec<Vec<FaceCorner>> = vec![Vec::new(); cage.v.len()];
    for (fi, face) in cage.f.iter().enumerate() {
        for (k, &v) in face.iter().enumerate() {
            incident[v].push(FaceCorner { fi, k });
        }
    }
    for (v, list) in incident.iter().enumerate() {
        let roots = smooth_groups(cage, v, list, creases);
        let mut sums: Vec<(usize, Vec3)> = Vec::new();
        for (&c, &r) in list.iter().zip(&roots) {
            let cw = cage.corner_weighted(c);
            match sums.iter_mut().find(|(root, _)| *root == r) {
                Some((_, sum)) => *sum = add(*sum, cw),
                None => sums.push((r, add([0.0; 3], cw))),
            }
        }
        for (r, sum) in sums {
            let members: Vec<FaceCorner> = list
                .iter()
                .zip(&roots)
                .filter(|&(_, &root)| root == r)
                .map(|(&c, _)| c)
                .collect();
            let n = choose_normal(cage, sum, &members);
            for c in &members {
                corner[c.fi][c.k] = n;
            }
        }
    }
    corner
}

/// 頂点 `v` を囲む角を、折れ目でない辺でつながるものどうしにまとめ、各角の群の代表を返す。
fn smooth_groups(
    cage: Cage<'_>,
    v: usize,
    list: &[FaceCorner],
    creases: &HashSet<EdgeKey>,
) -> Vec<usize> {
    let mut uf = UnionFind::new(list.len());
    let nbrs: Vec<[usize; 2]> = list.iter().map(|&c| [cage.next(c), cage.prev(c)]).collect();
    for i in 0..list.len() {
        for j in (i + 1)..list.len() {
            // 折れ目でも、両側の面がほぼ同じ平面なら、交線の向きが不安定なので折れ目として扱わない
            let flat_i = norm(cage.corner_weighted(list[i]));
            let flat_j = norm(cage.corner_weighted(list[j]));
            let coplanar = dot(flat_i, flat_j) > 0.0 && len(cross(flat_i, flat_j)) < 0.05;
            let shared = nbrs[i].iter().any(|&w| {
                nbrs[j].contains(&w) && (!creases.contains(&EdgeKey::new(v, w)) || coplanar)
            });
            if shared {
                uf.union(i, j);
            }
        }
    }
    (0..list.len()).map(|i| uf.find(i)).collect()
}

/// 接平面へ射影したとき、どの面の角も裏返らない法線を選ぶ（角度重み、重みなし、各面の法線の順）。
fn choose_normal(cage: Cage<'_>, sum: Vec3, members: &[FaceCorner]) -> Vec3 {
    let unweighted = norm(members.iter().fold([0.0; 3], |a, &c| add(a, cage.flat(c))));
    let mut candidates = vec![norm(sum), unweighted];
    candidates.extend(members.iter().map(|&c| cage.flat(c)));
    candidates
        .iter()
        .copied()
        .find(|&n| len(n) > 0.5 && !folded(cage, n, members))
        .unwrap_or(candidates[0])
}

fn folded(cage: Cage<'_>, n: Vec3, members: &[FaceCorner]) -> bool {
    members.iter().any(|&c| {
        let (e1, e2) = cage.corner_edges(c);
        let p1 = sub(e1, mul(n, dot(e1, n)));
        let p2 = sub(e2, mul(n, dot(e2, n)));
        dot(cross(p1, p2), n) <= 1e-6 * len(p1) * len(p2)
    })
}
