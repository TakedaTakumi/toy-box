//! 元に戻す・やり直しの履歴（コマンド方式）。

use std::collections::VecDeque;

use gregory_core::{CageModel, Vec3};

/// 履歴に残す操作の上限（JS 試作と同じ）。
pub const HISTORY_LIMIT: usize = 100;

/// ケージに対する1回の編集。`apply` で編集後に、`revert` で編集前にする。
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// `indices[k]` の頂点を `before[k]` から `after[k]` へ動かす。
    MoveVertices {
        indices: Vec<usize>,
        before: Vec<Vec3>,
        after: Vec<Vec3>,
    },
    /// ケージ全体を置き換える。
    ReplaceCage {
        before: Box<CageModel>,
        after: Box<CageModel>,
    },
}

impl Command {
    /// 編集後の状態にする。範囲外の番号は無視する。
    pub fn apply(&self, cage: &mut CageModel) {
        match self {
            Self::MoveVertices { indices, after, .. } => set_positions(cage, indices, after),
            Self::ReplaceCage { after, .. } => cage.clone_from(after),
        }
    }

    /// 編集前の状態にする。範囲外の番号は無視する。
    pub fn revert(&self, cage: &mut CageModel) {
        match self {
            Self::MoveVertices {
                indices, before, ..
            } => set_positions(cage, indices, before),
            Self::ReplaceCage { before, .. } => cage.clone_from(before),
        }
    }
}

/// 番号と位置の組を、短い方の長さまで書き込む。
pub fn set_positions(cage: &mut CageModel, indices: &[usize], positions: &[Vec3]) {
    for (&i, &p) in indices.iter().zip(positions) {
        if let Some(slot) = cage.v.get_mut(i) {
            *slot = p;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    undo: VecDeque<Command>,
    redo: Vec<Command>,
}

impl History {
    /// 適用済みの操作を積む。やり直しの履歴は消え、上限を超えた古いものから捨てる。
    pub fn push(&mut self, command: Command) {
        self.redo.clear();
        self.undo.push_back(command);
        while self.undo.len() > HISTORY_LIMIT {
            self.undo.pop_front();
        }
    }

    /// 直前の操作を取り消す。取り消せたら真。
    pub fn undo(&mut self, cage: &mut CageModel) -> bool {
        let Some(command) = self.undo.pop_back() else {
            return false;
        };
        command.revert(cage);
        self.redo.push(command);
        true
    }

    /// 取り消した操作をやり直す。やり直せたら真。
    pub fn redo(&mut self, cage: &mut CageModel) -> bool {
        let Some(command) = self.redo.pop() else {
            return false;
        };
        command.apply(cage);
        self.undo.push_back(command);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }
}
