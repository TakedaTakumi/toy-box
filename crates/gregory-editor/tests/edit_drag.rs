use gregory_core::{CageModel, Preset, Vec3};
use gregory_editor::editing::drag::{
    CLICK_TOLERANCE, Drag, DragEnd, DragStep, MOVE_THRESHOLD, MoveSession,
};
use gregory_editor::editing::gizmo::{Axis, Constraint, Handle, Plane};
use gregory_editor::editing::history::Command;
use proptest::prelude::*;

fn steps() -> impl Strategy<Value = Vec<(i32, i32)>> {
    proptest::collection::vec((-6i32..=6, -6i32..=6), 0..30)
}

fn bad_pos() -> impl Strategy<Value = [f64; 2]> {
    let bad = prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)];
    (bad, -100.0f64..100.0, any::<bool>())
        .prop_map(|(b, g, first)| if first { [b, g] } else { [g, b] })
}

fn constraint() -> impl Strategy<Value = Constraint> {
    prop_oneof![
        Just(Constraint::Free),
        Just(Constraint::Axis(Axis::X)),
        Just(Constraint::Axis(Axis::Y)),
        Just(Constraint::Plane(Plane::ZX)),
    ]
}

fn session(cage: &CageModel, indices: &[usize]) -> MoveSession {
    MoveSession::new(cage, indices, Constraint::Free)
}

fn start_pos() -> [f64; 2] {
    [100.0, 100.0]
}

