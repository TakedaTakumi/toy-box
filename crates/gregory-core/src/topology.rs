//! ケージの辺と、面の角の参照。

use std::collections::HashMap;
use std::fmt;

use crate::vec3::Vec3;

/// 向きを持たない辺の名前。JS の `ekey(a, b)`（"小さい番号-大きい番号"）に当たる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeKey {
    a: usize,
    b: usize,
}

impl EdgeKey {
    pub fn new(x: usize, y: usize) -> Self {
        Self { a: x.min(y), b: x.max(y) }
    }

    pub fn a(self) -> usize {
        self.a
    }

    pub fn b(self) -> usize {
        self.b
    }
}

impl fmt::Display for EdgeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.a, self.b)
    }
}

/// 面 `fi` の `k` 番目の角（辺としては、角 `k` から角 `k + 1` へ向かう辺）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FaceCorner {
    pub fi: usize,
    pub k: usize,
}

/// `build_model` が返す辺。`e` は辺の3次ベジェ曲線の制御点（`a` から `b` の向き）。
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub faces: Vec<FaceCorner>,
    pub e: [Vec3; 4],
    pub crease: bool,
}

pub(crate) struct EdgeEntry {
    pub a: usize,
    pub b: usize,
    pub faces: Vec<FaceCorner>,
}

/// 辺ごとに、その辺を持つ面の角を、初出の順に並べた表。
pub(crate) struct EdgeMap {
    entries: Vec<EdgeEntry>,
    index: HashMap<EdgeKey, usize>,
}

impl EdgeMap {
    pub fn build(f: &[Vec<usize>]) -> Self {
        let mut entries: Vec<EdgeEntry> = Vec::new();
        let mut index: HashMap<EdgeKey, usize> = HashMap::new();
        for (fi, face) in f.iter().enumerate() {
            let m = face.len();
            for k in 0..m {
                let key = EdgeKey::new(face[k], face[(k + 1) % m]);
                let slot = *index.entry(key).or_insert_with(|| {
                    entries.push(EdgeEntry { a: key.a, b: key.b, faces: Vec::new() });
                    entries.len() - 1
                });
                entries[slot].faces.push(FaceCorner { fi, k });
            }
        }
        Self { entries, index }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&EdgeEntry> {
        self.index.get(&EdgeKey::new(x, y)).map(|&i| &self.entries[i])
    }

    pub fn entries(&self) -> &[EdgeEntry] {
        &self.entries
    }
}
