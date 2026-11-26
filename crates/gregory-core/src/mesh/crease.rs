//! 折れ目の切り替え。

use super::model::CageModel;
use crate::topology::{EdgeKey, EdgeMap};

/// 存在する辺だけを対象に、すべてが折れ目なら外し、そうでなければすべてを折れ目にする。
pub fn toggle_creases(model: &CageModel, edge_keys: &[EdgeKey]) -> CageModel {
    let edges = EdgeMap::build(&model.f);
    let keys: Vec<EdgeKey> = edge_keys
        .iter()
        .copied()
        .filter(|&k| edges.contains(k))
        .collect();
    let mut out = model.clone();
    let all_on = !keys.is_empty() && keys.iter().all(|k| out.creases.contains(k));
    for k in keys {
        if all_on {
            out.creases.remove(&k);
        } else {
            out.creases.insert(k);
        }
    }
    out
}