proptest! {
    #[test]
    fn move_drag_follows_the_threshold_model(s in steps()) {
        let cage = Preset::Cube.make();
        let mut drag = Drag::begin_move(start_pos(), session(&cage, &[0, 1]));
        let (mut pos, mut moved, mut total) = (start_pos(), 0.0f64, [0.0f64; 2]);
        let mut started = false;
        for (dx, dy) in s {
            let (dx, dy) = (f64::from(dx), f64::from(dy));
            pos = [pos[0] + dx, pos[1] + dy];
            moved += dx.abs() + dy.abs();
            let step = drag.pointer_moved(pos);
            started = moved > MOVE_THRESHOLD;
            if started {
                total = [total[0] + dx, total[1] + dy];
                prop_assert_eq!(step, DragStep::Move { total, pos });
            } else {
                prop_assert_eq!(step, DragStep::Nothing);
            }
        }
        match drag.release() {
            DragEnd::Moved(_) => prop_assert!(started),
            DragEnd::Nothing => prop_assert!(!started),
            other => prop_assert!(false, "{other:?}"),
        }
        prop_assert!(!drag.is_active());
    }

    #[test]
    fn orbit_drag_reports_each_delta_and_click_by_tolerance(s in steps(), clear in any::<bool>()) {
        let mut drag = Drag::begin_orbit(start_pos(), clear);
        let (mut pos, mut moved) = (start_pos(), 0.0f64);
        for (dx, dy) in s {
            let (dx, dy) = (f64::from(dx), f64::from(dy));
            pos = [pos[0] + dx, pos[1] + dy];
            moved += dx.abs() + dy.abs();
            prop_assert_eq!(drag.pointer_moved(pos), DragStep::Orbit { dx, dy });
        }
        let end = drag.release();
        if moved < CLICK_TOLERANCE {
            prop_assert_eq!(end, DragEnd::Click { clear_selection: clear });
        } else {
            prop_assert_eq!(end, DragEnd::Nothing);
        }
        prop_assert!(!drag.is_active());
    }

    #[test]
    fn cancel_returns_a_session_only_after_the_move_started(s in steps()) {
        let cage = Preset::Cube.make();
        let mut drag = Drag::begin_move(start_pos(), session(&cage, &[2]));
        let mut pos = start_pos();
        let mut moved = 0.0f64;
        for (dx, dy) in s {
            pos = [pos[0] + f64::from(dx), pos[1] + f64::from(dy)];
            moved += f64::from(dx.abs() + dy.abs());
            drag.pointer_moved(pos);
        }
        prop_assert_eq!(drag.cancel().is_some(), moved > MOVE_THRESHOLD);
        prop_assert_eq!(drag, Drag::Idle);
    }

    #[test]
    fn cancel_never_returns_a_session_for_orbit(s in steps()) {
        let mut drag = Drag::begin_orbit(start_pos(), true);
        let mut pos = start_pos();
        for (dx, dy) in s {
            pos = [pos[0] + f64::from(dx), pos[1] + f64::from(dy)];
            drag.pointer_moved(pos);
        }
        prop_assert!(drag.cancel().is_none());
        prop_assert!(!drag.is_active());
    }

    #[test]
    fn non_finite_pointer_positions_are_ignored(bad in bad_pos(), orbit in any::<bool>(), good in (-50.0f64..50.0, -50.0f64..50.0)) {
        let cage = Preset::Cube.make();
        let mut drag = if orbit {
            Drag::begin_orbit(start_pos(), true)
        } else {
            Drag::begin_move(start_pos(), session(&cage, &[0]))
        };
        drag.pointer_moved([start_pos()[0] + good.0, start_pos()[1] + good.1]);
        let before = drag.clone();
        prop_assert_eq!(drag.pointer_moved(bad), DragStep::Nothing);
        prop_assert_eq!(drag, before);
    }

    #[test]
    fn idle_drag_ignores_everything(pos in (-1e6f64..1e6, -1e6f64..1e6)) {
        let mut d = Drag::Idle;
        prop_assert_eq!(d.pointer_moved([pos.0, pos.1]), DragStep::Nothing);
        prop_assert_eq!(d.release(), DragEnd::Nothing);
        prop_assert!(d.cancel().is_none());
        prop_assert_eq!(d, Drag::Idle);
    }

    #[test]
    fn extreme_finite_moves_do_not_panic(a in prop_oneof![Just(f64::MAX), Just(f64::MIN), Just(1e308), -1e6f64..1e6], b in prop_oneof![Just(f64::MAX), Just(f64::MIN), -1e6f64..1e6]) {
        let cage = Preset::Cube.make();
        let mut d = Drag::begin_move([0.0, 0.0], session(&cage, &[0]));
        let _ = d.pointer_moved([a, b]);
        let _ = d.pointer_moved([b, a]);
        let _ = d.release();
    }

    #[test]
    fn session_restore_returns_to_the_start_and_command_before_is_the_start(
        indices in proptest::collection::vec(0usize..8, 1..6),
        delta in [-20.0f64..20.0, -20.0f64..20.0, -20.0f64..20.0],
    ) {
        let original = Preset::Cube.make();
        let mut cage = original.clone();
        let s = session(&cage, &indices);
        s.apply(&mut cage, delta);
        // 同じ頂点が重複して指定されても、動かした先は開始位置 + delta。
        for (&i, p) in s.indices.iter().zip(&s.start) {
            for k in 0..3 {
                prop_assert!((cage.v[i][k] - (p[k] + delta[k])).abs() < 1e-12);
            }
        }
        match s.command(&cage) {
            Command::MoveVertices { indices: ci, before, after } => {
                prop_assert_eq!(&ci, &s.indices);
                prop_assert_eq!(&before, &s.start);
                let now: Vec<Vec3> = s.indices.iter().map(|&i| cage.v[i]).collect();
                prop_assert_eq!(after, now);
            }
            other => prop_assert!(false, "{other:?}"),
        }
        s.restore(&mut cage);
        prop_assert_eq!(cage, original);
    }

    #[test]
    fn session_apply_is_relative_to_the_start_not_cumulative(d1 in [-5.0f64..5.0, -5.0f64..5.0, -5.0f64..5.0], d2 in [-5.0f64..5.0, -5.0f64..5.0, -5.0f64..5.0]) {
        let original = Preset::Cube.make();
        let mut a = original.clone();
        let mut b = original.clone();
        let s = session(&original, &[3, 4]);
        s.apply(&mut a, d1);
        s.apply(&mut a, d2);
        s.apply(&mut b, d2);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn session_apply_ignores_non_finite_delta(bad in prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)], axis in 0usize..3) {
        let original = Preset::Cube.make();
        let mut cage = original.clone();
        let mut d = [1.0, 2.0, 3.0];
        d[axis] = bad;
        session(&original, &[0, 1, 2]).apply(&mut cage, d);
        prop_assert_eq!(cage, original);
    }

    #[test]
    fn session_new_drops_out_of_range_indices(indices in proptest::collection::vec(prop_oneof![0usize..8, 8usize..100, Just(usize::MAX)], 0..10), c in constraint()) {
        let cage = Preset::Cube.make();
        let s = MoveSession::new(&cage, &indices, c);
        let want: Vec<usize> = indices.iter().copied().filter(|&i| i < 8).collect();
        prop_assert_eq!(&s.indices, &want);
        prop_assert_eq!(s.start.len(), want.len());
        prop_assert_eq!(s.constraint, c);
        prop_assert_eq!(s.handle, None);
        prop_assert_eq!(s.plane_origin, None);
        if want.is_empty() {
            prop_assert_eq!(s.center, [0.0; 3]);
        }
    }
}

