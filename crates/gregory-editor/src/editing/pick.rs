//! 画面上の位置から、点・辺・面を選び出す。

use gregory_core::{
    CageModel, Edge, EdgeKey, Model, SelectMode, TriMesh, Vec3, add, bezier3, dot, mul, norm, sub,
};

use super::gizmo::{ScreenPos, segment_distance};
use super::selection::Element;
use crate::camera::{Projected, ScreenView};

/// 点をつかめる画面上の距離（これ未満）。
pub const VERTEX_PICK_RADIUS: f64 = 13.0;
/// 辺をつかめる画面上の距離（これ未満）。
pub const EDGE_PICK_RADIUS: f64 = 10.0;
/// 辺の曲線を折れ線にするときの分割数。
pub const EDGE_SAMPLES: usize = 24;
/// 三角形の頂点の法線と視線の内積の平均がこれを超えたら、裏向きとして面のピックから除く。
pub const BACK_FACE_LIMIT: f64 = 0.06;

/// 法線 `n` の点 `p` が、カメラの側を向いているか。
pub fn facing(view: &ScreenView, p: Vec3, n: Vec3) -> bool {
    dot(n, sub(view.frame().eye, p)) > 0.0
}

/// 頂点の画面上の位置。裏向き（`normals` で判定）と、射影できない頂点は `None`。法線のない頂点も `None`。
pub fn vertex_screen_points(
    view: &ScreenView,
    positions: &[Vec3],
    normals: &[Vec3],
) -> Vec<Option<ScreenPos>> {
    positions
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let n = *normals.get(i)?;
            if !facing(view, p, n) {
                return None;
            }
            view.project(p).map(|s| s.xy())
        })
        .collect()
}

/// 1本の辺の曲線のうち、表を向いた区間ごとの画面上の折れ線。
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeRuns {
    pub key: EdgeKey,
    pub runs: Vec<Vec<ScreenPos>>,
}

/// 辺の曲線を `EDGE_SAMPLES` 分割し、表を向いた点だけをつないだ折れ線にする。向きは両端の頂点の法線を補間して判定する。
pub fn edge_screen_runs(view: &ScreenView, edges: &[Edge], normals: &[Vec3]) -> Vec<EdgeRuns> {
    edges
        .iter()
        .map(|edge| {
            let mut runs: Vec<Vec<ScreenPos>> = Vec::new();
            let mut open = false;
            let ends = (normals.get(edge.a), normals.get(edge.b));
            for i in 0..=EDGE_SAMPLES {
                let t = i as f64 / EDGE_SAMPLES as f64;
                let p = bezier3(&edge.e, t);
                let visible = match ends {
                    (Some(&na), Some(&nb)) => {
                        let n = norm(add(mul(na, 1.0 - t), mul(nb, t)));
                        facing(view, p, n)
                    }
                    _ => false,
                };
                let screen = if visible { view.project(p) } else { None };
                match screen {
                    Some(s) => {
                        if !open {
                            runs.push(Vec::new());
                            open = true;
                        }
                        if let Some(run) = runs.last_mut() {
                            run.push(s.xy());
                        }
                    }
                    None => open = false,
                }
            }
            EdgeRuns {
                key: EdgeKey::new(edge.a, edge.b),
                runs,
            }
        })
        .collect()
}

/// 曲面の細かい三角形の頂点の、画面上の位置と、法線と視線の内積。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceScreen {
    pub points: Vec<Option<Projected>>,
    pub facing_z: Vec<f64>,
}

pub fn surface_screen(view: &ScreenView, mesh: &TriMesh) -> SurfaceScreen {
    let forward = view.frame().forward;
    SurfaceScreen {
        points: mesh.pos.iter().map(|&p| view.project(p)).collect(),
        facing_z: (0..mesh.pos.len())
            .map(|i| mesh.nrm.get(i).map_or(f64::NAN, |&n| dot(n, forward)))
            .collect(),
    }
}

/// `radius` 未満で最も近い点の番号。
pub fn nearest_point(points: &[Option<ScreenPos>], pos: ScreenPos, radius: f64) -> Option<usize> {
    if !is_finite(pos) {
        return None;
    }
    let mut best = None;
    let mut best_distance = radius;
    for (i, p) in points.iter().enumerate() {
        let Some(p) = p else { continue };
        let d = (p[0] - pos[0]).hypot(p[1] - pos[1]);
        if d < best_distance {
            best_distance = d;
            best = Some(i);
        }
    }
    best
}

/// `radius` 未満で最も近い辺。
pub fn nearest_edge(edges: &[EdgeRuns], pos: ScreenPos, radius: f64) -> Option<EdgeKey> {
    if !is_finite(pos) {
        return None;
    }
    let mut best = None;
    let mut best_distance = radius;
    for edge in edges {
        for run in &edge.runs {
            for pair in run.windows(2) {
                let d = segment_distance(pair[0], pair[1], pos);
                if d < best_distance {
                    best_distance = d;
                    best = Some(edge.key);
                }
            }
        }
    }
    best
}

/// 位置を含む表向きの三角形のうち、最も手前のものが属する面。
pub fn front_face(
    surface: &SurfaceScreen,
    tris: &[[usize; 3]],
    tri_face: &[usize],
    pos: ScreenPos,
) -> Option<usize> {
    if !is_finite(pos) {
        return None;
    }
    let [x, y] = pos;
    let mut best = None;
    let mut best_depth = f64::INFINITY;
    for (t, tri) in tris.iter().enumerate() {
        let Some(&face) = tri_face.get(t) else {
            continue;
        };
        let nz = tri.map(|i| surface.facing_z.get(i).copied().unwrap_or(f64::NAN));
        let mean = (nz[0] + nz[1] + nz[2]) / 3.0;
        if mean.is_nan() || mean > BACK_FACE_LIMIT {
            continue;
        }
        let corners = tri.map(|i| surface.points.get(i).copied().flatten());
        let [Some(a), Some(b), Some(c)] = corners else {
            continue;
        };
        let d = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if d.is_nan() || d.abs() < 1e-9 {
            continue;
        }
        let l1 = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / d;
        let l2 = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / d;
        if l1 < 0.0 || l2 < 0.0 || l1 + l2 > 1.0 {
            continue;
        }
        let depth = a.depth * l1 + b.depth * l2 + c.depth * (1.0 - l1 - l2);
        if depth < best_depth {
            best_depth = depth;
            best = Some(face);
        }
    }
    best
}

/// いまのモードの要素のうち、画面上の位置にあるもの。`model` は `cage` から組み立てたもの。
pub fn pick_element(
    view: &ScreenView,
    cage: &CageModel,
    model: &Model,
    mode: SelectMode,
    pos: ScreenPos,
) -> Option<Element> {
    if !is_finite(pos) {
        return None;
    }
    match mode {
        SelectMode::Vertex => {
            let points = vertex_screen_points(view, &cage.v, &model.nrm);
            nearest_point(&points, pos, VERTEX_PICK_RADIUS).map(Element::Vertex)
        }
        SelectMode::Edge => {
            let runs = edge_screen_runs(view, &model.edges, &model.nrm);
            nearest_edge(&runs, pos, EDGE_PICK_RADIUS).map(Element::Edge)
        }
        SelectMode::Face => {
            let surface = surface_screen(view, &model.mesh);
            front_face(&surface, &model.mesh.tris, &model.mesh.tri_face, pos).map(Element::Face)
        }
    }
}

fn is_finite(p: ScreenPos) -> bool {
    p.iter().all(|c| c.is_finite())
}
