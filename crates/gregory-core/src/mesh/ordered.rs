//! 挿入順を保つ集合と表。JS の `Set`・`Map` の反復順が結果の番号の振り方を決めるため、それを再現する。

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub(crate) struct OrderedSet<T> {
    items: Vec<T>,
    index: HashMap<T, usize>,
}

impl<T> Default for OrderedSet<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> OrderedSet<T> {
    /// 新しく入れたときだけ `true`。
    pub fn insert(&mut self, x: T) -> bool {
        if self.index.contains_key(&x) {
            return false;
        }
        self.index.insert(x, self.items.len());
        self.items.push(x);
        true
    }

    pub fn contains(&self, x: &T) -> bool {
        self.index.contains_key(x)
    }

    pub fn position(&self, x: &T) -> Option<usize> {
        self.index.get(x).copied()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T: Copy + Eq + Hash> FromIterator<T> for OrderedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::default();
        for x in iter {
            set.insert(x);
        }
        set
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OrderedMap<K, V> {
    entries: Vec<(K, V)>,
    index: HashMap<K, usize>,
}

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<K: Copy + Eq + Hash, V> OrderedMap<K, V> {
    /// `k` の位置と値。なければ `init` の値を末尾に入れる。
    pub fn slot_mut(&mut self, k: K, init: impl FnOnce() -> V) -> (usize, &mut V) {
        let slot = match self.index.get(&k) {
            Some(&slot) => slot,
            None => {
                self.index.insert(k, self.entries.len());
                self.entries.push((k, init()));
                self.entries.len() - 1
            }
        };
        (slot, &mut self.entries[slot].1)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (K, V)> {
        self.entries.iter()
    }
}
