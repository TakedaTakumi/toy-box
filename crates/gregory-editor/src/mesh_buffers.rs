//! gregory-core の出力から、GPU に送る頂点・インデックス・線分・点の配列を作る。

use std::collections::HashSet;

use bytemuck::{Pod, Zeroable};
use gregory_core::{
    CageModel, Edge, EdgeKey, Model, SelectMode, Selection, TriMesh, Vec3, cross, norm, sub,
};

use crate::palette;
use crate::settings::Shading;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SurfaceVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// 1 なら選択中の面として色を混ぜる。
    pub tint: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

/// 画面上で一定の大きさの四角として描く点。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PointInstance {
    pub position: [f32; 3],
    pub color: [f32; 3],
    /// 一辺の長さ（ポイント）。
    pub size: f32,
}

/// 選択中の点の一辺（ポイント）。
pub const POINT_SIZE_SELECTED: f32 = 11.0;
/// 点モードで、選んでいない点の一辺。
pub const POINT_SIZE_VERTEX_MODE: f32 = 8.0;
/// 辺・面モードでの点の一辺。
pub const POINT_SIZE_OTHER: f32 = 5.0;

/// 選択の表示に使う、モードと選択中の要素。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub mode: SelectMode,
    pub vertices: HashSet<usize>,
    pub edges: HashSet<EdgeKey>,
    pub faces: HashSet<usize>,
}

impl Default for Highlight {
    fn default() -> Self {
        Self::of(&Selection::Vertices(Vec::new()))
    }
}

impl Highlight {
    pub fn of(selection: &Selection) -> Self {
        let mut h = Self {
            mode: selection.mode(),
            vertices: HashSet::new(),
            edges: HashSet::new(),
            faces: HashSet::new(),
        };
        match selection {
            Selection::Vertices(v) => h.vertices.extend(v.iter().copied()),
            Selection::Edges(e) => h.edges.extend(e.iter().copied()),
            Selection::Faces(f) => h.faces.extend(f.iter().copied()),
        }
        h
    }
}

/// 三角形リスト。`indices` は3つずつで1つの三角形。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceMesh {
    pub vertices: Vec<SurfaceVertex>,
    pub indices: Vec<u32>,
}

/// 1回の描画に必要な配列一式。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneGeometry {
    pub surface: SurfaceMesh,
    /// 2つずつで1本の線分。
    pub cage_lines: Vec<LineVertex>,
    pub cage_points: Vec<PointInstance>,
}

pub fn scene_geometry(
    cage: &CageModel,
    model: &Model,
    shading: Shading,
    highlight: &Highlight,
) -> SceneGeometry {
    let tinted = if highlight.mode == SelectMode::Face {
        &highlight.faces
    } else {
        &HashSet::new()
    };
    SceneGeometry {
        surface: surface_mesh(&model.mesh, shading, tinted),
        cage_lines: cage_lines(&cage.v, &model.edges, highlight),
        cage_points: cage_points(&cage.v, highlight),
    }
}

/// `tinted` は、選択の色を混ぜる面の番号。
pub fn surface_mesh(mesh: &TriMesh, shading: Shading, tinted: &HashSet<usize>) -> SurfaceMesh {
    match shading {
        Shading::Smooth => smooth_surface(mesh, tinted),
        Shading::Flat => flat_surface(mesh, tinted),
    }
}

/// 頂点を共有し、法線は gregory-core の頂点法線を使う。
///
/// gregory-core はパッチごとに頂点を分けて作るので、頂点の色味は、その頂点を使う三角形の面で決める
/// （どれか1つでも色を混ぜる面なら混ぜる）。
pub fn smooth_surface(mesh: &TriMesh, tinted: &HashSet<usize>) -> SurfaceMesh {
    let positions: Vec<Option<[f32; 3]>> = mesh.pos.iter().map(|p| to_finite_f32(*p)).collect();
    let mut tint = vec![0.0f32; positions.len()];
    for (tri, face) in mesh.tris.iter().zip(&mesh.tri_face) {
        if tinted.contains(face) {
            for &i in tri {
                if let Some(t) = tint.get_mut(i) {
                    *t = 1.0;
                }
            }
        }
    }
    let vertices = positions
        .iter()
        .enumerate()
        .map(|(i, p)| SurfaceVertex {
            position: p.unwrap_or_default(),
            normal: mesh
                .nrm
                .get(i)
                .and_then(|n| to_finite_f32(*n))
                .unwrap_or_default(),
            tint: tint[i],
        })
        .collect();
    let mut indices = Vec::with_capacity(mesh.tris.len() * 3);
    for tri in &mesh.tris {
        let drawable = tri
            .iter()
            .all(|&i| positions.get(i).is_some_and(Option::is_some));
        let ids = tri.map(|i| u32::try_from(i).ok());
        if let (true, [Some(a), Some(b), Some(c)]) = (drawable, ids) {
            indices.extend([a, b, c]);
        }
    }
    SurfaceMesh { vertices, indices }
}

