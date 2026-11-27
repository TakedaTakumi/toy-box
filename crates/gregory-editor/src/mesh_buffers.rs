//! gregory-core の出力から、GPU に送る頂点・インデックス・線分・点の配列を作る。

use bytemuck::{Pod, Zeroable};
use gregory_core::{CageModel, Edge, Model, TriMesh, Vec3, cross, norm, sub};

use crate::palette;
use crate::settings::Shading;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SurfaceVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
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

pub fn scene_geometry(cage: &CageModel, model: &Model, shading: Shading) -> SceneGeometry {
    SceneGeometry {
        surface: surface_mesh(&model.mesh, shading),
        cage_lines: cage_lines(&cage.v, &model.edges),
        cage_points: cage_points(&cage.v),
    }
}

pub fn surface_mesh(mesh: &TriMesh, shading: Shading) -> SurfaceMesh {
    match shading {
        Shading::Smooth => smooth_surface(mesh),
        Shading::Flat => flat_surface(mesh),
    }
}

/// 頂点を共有し、法線は gregory-core の頂点法線を使う。
pub fn smooth_surface(mesh: &TriMesh) -> SurfaceMesh {
    let positions: Vec<Option<[f32; 3]>> = mesh.pos.iter().map(|p| to_finite_f32(*p)).collect();
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
pub fn flat_surface(mesh: &TriMesh) -> SurfaceMesh {
    let mut vertices = Vec::with_capacity(mesh.tris.len() * 3);
    for tri in &mesh.tris {
        let corners = tri.map(|i| mesh.pos.get(i).copied());
        let [Some(a), Some(b), Some(c)] = corners else {
            continue;
        };
        let (Some(fa), Some(fb), Some(fc)) = (to_finite_f32(a), to_finite_f32(b), to_finite_f32(c))
        else {
            continue;
        };
        let normal = to_finite_f32(norm(cross(sub(b, a), sub(c, a)))).unwrap_or_default();
        vertices.extend([fa, fb, fc].map(|position| SurfaceVertex { position, normal }));
    }
    let count = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
    let indices = (0..count).collect();
    SurfaceMesh { vertices, indices }
}

/// ケージの辺を線分にする。折れ目の辺は色を変える。
pub fn cage_lines(vertices: &[Vec3], edges: &[Edge]) -> Vec<LineVertex> {
    let mut lines = Vec::with_capacity(edges.len() * 2);
    for edge in edges {
        let ends = (
            vertices.get(edge.a).and_then(|p| to_finite_f32(*p)),
            vertices.get(edge.b).and_then(|p| to_finite_f32(*p)),
        );
        let (Some(a), Some(b)) = ends else {
            continue;
        };
        let color = if edge.crease {
            palette::CREASE_LINE
        } else {
            palette::CAGE_LINE
        };
        lines.push(LineVertex { position: a, color });
        lines.push(LineVertex { position: b, color });
    }
    lines
}

pub fn cage_points(vertices: &[Vec3]) -> Vec<PointInstance> {
    vertices
        .iter()
        .filter_map(|p| to_finite_f32(*p))
        .map(|position| PointInstance {
            position,
            color: palette::CAGE_POINT,
        })
        .collect()
}

/// f32 にしても有限な値だけを通す（f32 の範囲を超える値は無限大になるため、変換後に確かめる）。
fn to_finite_f32(v: Vec3) -> Option<[f32; 3]> {
    let f = v.map(|x| x as f32);
    f.iter().all(|x| x.is_finite()).then_some(f)
}
