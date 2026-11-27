use egui::{Key, Modifiers};
use gregory_core::SelectMode;
use gregory_editor::editing::EditAction;
use gregory_editor::editing::gizmo::Axis;
use gregory_editor::shortcuts::{Shortcut, shortcut};
use proptest::prelude::*;

fn key() -> impl Strategy<Value = Key> {
    (0..Key::ALL.len()).prop_map(|i| Key::ALL[i])
}

fn modifiers() -> impl Strategy<Value = Modifiers> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(alt, ctrl, shift, mac_cmd, command)| Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        })
}

fn edit(a: EditAction) -> Option<Shortcut> {
    Some(Shortcut::Edit(a))
}

fn with(f: impl FnOnce(&mut Modifiers)) -> Modifiers {
    let mut m = Modifiers::NONE;
    f(&mut m);
    m
}

/// 実装を見ずに仕様の文面から書いた期待値。
fn expected(key: Key, m: Modifiers) -> Option<Shortcut> {
    if m.alt {
        return None;
    }
    if m.ctrl || m.command || m.mac_cmd {
        return match (key, m.shift) {
            (Key::Z, true) | (Key::Y, _) => edit(EditAction::Redo),
            (Key::Z, false) => edit(EditAction::Undo),
            (Key::A, _) => edit(EditAction::SelectAll),
            _ => None,
        };
    }
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

proptest! {
    #[test]
    fn every_key_and_modifier_combination_matches_the_spec(k in key(), m in modifiers()) {
        prop_assert_eq!(shortcut(k, m), expected(k, m));
    }

    #[test]
    fn alt_always_disables_the_shortcut(k in key(), mut m in modifiers()) {
        m.alt = true;
        prop_assert_eq!(shortcut(k, m), None);
    }

    #[test]
    fn ctrl_command_and_mac_cmd_are_interchangeable(k in key(), shift in any::<bool>()) {
        let base = shortcut(k, with(|m| { m.ctrl = true; m.shift = shift; }));
        prop_assert_eq!(shortcut(k, with(|m| { m.command = true; m.shift = shift; })), base);
        prop_assert_eq!(shortcut(k, with(|m| { m.mac_cmd = true; m.shift = shift; })), base);
    }
}

#[test]
fn undo_redo_select_all() {
    for command in [Modifiers::CTRL, Modifiers::COMMAND, Modifiers::MAC_CMD] {
        assert_eq!(shortcut(Key::Z, command), edit(EditAction::Undo));
        assert_eq!(
            shortcut(Key::Z, command | Modifiers::SHIFT),
            edit(EditAction::Redo)
        );
        assert_eq!(shortcut(Key::Y, command), edit(EditAction::Redo));
        assert_eq!(shortcut(Key::A, command), edit(EditAction::SelectAll));
        assert_eq!(shortcut(Key::Num1, command), None);
        assert_eq!(shortcut(Key::S, command), None);
    }
}

#[test]
fn plain_keys() {
    let n = Modifiers::NONE;
    assert_eq!(
        shortcut(Key::Num1, n),
        edit(EditAction::SetMode(SelectMode::Vertex))
    );
    assert_eq!(
        shortcut(Key::Num2, n),
        edit(EditAction::SetMode(SelectMode::Edge))
    );
    assert_eq!(
        shortcut(Key::Num3, n),
        edit(EditAction::SetMode(SelectMode::Face))
    );
    assert_eq!(shortcut(Key::Escape, n), edit(EditAction::ClearSelection));
    assert_eq!(
        shortcut(Key::X, n),
        edit(EditAction::ToggleAxisLock(Axis::X))
    );
    assert_eq!(
        shortcut(Key::Y, n),
        edit(EditAction::ToggleAxisLock(Axis::Y))
    );
    assert_eq!(
        shortcut(Key::Z, n),
        edit(EditAction::ToggleAxisLock(Axis::Z))
    );
    assert_eq!(shortcut(Key::S, n), Some(Shortcut::ToggleShading));
    assert_eq!(shortcut(Key::Num4, n), None);
    assert_eq!(shortcut(Key::Num0, n), None);
    assert_eq!(shortcut(Key::Enter, n), None);
}

#[test]
fn alt_blocks_even_with_command() {
    assert_eq!(shortcut(Key::Z, Modifiers::CTRL | Modifiers::ALT), None);
    assert_eq!(shortcut(Key::Num1, Modifiers::ALT), None);
    assert_eq!(shortcut(Key::S, Modifiers::ALT | Modifiers::SHIFT), None);
}
