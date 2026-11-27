//! 入力が変わったときだけ値を計算し直す入れ物。

/// 直前のキーと値を1組だけ覚える。`revision` は値を計算し直すたびに増える。
#[derive(Debug)]
pub struct Memo<K, V> {
    entry: Option<(K, V)>,
    revision: u64,
}

impl<K, V> Default for Memo<K, V> {
    fn default() -> Self {
        Self {
            entry: None,
            revision: 0,
        }
    }
}

impl<K: PartialEq, V> Memo<K, V> {
    /// キーが前回と同じなら覚えている値を返し、違えば `compute` で計算し直す。
    pub fn get_or_compute(&mut self, key: K, compute: impl FnOnce(&K) -> V) -> &V {
        if self.entry.as_ref().is_some_and(|(k, _)| *k != key) {
            self.entry = None;
        }
        let revision = &mut self.revision;
        let (_, value) = self.entry.get_or_insert_with(|| {
            *revision = revision.wrapping_add(1);
            let value = compute(&key);
            (key, value)
        });
        value
    }

    /// 覚えている値（まだ一度も計算していなければ `None`）。
    pub fn value(&self) -> Option<&V> {
        self.entry.as_ref().map(|(_, v)| v)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}
