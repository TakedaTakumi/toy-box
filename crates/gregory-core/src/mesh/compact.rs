//! 使われていない頂点を取り除き、番号を詰める。

use std::collections::HashSet;

use super::error::MeshError;
use super::model::CageModel;
use crate::topology::{EdgeKey, EdgeMap};
use crate::vec3::Vec3;

/// `remap[i]` は、元の頂点 `i` の新しい番号（取り除いた頂点は `None`。JS の `Map` に無い番号）。
#[derive(Clone, Debug, PartialEq)]
pub struct Compacted {
    pub model: CageModel,
    pub remap: Vec<Option<usize>>,
}

/// 使われていない頂点を取り除き、折れ目の番号も振り直す（JS の `compact`）。
pub fn compact(model: &CageModel) -> Result<Compacted, MeshError> {
    compact_parts(&model.v, &model.f, &model.creases)
}

pub(crate) fn compact_parts(
    v: &[Vec3],
    f: &[Vec<usize>],
    creases: &HashSet<EdgeKey>,
) -> Result<Compacted, MeshError> {
    let used: HashSet<usize> = f.iter().flatten().copied().collect();
    let mut remap = vec![None; v.len()];
    let mut new_v = Vec::with_capacity(used.len());
    for (i, (p, slot)) in v.iter().zip(remap.iter_mut()).enumerate() {
        if used.contains(&i) {
            *slot = Some(new_v.len());
            new_v.push(*p);
        }
    }
    let new_f = f
        .iter()
        .map(|face| {
            face.iter()
                .map(|&x| {
                    remapped(&remap, x).ok_or(MeshError::VertexOutOfRange {
                        vertex: x,
                        vertex_count: v.len(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let edges = EdgeMap::build(&new_f);
    let new_creases = creases
        .iter()
        .filter_map(|k| {
            let key = EdgeKey::new(remapped(&remap, k.a())?, remapped(&remap, k.b())?);
            edges.contains(key).then_some(key)
        })
        .collect();
    Ok(Compacted {
        model: CageModel {
            v: new_v,
            f: new_f,
            creases: new_creases,
        },
        remap,
    })
}

fn remapped(remap: &[Option<usize>], i: usize) -> Option<usize> {
    remap.get(i).copied().flatten()
}
