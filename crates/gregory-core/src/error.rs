//! `build_model` の失敗。JS 版が例外を投げる入力に対応する。

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GregoryError {
    /// 面が、存在しない頂点の番号を参照している。
    VertexOutOfRange {
        face: usize,
        corner: usize,
        vertex: usize,
        vertex_count: usize,
    },
    /// 面の頂点が3つ未満。
    TooFewCorners { face: usize, corners: usize },
    /// 5辺以上の面の4辺目以降が、ちょうど2面で共有されている（三角形パッチとして扱うため、その辺を評価できない）。
    EdgeOutsidePatch { face: usize, edge: usize },
}

impl fmt::Display for GregoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VertexOutOfRange {
                face,
                corner,
                vertex,
                vertex_count,
            } => write!(
                f,
                "面{face}の{corner}番目の頂点番号{vertex}が範囲外（頂点数{vertex_count}）"
            ),
            Self::TooFewCorners { face, corners } => {
                write!(f, "面{face}の頂点が{corners}個しかない")
            }
            Self::EdgeOutsidePatch { face, edge } => {
                write!(f, "面{face}の辺{edge}は、パッチの辺として評価できない")
            }
        }
    }
}

impl std::error::Error for GregoryError {}
