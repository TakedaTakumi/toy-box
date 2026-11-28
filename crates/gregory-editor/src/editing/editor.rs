//! 編集の状態（ケージ・選択・履歴・ドラッグ・軸の固定）と、それを変える操作。

use gregory_core::{CageModel, Preset, SelectMode, Vec3};

use super::drag::{Drag, DragEnd, DragStep, MoveSession};
use super::gizmo::{self, Axis, Constraint, Handle, ScreenPos, constraint_for_lock};
use super::history::{Command, History};
use super::selection::{EditSelection, Element};
use crate::camera::ScreenView;

/// パネル・メニュー・キーボードから来る編集の操作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditAction {
    Undo,
    Redo,
    SelectAll,
    ClearSelection,
    SetMode(SelectMode),
    /// 要素のドラッグを、軸に固定するか（`None` は自由）。
    SetAxisLock(Option<Axis>),
    /// 同じ軸なら自由に戻し、違えばその軸に固定する。
    ToggleAxisLock(Axis),
    NewShape(Preset),
}

/// パネルの表示に使う、編集の状態の要約。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditStatus {
    pub mode: SelectMode,
    pub selected: usize,
    /// 点を1つだけ選んでいるときの、その座標。
    pub single_vertex: Option<Vec3>,
    pub axis_lock: Option<Axis>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dragging: bool,
}

#[derive(Clone, Debug)]
pub struct Editor {
    cage: CageModel,
    revision: u64,
    selection: EditSelection,
    history: History,
    drag: Drag,
    axis_lock: Option<Axis>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new(Preset::Cube.make())
    }
}

impl Editor {
    pub fn new(cage: CageModel) -> Self {
        Self {
            cage,
            revision: 0,
            selection: EditSelection::default(),
            history: History::default(),
            drag: Drag::Idle,
            axis_lock: None,
        }
    }

    pub fn cage(&self) -> &CageModel {
        &self.cage
    }