#[test]
fn threshold_boundaries_are_exclusive_for_move_and_tolerance_for_click() {
    let cage = Preset::Cube.make();
    let mut d = Drag::begin_move([0.0, 0.0], session(&cage, &[0]));
    assert_eq!(d.pointer_moved([3.0, 0.0]), DragStep::Nothing);
    assert_eq!(d.clone().release(), DragEnd::Nothing);
    assert!(d.clone().cancel().is_none());
    assert!(
        matches!(d.pointer_moved([3.5, 0.0]), DragStep::Move { total, .. } if total == [0.5, 0.0])
    );

    let mut o = Drag::begin_orbit([0.0, 0.0], true);
    o.pointer_moved([3.999, 0.0]);
    assert_eq!(
        o.release(),
        DragEnd::Click {
            clear_selection: true
        }
    );
    let mut o = Drag::begin_orbit([0.0, 0.0], true);
    o.pointer_moved([4.0, 0.0]);
    assert_eq!(o.release(), DragEnd::Nothing);
    assert_eq!((MOVE_THRESHOLD, CLICK_TOLERANCE), (3.0, 4.0));
}

#[test]
fn rotation_starts_immediately_without_a_threshold() {
    let mut o = Drag::begin_orbit([0.0, 0.0], false);
    assert_eq!(
        o.pointer_moved([0.5, -0.25]),
        DragStep::Orbit { dx: 0.5, dy: -0.25 }
    );
}

#[test]
fn drag_state_accessors() {
    let cage = Preset::Cube.make();
    let mut s = session(&cage, &[0]);
    s.constraint = Constraint::Axis(Axis::Z);
    s.handle = Some(Handle::Axis(Axis::Z));
    let m = Drag::begin_move([0.0, 0.0], s);
    assert!(m.is_active() && m.is_moving());
    assert_eq!(m.move_constraint(), Some(Constraint::Axis(Axis::Z)));
    let o = Drag::begin_orbit([0.0, 0.0], false);
    assert!(o.is_active() && !o.is_moving());
    assert_eq!(o.move_constraint(), None);
    assert!(!Drag::Idle.is_active());
}

#[test]
fn session_center_is_the_mean_of_the_start_positions() {
    let cage = Preset::Cube.make();
    let s = session(&cage, &[0, 1]);
    let want: Vec3 = std::array::from_fn(|k| (cage.v[0][k] + cage.v[1][k]) / 2.0);
    assert_eq!(s.center, want);
}

#[test]
fn command_after_falls_back_to_start_when_the_cage_shrank() {
    let cage = Preset::Cube.make();
    let s = session(&cage, &[5, 7]);
    let small = CageModel {
        v: vec![[0.0; 3]],
        ..CageModel::default()
    };
    match s.command(&small) {
        Command::MoveVertices { after, before, .. } => assert_eq!(after, before),
        other => panic!("{other:?}"),
    }
}

fn any_bad_pos() -> impl Strategy<Value = [f64; 2]> {
    let bad = prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)];
    (bad.clone(), bad, -100.0f64..100.0, 0u8..3).prop_map(|(a, b, g, k)| match k {
        0 => [a, b],
        1 => [a, g],
        _ => [g, b],
    })
}

fn finite_delta() -> impl Strategy<Value = Vec<[f64; 2]>> {
    proptest::collection::vec([-50.0f64..50.0, -50.0f64..50.0], 1..20)
}

proptest! {
    // 押した位置が非有限でも、有限の位置への移動で壊れた差分が出ないことを確かめる。
    #[test]
    fn dragging_from_a_non_finite_press_never_yields_non_finite_steps(
        bad in any_bad_pos(),
        orbit in any::<bool>(),
        path in finite_delta(),
    ) {
        let cage = Preset::Cube.make();
        let mut drag = if orbit {
            Drag::begin_orbit(bad, true)
        } else {
            Drag::begin_move(bad, session(&cage, &[0]))
        };
        for p in path {
            match drag.pointer_moved(p) {
                DragStep::Nothing => {}
                DragStep::Orbit { dx, dy } => prop_assert!(dx.is_finite() && dy.is_finite()),
                DragStep::Move { total, pos } => {
                    prop_assert!(total.iter().chain(&pos).all(|x| x.is_finite()));
                }
            }
            match &drag {
                Drag::Orbit(o) => prop_assert!(!o.moved.is_nan()),
                Drag::Move(m) => prop_assert!(!m.moved.is_nan()),
                Drag::Idle => {}
            }
        }
    }

    #[test]
    fn releasing_a_non_finite_press_without_moving_is_a_click_or_nothing(
        bad in any_bad_pos(),
        clear in any::<bool>(),
    ) {
        let cage = Preset::Cube.make();
        prop_assert_eq!(
            Drag::begin_orbit(bad, clear).release(),
            DragEnd::Click { clear_selection: clear }
        );
        prop_assert_eq!(Drag::begin_move(bad, session(&cage, &[0])).release(), DragEnd::Nothing);
    }
}
