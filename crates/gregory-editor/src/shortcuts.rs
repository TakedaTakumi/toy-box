//! キーボードのショートカットの割り当て。

use egui::{Key, Modifiers};
use gregory_core::SelectMode;

use crate::editing::EditAction;
use crate::editing::gizmo::Axis;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    Edit(EditAction),
    ToggleShading,
}

/// 押したキーと修飾キーから、ショートカットを決める。Ctrl と Mac の Cmd は同じに扱う。
pub fn shortcut(key: Key, modifiers: Modifiers) -> Option<Shortcut> {
    let command = modifiers.command || modifiers.ctrl || modifiers.mac_cmd;
    if modifiers.alt {
        return None;
    }
    if command {
        return match key {
            Key::Z if modifiers.shift => Some(Shortcut::Edit(EditAction::Redo)),
            Key::Z => Some(Shortcut::Edit(EditAction::Undo)),
            Key::Y => Some(Shortcut::Edit(EditAction::Redo)),
            Key::A => Some(Shortcut::Edit(EditAction::SelectAll)),
            _ => None,
        };
    }
    let edit = |action| Some(Shortcut::Edit(action));
    match key {
        Key::Num1 => edit(EditAction::SetMode(SelectMode::Vertex)),
        Key::Num2 => edit(EditAction::SetMode(SelectMode::Edge)),
        Key::Num3 => edit(EditAction::SetMode(SelectMode::Face)),
        Key::Escape => edit(EditAction::ClearSelection),
        Key::X => edit(EditAction::ToggleAxisLock(Axis::X)),
        Key::Y => edit(EditAction::ToggleAxisLock(Axis::Y)),
        Key::Z => edit(EditAction::ToggleAxisLock(Axis::Z)),
        Key::S => Some(Shortcut::ToggleShading),
        _ => None,
    }
}
