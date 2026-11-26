//! ケージから曲面を組み立てる（JS の `buildModel`）。

use std::collections::HashSet;

use crate::cage::Cage;
use crate::continuity::{ContinuityCheck, check_continuity};
use crate::curves::EdgeCurves;
use crate::edge_data::edge_data;
use crate::error::GregoryError;
use crate::normals::{corner_normals, vertex_normals};
use crate::patch::{EdgeData, Patch, QuadPatch, TriPatch};
use crate::tessellate::{TriMesh, tessellate};
use crate::topology::{Edge, EdgeKey, EdgeMap, FaceCorner};
use crate::vec3::Vec3;

/// 辺をまたぐ接線場の決め方。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    /// 両側の面で接線場を共有し、G1 連続にする（JS の `'g1'`）。
    G1,
    /// 隣の辺の接線を線形に補間する（JS で `'g1'` 以外を渡したとき）。
    Linear,
}

impl Mode {
    /// JS の `mode` の文字列から決める。`"g1"` 以外はすべて `Linear`。
    pub fn from_js_name(name: &str) -> Self {
        if name == "g1" { Self::G1 } else { Self::Linear }
    }
}

/// JS の `opts`。`n` は JS の `N`（テッセレーションの分割数）。
#[derive(Clone, Debug, PartialEq)]
pub struct BuildOptions {
    pub tau: f64,
    pub mode: Mode,
    pub n: usize,
    pub creases: HashSet<EdgeKey>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self { tau: 1.0, mode: Mode::G1, n: 8, creases: HashSet::new() }
    }
}

/// `q` は四角形（4辺）の面、`t` は三角形（3辺）の面の数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceCount {
    pub q: usize,
    pub t: usize,
}

/// JS の `buildModel` の戻り値。`nrm` は頂点ごとの全周の法線。
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub nrm: Vec<Vec3>,
    pub patches: Vec<Patch>,
    pub edges: Vec<Edge>,
    pub mesh: TriMesh,
    pub check: ContinuityCheck,
    pub count: FaceCount,
}

pub fn build_model(v: &[Vec3], f: &[Vec<usize>], opts: &BuildOptions) -> Result<Model, GregoryError> {
    validate(v, f)?;
    let cage = Cage::new(v, f);
    let edge_map = EdgeMap::build(f);
    let nrm = vertex_normals(cage);
    let corner = corner_normals(cage, &opts.creases);
    let mut curves = EdgeCurves::new(cage, &edge_map, &corner, &opts.creases, opts.tau);

    let mut patches = Vec::with_capacity(f.len());
    for (fi, face) in f.iter().enumerate() {
        let ed: Vec<EdgeData> = (0..face.len())
            .map(|k| edge_data(cage, &corner, &mut curves, opts.mode, FaceCorner { fi, k }))
            .collect();
        patches.push(make_patch(fi, ed)?);
    }

    let edges: Vec<Edge> = edge_map
        .entries()
        .iter()
        .map(|entry| Edge {
            a: entry.a,
            b: entry.b,
            faces: entry.faces.clone(),
            e: curves.curve(entry.a, entry.b),
            crease: opts.creases.contains(&EdgeKey::new(entry.a, entry.b)),
        })
        .collect();

    let check = check_continuity(f, &patches, &edges)?;
    let mesh = tessellate(&patches, opts.n);
    let count = FaceCount {
        q: f.iter().filter(|face| face.len() == 4).count(),
        t: f.iter().filter(|face| face.len() == 3).count(),
    };
    Ok(Model { nrm, patches, edges, mesh, check, count })
}

/// JS 版が必ず例外を投げる入力を、計算の前に弾く。
fn validate(v: &[Vec3], f: &[Vec<usize>]) -> Result<(), GregoryError> {
    for (fi, face) in f.iter().enumerate() {
        if let Some((k, &vertex)) = face.iter().enumerate().find(|&(_, &x)| x >= v.len()) {
            return Err(GregoryError::VertexOutOfRange { face: fi, corner: k, vertex, vertex_count: v.len() });
        }
    }
    match f.iter().position(|face| face.len() < 3) {
        Some(fi) => Err(GregoryError::TooFewCorners { face: fi, corners: f[fi].len() }),
        None => Ok(()),
    }
}

/// 4辺の面は四角形パッチ、それ以外は三角形パッチにする（5辺以上でも、JS と同じく三角形として扱う）。
fn make_patch(fi: usize, ed: Vec<EdgeData>) -> Result<Patch, GregoryError> {
    match <[EdgeData; 4]>::try_from(ed) {
        Ok(quad) => Ok(Patch::Quad(QuadPatch::new(quad))),
        Err(ed) => {
            let corners = ed.len();
            TriPatch::new(ed).map(Patch::Tri).ok_or(GregoryError::TooFewCorners { face: fi, corners })
        }
    }
}
