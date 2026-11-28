//! 3D ビューの上に egui で重ねて描くもの（移動ハンドル）。

use egui::{Align2, Color32, FontId, Painter, Pos2, Shape, Stroke, pos2};

use crate::editing::gizmo::{Axis, Gizmo, Handle, Plane, ScreenPos};
use crate::palette;

fn axis_color(axis: Axis) -> Color32 {
    match axis {
        Axis::X => palette::AXIS_X,
        Axis::Y => palette::AXIS_Y,
        Axis::Z => palette::AXIS_Z,
    }
}

/// 移動ハンドルを描く。`lit` は、ホバー中かつかんでいるハンドル（選択の色にする）。
pub fn paint_gizmo(painter: &Painter, origin: Pos2, gizmo: &Gizmo, lit: Option<Handle>) {
    let at = |p: ScreenPos| pos2(origin.x + p[0] as f32, origin.y + p[1] as f32);
    for (plane, quad) in Plane::ALL.into_iter().zip(&gizmo.planes) {
        let on = lit == Some(Handle::Plane(plane));
        let base = if on {
            palette::SELECTION_UI
        } else {
            axis_color(plane.normal())
        };
        let alpha = if on { 0.75 } else { 0.4 };
        painter.add(Shape::convex_polygon(
            quad.iter().map(|&p| at(p)).collect(),
            base.gamma_multiply(alpha),
            Stroke::new(1.0, base),
        ));
    }
    let center = at(gizmo.origin);
    for axis in Axis::ALL {
        let tip = at(gizmo.tip(axis));
        let color = if lit == Some(Handle::Axis(axis)) {
            palette::SELECTION_UI
        } else {
            axis_color(axis)
        };
        let dir = tip - center;
        let length = dir.length();
        let unit = if length > 0.0 {
            dir / length
        } else {
            egui::vec2(1.0, 0.0)
        };
        let side = egui::vec2(-unit.y, unit.x);
        painter.line_segment([center, tip - unit * 8.0], Stroke::new(3.0, color));
        painter.add(Shape::convex_polygon(
            vec![
                tip,
                tip - unit * 14.0 + side * 5.5,
                tip - unit * 14.0 - side * 5.5,
            ],
            color,
            Stroke::NONE,
        ));
        painter.text(
            tip + unit * 13.0,
            Align2::CENTER_CENTER,
            axis.label(),
            FontId::monospace(12.0),
            color,
        );
    }
    painter.circle_filled(center, 4.0, palette::GIZMO_CENTER);
}