    /// ケージの版番号。ケージが変わるたびに増える。
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn selection(&self) -> &EditSelection {
        &self.selection
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn drag(&self) -> &Drag {
        &self.drag
    }

    pub fn axis_lock(&self) -> Option<Axis> {
        self.axis_lock
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_active()
    }

    pub fn status(&self) -> EditStatus {
        EditStatus {
            mode: self.selection.mode(),
            selected: self.selection.len(),
            single_vertex: self
                .selection
                .single_vertex()
                .and_then(|v| self.cage.v.get(v).copied()),
            axis_lock: self.axis_lock,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            dragging: self.is_dragging(),
        }
    }

    /// 移動ハンドルの中心（選択した頂点の平均）。選択が空なら `None`。
    pub fn gizmo_center(&self) -> Option<Vec3> {
        gizmo::selection_center(&self.cage.v, &self.selection.vertices(&self.cage))
    }

    /// ハンドルでの移動中なら、つかんでいるハンドル。
    pub fn active_handle(&self) -> Option<Handle> {
        match self.drag() {
            Drag::Move(m) => m.session.handle,
            _ => None,
        }
    }

    /// 操作を行う。ドラッグ中は何もしない。状態が変わったら真。
    pub fn apply(&mut self, action: EditAction) -> bool {
        if self.is_dragging() {
            return false;
        }
        match action {
            EditAction::Undo => self.step_history(History::undo),
            EditAction::Redo => self.step_history(History::redo),
            EditAction::SelectAll => {
                self.selection.select_all(&self.cage);
                true
            }
            EditAction::ClearSelection => {
                self.selection.clear();
                true
            }
            EditAction::SetMode(mode) => {
                self.selection.set_mode(&self.cage, mode);
                true
            }
            EditAction::SetAxisLock(lock) => {
                self.axis_lock = lock;
                true
            }
            EditAction::ToggleAxisLock(axis) => {
                self.axis_lock = if self.axis_lock == Some(axis) {
                    None
                } else {
                    Some(axis)
                };
                true
            }
            EditAction::NewShape(preset) => {
                self.replace_cage(preset.make());
                true
            }
        }
    }

    /// ケージを置き換え、履歴に積む。モードは点に戻し、選択は空にする。
    pub fn replace_cage(&mut self, cage: CageModel) {
        if self.is_dragging() {
            return;
        }
        let command = Command::ReplaceCage {
            before: Box::new(self.cage.clone()),
            after: Box::new(cage),
        };
        command.apply(&mut self.cage);
        self.history.push(command);
        self.selection = EditSelection::new(SelectMode::Vertex);
        self.bump();
    }

    /// 移動ハンドルを押した。ハンドルの拘束で移動のドラッグに入る。
    pub fn press_handle(&mut self, view: &ScreenView, handle: Handle, pos: ScreenPos) {
        if self.is_dragging() || !pos.iter().all(|c| c.is_finite()) {
            return;
        }
        let mut session = MoveSession::new(
            &self.cage,
            &self.selection.vertices(&self.cage),
            Constraint::from(handle),
        );
        session.handle = Some(handle);
        if let Handle::Plane(plane) = handle {
            session.plane_origin = gizmo::plane_point(view, plane, session.center, pos);
        }
        self.drag = Drag::begin_move(pos, session);
    }

    /// 要素を押した。選択を変え、軸の固定に従う移動のドラッグに入る。実在しない要素は無視する。
    pub fn press_element(&mut self, element: Element, additive: bool, pos: ScreenPos) {
        if self.is_dragging()
            || !pos.iter().all(|c| c.is_finite())
            || !element.exists_in(&self.cage)
        {
            return;
        }
        self.selection.press(element, additive);
        let session = MoveSession::new(
            &self.cage,
            &self.selection.vertices(&self.cage),
            constraint_for_lock(self.axis_lock),
        );
        self.drag = Drag::begin_move(pos, session);
    }

    /// 空所を押した。視点を回すドラッグに入る。
    pub fn press_empty(&mut self, pos: ScreenPos, clear_on_click: bool) {
        if self.is_dragging() {
            return;
        }
        self.drag = Drag::begin_orbit(pos, clear_on_click);
    }

    /// ポインタが動いた。視点を回すときは、その量（ポイント）を返す。
    pub fn pointer_moved(&mut self, view: &ScreenView, pos: ScreenPos) -> Option<[f64; 2]> {
        match self.drag.pointer_moved(pos) {
            DragStep::Nothing => None,
            DragStep::Orbit { dx, dy } => Some([dx, dy]),
            DragStep::Move { total, pos } => {
                if let Drag::Move(m) = &self.drag {
                    let s = &m.session;
                    let delta =
                        gizmo::move_delta(view, s.constraint, s.center, total, pos, s.plane_origin);
                    s.apply(&mut self.cage, delta);
                    self.bump();
                }
                None
            }
        }
    }

    /// 指を離した。移動なら履歴に積み、空所のクリックなら選択を解除する。
    pub fn release(&mut self) {
        match self.drag.release() {
            DragEnd::Nothing => {}
            DragEnd::Click { clear_selection } => {
                if clear_selection {
                    self.selection.clear();
                }
            }
            DragEnd::Moved(session) => {
                let command = session.command(&self.cage);
                self.history.push(command);
            }
        }
    }

    /// ドラッグを取り消す。動かした頂点は開始時の位置に戻し、履歴には積まない。
    pub fn cancel_drag(&mut self) {
        if let Some(session) = self.drag.cancel() {
            session.restore(&mut self.cage);
            self.bump();
        }
    }

    fn step_history(&mut self, step: fn(&mut History, &mut CageModel) -> bool) -> bool {
        if !step(&mut self.history, &mut self.cage) {
            return false;
        }
        self.selection.clear();
        self.bump();
        true
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
