//! ポインタのドラッグの状態機械。押す → 動かす → 離す（確定）か取り消し。

use gregory_core::{CageModel, Vec3, add};

use super::gizmo::{Constraint, Handle, ScreenPos, selection_center};
use super::history::{Command, set_positions};

/// 移動のドラッグは、累計の移動量（|dx| + |dy|）がこれを超えてから動き出す。
pub const MOVE_THRESHOLD: f64 = 3.0;
/// 空所を押して離すまでの移動量がこれ未満なら、クリックとみなす。
pub const CLICK_TOLERANCE: f64 = 4.0;

/// 移動のドラッグで動かす頂点と、開始時の位置。
#[derive(Clone, Debug, PartialEq)]
pub struct MoveSession {
    pub indices: Vec<usize>,
    pub start: Vec<Vec3>,
    pub center: Vec3,
    pub constraint: Constraint,
    /// 移動ハンドルをつかんだときの、そのハンドル（要素のドラッグでは `None`）。
    pub handle: Option<Handle>,
    /// 平面の拘束で、押した位置の視線と平面の交点。
    pub plane_origin: Option<Vec3>,
}

impl MoveSession {
    /// 範囲外の番号は除く。中心は開始時の位置の平均（なければ原点）。
    pub fn new(cage: &CageModel, indices: &[usize], constraint: Constraint) -> Self {
        let indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| i < cage.v.len())
            .collect();
        let start: Vec<Vec3> = indices.iter().map(|&i| cage.v[i]).collect();
        let center = selection_center(&cage.v, &indices).unwrap_or([0.0; 3]);
        Self {
            indices,
            start,
            center,
            constraint,
            handle: None,
            plane_origin: None,
        }
    }

    /// 開始時の位置から `delta` だけ動かす。有限でない移動量は無視する。
    pub fn apply(&self, cage: &mut CageModel, delta: Vec3) {
        if !delta.iter().all(|c| c.is_finite()) {
            return;
        }
        let moved: Vec<Vec3> = self.start.iter().map(|&p| add(p, delta)).collect();
        set_positions(cage, &self.indices, &moved);
    }

    /// 開始時の位置に戻す。
    pub fn restore(&self, cage: &mut CageModel) {
        set_positions(cage, &self.indices, &self.start);
    }

    /// 開始時の位置を移動前、いまの位置を移動後とする履歴のコマンド。
    pub fn command(&self, cage: &CageModel) -> Command {
        let after = self
            .indices
            .iter()
            .zip(&self.start)
            .map(|(&i, &s)| cage.v.get(i).copied().unwrap_or(s))
            .collect();
        Command::MoveVertices {
            indices: self.indices.clone(),
            before: self.start.clone(),
            after,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrbitDrag {
    pub last: ScreenPos,
    pub moved: f64,
    /// クリックで終わったときに選択を解除するか。
    pub clear_on_click: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveDrag {
    pub session: MoveSession,
    pub last: ScreenPos,
    pub moved: f64,
    /// 動き出してからの画面上の移動量の累計。
    pub total: ScreenPos,
    /// いまのポインタの位置（動き出すまでは押した位置）。
    pub pos: ScreenPos,
}

impl MoveDrag {
    pub fn started(&self) -> bool {
        self.moved > MOVE_THRESHOLD
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Drag {
    #[default]
    Idle,
    Orbit(OrbitDrag),
    Move(MoveDrag),
}

/// ポインタが動いたときに、呼び出し側がすること。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragStep {
    Nothing,
    /// 視点をこの量だけ回す。
    Orbit {
        dx: f64,
        dy: f64,
    },
    /// 移動の拘束に従い、この累計量・位置で頂点を動かす。
    Move {
        total: ScreenPos,
        pos: ScreenPos,
    },
}

/// 指を離したときの結果。
#[derive(Clone, Debug, PartialEq)]
pub enum DragEnd {
    Nothing,
    /// 空所のクリック。
    Click {
        clear_selection: bool,
    },
    /// 移動を確定した。履歴に積む。
    Moved(MoveSession),
}

impl Drag {
    /// 押した位置が有限でないときは、最初に動いた有限の位置を起点にする。
    pub fn begin_orbit(pos: ScreenPos, clear_on_click: bool) -> Self {
        Self::Orbit(OrbitDrag {
            last: pos,
            moved: 0.0,
            clear_on_click,
        })
    }

    /// 押した位置が有限でないときは、最初に動いた有限の位置を起点にする。
    pub fn begin_move(pos: ScreenPos, session: MoveSession) -> Self {
        Self::Move(MoveDrag {
            session,
            last: pos,
            moved: 0.0,
            total: [0.0, 0.0],
            pos,
        })
    }

    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    pub fn is_moving(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    /// 移動のドラッグ中なら、その拘束。
    pub fn move_constraint(&self) -> Option<Constraint> {
        match self {
            Self::Move(m) => Some(m.session.constraint),
            _ => None,
        }
    }

    /// ポインタが `pos` に動いた。有限でない位置は無視する。
    pub fn pointer_moved(&mut self, pos: ScreenPos) -> DragStep {
        if !is_finite(pos) {
            return DragStep::Nothing;
        }
        match self {
            Self::Idle => DragStep::Nothing,
            Self::Orbit(o) => {
                if !is_finite(o.last) {
                    o.last = pos;
                    return DragStep::Nothing;
                }
                let (dx, dy) = (pos[0] - o.last[0], pos[1] - o.last[1]);
                o.last = pos;
                o.moved += dx.abs() + dy.abs();
                DragStep::Orbit { dx, dy }
            }
            Self::Move(m) => {
                if !is_finite(m.last) {
                    m.last = pos;
                    m.pos = pos;
                    return DragStep::Nothing;
                }
                let (dx, dy) = (pos[0] - m.last[0], pos[1] - m.last[1]);
                m.last = pos;
                m.moved += dx.abs() + dy.abs();
                if !m.started() {
                    return DragStep::Nothing;
                }
                m.total = [m.total[0] + dx, m.total[1] + dy];
                m.pos = pos;
                DragStep::Move {
                    total: m.total,
                    pos: m.pos,
                }
            }
        }
    }

    /// 指を離した。状態は `Idle` に戻る。
    pub fn release(&mut self) -> DragEnd {
        match std::mem::take(self) {
            Self::Idle => DragEnd::Nothing,
            Self::Orbit(o) if o.moved < CLICK_TOLERANCE => DragEnd::Click {
                clear_selection: o.clear_on_click,
            },
            Self::Orbit(_) => DragEnd::Nothing,
            Self::Move(m) if m.started() => DragEnd::Moved(m.session),
            Self::Move(_) => DragEnd::Nothing,
        }
    }

    /// ドラッグを取り消す。動き出していた移動なら、元の位置に戻すためにその情報を返す。状態は `Idle` に戻る。
    pub fn cancel(&mut self) -> Option<MoveSession> {
        match std::mem::take(self) {
            Self::Move(m) if m.started() => Some(m.session),
            _ => None,
        }
    }
}

fn is_finite(p: ScreenPos) -> bool {
    p.iter().all(|c| c.is_finite())
}
