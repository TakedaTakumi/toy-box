//! 面の削除。

use std::collections::HashSet;

use super::compact::compact_parts;
use super::error::MeshError;
use super::model::CageModel;

/// 指定した番号の面を取り除き、使われなくなった頂点も詰める。存在しない番号は無視する。
pub fn delete_faces(model: &CageModel, face_idxs: &[usize]) -> Result<CageModel, MeshError> {
    let del: HashSet<usize> = face_idxs.iter().copied().collect();
    let f: Vec<Vec<usize>> = model
        .f
        .iter()
        .enumerate()
        .filter(|(i, _)| !del.contains(i))
        .map(|(_, face)| face.clone())
        .collect();
    Ok(compact_parts(&model.v, &f, &model.creases)?.model)
}
