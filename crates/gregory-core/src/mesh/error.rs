//! 編集操作の失敗。JS 版が例外を投げるか、`undefined` の混ざった結果を返す入力に対応する。

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// 面が、存在しない頂点の番号を参照している（JS では TypeError か、面に `undefined` が混ざる）。
    VertexOutOfRange { vertex: usize, vertex_count: usize },
    /// ループ分割で、面の角か辺の中点が定まらない（JS では、面に `undefined` が混ざる）。
    UndefinedCorner { face: usize },
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VertexOutOfRange {
                vertex,
                vertex_count,
            } => write!(f, "頂点番号{vertex}が範囲外（頂点数{vertex_count}）"),
            Self::UndefinedCorner { face } => {
                write!(f, "面{face}のループ分割で、角か辺の中点が定まらない")
            }
        }
    }
}

impl std::error::Error for MeshError {}
