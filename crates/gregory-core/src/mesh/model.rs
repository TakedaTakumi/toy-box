//! 編集対象のケージ。

use std::collections::HashSet;

use super::error::MeshError;
use crate::topology::EdgeKey;
use crate::vec3::Vec3;

/// JS の `{ V, F, creases }`。`creases` は JS の `Set` と違い、反復順を持たない。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CageModel {
    pub v: Vec<Vec3>,
    pub f: Vec<Vec<usize>>,
    pub creases: HashSet<EdgeKey>,
}

pub fn clone_model(m: &CageModel) -> CageModel {
    m.clone()
}

pub(crate) fn vertex_at(v: &[Vec3], i: usize) -> Result<Vec3, MeshError> {
    v.get(i).copied().ok_or(MeshError::VertexOutOfRange {
        vertex: i,
        vertex_count: v.len(),
    })
}
