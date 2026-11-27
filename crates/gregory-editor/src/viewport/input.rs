//! egui の入力を、3D ビューのポインタ操作に直す。

use egui::{Event, InputState, PointerButton, Rect, TouchPhase};

/// 3D ビューのポインタ操作。位置はビューの左上を原点とするポイント。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerInput {
    Press {
        pos: [f64; 2],
        /// 右・中ボタンか Alt 付きなら、要素やハンドルの上でも視点を回す。
        force_orbit: bool,
        /// Shift・Ctrl・Cmd 付き（選択に足す・外す）。
        additive: bool,
    },
    Move([f64; 2]),
    Release,
    /// ピンチ（2本目の指）、タッチの取り消し、ウィンドウのフォーカスが外れたとき。
    Cancel,
}

/// このフレームのポインタ操作を、起きた順に並べる。`accept_press` が偽なら押下を無視する（他の部品の上など）。
pub fn collect(
    input: &InputState,
    rect: Rect,
    accept_press: bool,
    dragging: bool,
) -> Vec<PointerInput> {
    let local = |p: egui::Pos2| [f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y)];
    let mut out = Vec::new();
    let mut active = dragging;
    for event in &input.events {
        match event {
            Event::PointerMoved(p) => out.push(PointerInput::Move(local(*p))),
            Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers,
            } if !active && accept_press && rect.contains(*pos) => {
                let force_orbit =
                    matches!(button, PointerButton::Secondary | PointerButton::Middle)
                        || modifiers.alt;
                let additive =
                    modifiers.shift || modifiers.ctrl || modifiers.command || modifiers.mac_cmd;
                out.push(PointerInput::Press {
                    pos: local(*pos),
                    force_orbit,
                    additive,
                });
                active = true;
            }
            Event::PointerButton { pressed: false, .. } if active => {
                out.push(PointerInput::Release);
                active = false;
            }
            Event::Touch {
                phase: TouchPhase::Cancel,
                ..
            }
            | Event::WindowFocused(false)
                if active =>
            {
                out.push(PointerInput::Cancel);
                active = false;
            }
            _ => {}
        }
    }
    if active && input.multi_touch().is_some() {
        out.push(PointerInput::Cancel);
        active = false;
    }
    // 離した知らせを取りこぼしても、ドラッグが残り続けないようにする
    if active && !input.pointer.any_down() {
        out.push(PointerInput::Release);
    }
    out
}