/// 三角形ごとに頂点を分け、3頂点とも三角形の面法線にする。
pub fn flat_surface(mesh: &TriMesh, tinted: &HashSet<usize>) -> SurfaceMesh {
    let mut vertices = Vec::with_capacity(mesh.tris.len() * 3);
    for (t, tri) in mesh.tris.iter().enumerate() {
        let corners = tri.map(|i| mesh.pos.get(i).copied());
        let [Some(a), Some(b), Some(c)] = corners else {
            continue;
        };
        let (Some(fa), Some(fb), Some(fc)) = (to_finite_f32(a), to_finite_f32(b), to_finite_f32(c))
        else {
            continue;
        };
        let normal = to_finite_f32(norm(cross(sub(b, a), sub(c, a)))).unwrap_or_default();
        let tint = match mesh.tri_face.get(t) {
            Some(face) if tinted.contains(face) => 1.0,
            _ => 0.0,
        };
        vertices.extend([fa, fb, fc].map(|position| SurfaceVertex {
            position,
            normal,
            tint,
        }));
    }
    let count = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
    let indices = (0..count).collect();
    SurfaceMesh { vertices, indices }
}

/// ケージの辺を線分にする。辺モードで選択中の辺と、折れ目の辺は色を変える（選択の色を優先する）。
pub fn cage_lines(vertices: &[Vec3], edges: &[Edge], highlight: &Highlight) -> Vec<LineVertex> {
    let mut lines = Vec::with_capacity(edges.len() * 2);
    for edge in edges {
        let ends = (
            vertices.get(edge.a).and_then(|p| to_finite_f32(*p)),
            vertices.get(edge.b).and_then(|p| to_finite_f32(*p)),
        );
        let (Some(a), Some(b)) = ends else {
            continue;
        };
        let selected = highlight.mode == SelectMode::Edge
            && highlight.edges.contains(&EdgeKey::new(edge.a, edge.b));
        let color = if selected {
            palette::SELECTION
        } else if edge.crease {
            palette::CREASE_LINE
        } else {
            palette::CAGE_LINE
        };
        lines.push(LineVertex { position: a, color });
        lines.push(LineVertex { position: b, color });
    }
    lines
}

/// ケージの頂点を点にする。点モードでは大きめにし、選択中の点はさらに大きく選択の色にする。
pub fn cage_points(vertices: &[Vec3], highlight: &Highlight) -> Vec<PointInstance> {
    let vertex_mode = highlight.mode == SelectMode::Vertex;
    vertices
        .iter()
        .enumerate()
        .filter_map(|(i, p)| to_finite_f32(*p).map(|position| (i, position)))
        .map(|(i, position)| {
            let (color, size) = if vertex_mode && highlight.vertices.contains(&i) {
                (palette::SELECTION, POINT_SIZE_SELECTED)
            } else if vertex_mode {
                (palette::CAGE_POINT, POINT_SIZE_VERTEX_MODE)
            } else {
                (palette::CAGE_POINT, POINT_SIZE_OTHER)
            };
            PointInstance {
                position,
                color,
                size,
            }
        })
        .collect()
}

/// f32 にしても有限な値だけを通す（f32 の範囲を超える値は無限大になるため、変換後に確かめる）。
fn to_finite_f32(v: Vec3) -> Option<[f32; 3]> {
    let f = v.map(|x| x as f32);
    f.iter().all(|x| x.is_finite()).then_some(f)
}
