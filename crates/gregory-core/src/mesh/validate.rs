//! ケージの位相と座標の検査。

use std::collections::HashSet;
use std::fmt;

use super::model::CageModel;
use super::ordered::OrderedMap;
use crate::topology::{EdgeKey, EdgeMap};

/// `validate_model` が見つけた問題。`Display` は JS の問題の文字列と同じ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelProblem {
    /// 座標に NaN・無限大がある。
    InvalidCoordinate { vertex: usize },
    /// 面の辺の数が 3 でも 4 でもない。
    FaceSides { face: usize, sides: usize },
    /// 面の中で頂点番号が重複している。
    DuplicateVertex { face: usize },
    /// 面の頂点番号が範囲外（範囲外の角ごとに1件）。
    VertexOutOfRange { face: usize },
    /// 同じ向きの辺が複数の角で使われている（面の向きの不一致など）。
    DirectedEdgeReused {
        from: usize,
        to: usize,
        count: usize,
    },
    /// 3面以上で共有されている辺。
    EdgeShared { edge: EdgeKey, faces: usize },
    /// どの面にも使われていない頂点。
    UnusedVertex { vertex: usize },
}

impl fmt::Display for ModelProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoordinate { vertex } => write!(f, "頂点{vertex}の座標が不正"),
            Self::FaceSides { face, sides } => write!(f, "面{face}が{sides}辺"),
            Self::DuplicateVertex { face } => write!(f, "面{face}に重複頂点"),
            Self::VertexOutOfRange { face } => write!(f, "面{face}の頂点番号が範囲外"),
            Self::DirectedEdgeReused { from, to, count } => {
                write!(f, "有向辺{from}>{to}が{count}回使われている")
            }
            Self::EdgeShared { edge, faces } => write!(f, "辺{edge}が{faces}面で共有されている"),
            Self::UnusedVertex { vertex } => write!(f, "頂点{vertex}が未使用"),
        }
    }
}

/// 問題を JS と同じ順に並べて返す。問題がなければ空。
pub fn validate_model(model: &CageModel) -> Vec<ModelProblem> {
    let (v, f) = (&model.v, &model.f);
    let mut problems = Vec::new();
    for (i, p) in v.iter().enumerate() {
        if p.iter().any(|x| !x.is_finite()) {
            problems.push(ModelProblem::InvalidCoordinate { vertex: i });
        }
    }
    let mut directed: OrderedMap<(usize, usize), usize> = OrderedMap::default();
    for (fi, face) in f.iter().enumerate() {
        let n = face.len();
        if n != 3 && n != 4 {
            problems.push(ModelProblem::FaceSides { face: fi, sides: n });
        }
        if face.iter().collect::<HashSet<_>>().len() != n {
            problems.push(ModelProblem::DuplicateVertex { face: fi });
        }
        for (i, &x) in face.iter().enumerate() {
            if x >= v.len() {
                problems.push(ModelProblem::VertexOutOfRange { face: fi });
            }
            let (_, count) = directed.slot_mut((x, face[(i + 1) % n]), || 0);
            *count += 1;
        }
    }
    for &((from, to), count) in directed.iter() {
        if count > 1 {
            problems.push(ModelProblem::DirectedEdgeReused { from, to, count });
        }
    }
    for e in EdgeMap::build(f).entries() {
        if e.faces.len() > 2 {
            problems.push(ModelProblem::EdgeShared {
                edge: e.key(),
                faces: e.faces.len(),
            });
        }
    }
    let used: HashSet<usize> = f.iter().flatten().copied().collect();
    problems.extend(
        (0..v.len())
            .filter(|i| !used.contains(i))
            .map(|vertex| ModelProblem::UnusedVertex { vertex }),
    );
    problems
}
