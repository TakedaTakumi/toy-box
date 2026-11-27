//! 選択の要素と、モードの間の変換。

use std::collections::HashSet;

use super::model::CageModel;
use super::ordered::OrderedSet;
use crate::topology::{EdgeEntry, EdgeKey, EdgeMap};

/// 選択のモード（JS の `'v'`・`'e'`・`'f'`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectMode {
    Vertex,
    Edge,
    Face,
}

impl SelectMode {
    /// JS の `mode` の文字列から決める。`"v"`・`"e"` 以外はすべて `Face`。
    pub fn from_js_name(name: &str) -> Self {
        match name {
            "v" => Self::Vertex,
            "e" => Self::Edge,
            _ => Self::Face,
        }
    }
}

/// モードと要素の組（JS の `mode` と `items`）。点は頂点番号、辺は辺の名前、面は面番号。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Vertices(Vec<usize>),
    Edges(Vec<EdgeKey>),
    Faces(Vec<usize>),
}

impl Selection {
    pub fn mode(&self) -> SelectMode {
        match self {
            Self::Vertices(_) => SelectMode::Vertex,
            Self::Edges(_) => SelectMode::Edge,
            Self::Faces(_) => SelectMode::Face,
        }
    }
}

/// 選択の要素に含まれる頂点（重複なし、初出の順）。範囲外の要素は無視する。
///
/// 辺は、両端が範囲内なら、辺が存在しなくても両端を入れる。面は、その面の頂点を範囲を確かめずに入れる。
pub fn selection_vertices(model: &CageModel, items: &Selection) -> Vec<usize> {
    let nv = model.v.len();
    let mut out = OrderedSet::default();
    match items {
        Selection::Vertices(vs) => {
            for &x in vs.iter().filter(|&&x| x < nv) {
                out.insert(x);
            }
        }
        Selection::Edges(es) => {
            for e in es.iter().filter(|e| e.a() < nv && e.b() < nv) {
                out.insert(e.a());
                out.insert(e.b());
            }
        }
        Selection::Faces(fs) => {
            for face in fs.iter().filter_map(|&fi| model.f.get(fi)) {
                for &v in face {
                    out.insert(v);
                }
            }
        }
    }
    out.into_vec()
}

/// 選択を別のモードの要素に変換する。辺は両端が選ばれたもの、面はすべての頂点が選ばれたもの（頂点のない面も含む）。
pub fn convert_selection(model: &CageModel, from: &Selection, to: SelectMode) -> Selection {
    let verts = selection_vertices(model, from);
    let chosen: HashSet<usize> = verts.iter().copied().collect();
    match to {
        SelectMode::Vertex => Selection::Vertices(verts),
        SelectMode::Edge => Selection::Edges(
            EdgeMap::build(&model.f)
                .entries()
                .iter()
                .filter(|e| chosen.contains(&e.a) && chosen.contains(&e.b))
                .map(EdgeEntry::key)
                .collect(),
        ),
        SelectMode::Face => Selection::Faces(
            model
                .f
                .iter()
                .enumerate()
                .filter(|(_, face)| face.iter().all(|v| chosen.contains(v)))
                .map(|(i, _)| i)
                .collect(),
        ),
    }
}
