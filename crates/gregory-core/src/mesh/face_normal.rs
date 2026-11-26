//! 面の法線。

use super::error::MeshError;
use super::model::vertex_at;
use crate::vec3::{Vec3, add, cross, norm};

/// 各辺の外積の和を正規化した、面の向きの法線（JS の `faceNormal`）。
pub fn face_normal(v: &[Vec3], f: &[usize]) -> Result<Vec3, MeshError> {
    let m = f.len();
    let mut n = [0.0; 3];
    for (i, &a) in f.iter().enumerate() {
        n = add(n, cross(vertex_at(v, a)?, vertex_at(v, f[(i + 1) % m])?));
    }
    Ok(norm(n))
}
