//! 辺の名前と辺の表。

use crate::topology::{EdgeKey, EdgeMap};

pub fn ekey(a: usize, b: usize) -> EdgeKey {
    EdgeKey::new(a, b)
}

pub fn edge_map(f: &[Vec<usize>]) -> EdgeMap {
    EdgeMap::build(f)
}
