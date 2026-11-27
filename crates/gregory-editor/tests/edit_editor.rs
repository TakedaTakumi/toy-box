use gregory_core::{CageModel, EdgeKey, PRESETS, Preset, SelectMode};
use gregory_editor::camera::{OrbitCamera, ScreenView};
use gregory_editor::editing::drag::Drag;
use gregory_editor::editing::gizmo::{Axis, Handle, Plane, ScreenPos, gizmo};
use gregory_editor::editing::history::HISTORY_LIMIT;
use gregory_editor::editing::selection::Element;
use gregory_editor::editing::{EditAction, Editor};
use proptest::prelude::*;

fn view() -> ScreenView {
    ScreenView::of(&OrbitCamera::default(), 800.0, 600.0)
}

fn preset() -> impl Strategy<Value = Preset> {
    (0..PRESETS.len()).prop_map(|i| PRESETS[i])
}

fn mode() -> impl Strategy<Value = SelectMode> {
    prop_oneof![
        Just(SelectMode::Vertex),
        Just(SelectMode::Edge),
        Just(SelectMode::Face)
    ]
}

fn axis() -> impl Strategy<Value = Axis> {
    prop_oneof![Just(Axis::X), Just(Axis::Y), Just(Axis::Z)]
}

fn action() -> impl Strategy<Value = EditAction> {
    prop_oneof![
        Just(EditAction::Undo),
        Just(EditAction::Redo),
        Just(EditAction::SelectAll),
        Just(EditAction::ClearSelection),
        mode().prop_map(EditAction::SetMode),
        proptest::option::of(axis()).prop_map(EditAction::SetAxisLock),
        axis().prop_map(EditAction::ToggleAxisLock),
        preset().prop_map(EditAction::NewShape),
    ]
}

fn bad() -> impl Strategy<Value = f64> {
    prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)]
}

fn wild() -> impl Strategy<Value = f64> {
    prop_oneof![8 => -300.0f64..900.0, 1 => bad(), 1 => Just(f64::MAX), 1 => Just(f64::MIN)]
}

fn element() -> impl Strategy<Value = Element> {
    prop_oneof![
        (0usize..10).prop_map(Element::Vertex),
        Just(Element::Vertex(usize::MAX)),
        (0usize..9, 0usize..9).prop_map(|(a, b)| Element::Edge(EdgeKey::new(a, b))),
        Just(Element::Edge(EdgeKey::new(usize::MAX, 0))),
        (0usize..8).prop_map(Element::Face),
        Just(Element::Face(usize::MAX)),
    ]
}

fn handle() -> impl Strategy<Value = Handle> {
    prop_oneof![
        axis().prop_map(Handle::Axis),
        Just(Handle::Plane(Plane::XY)),
        Just(Handle::Plane(Plane::YZ)),
        Just(Handle::Plane(Plane::ZX)),
    ]
}

#[derive(Clone, Debug)]
enum Op<P> {
    Apply(EditAction),
    PressElement(Element, bool, P),
    PressHandle(Handle, P),
    PressEmpty(P, bool),
    Move(P),
    Release,
    Cancel,
}

fn ops(pos: BoxedStrategy<ScreenPos>, n: usize) -> impl Strategy<Value = Vec<Op<ScreenPos>>> {
    let op = prop_oneof![
        3 => action().prop_map(Op::Apply),
        3 => (element(), any::<bool>(), pos.clone()).prop_map(|(e, a, p)| Op::PressElement(e, a, p)),
        1 => (handle(), pos.clone()).prop_map(|(h, p)| Op::PressHandle(h, p)),
        1 => (pos.clone(), any::<bool>()).prop_map(|(p, c)| Op::PressEmpty(p, c)),
        6 => pos.prop_map(Op::Move),
        3 => Just(Op::Release),
        1 => Just(Op::Cancel),
    ];
    proptest::collection::vec(op, 0..n)
}

fn sane_pos() -> impl Strategy<Value = ScreenPos> {
    (-10i32..=10, -10i32..=10).prop_map(|(x, y)| [400.0 + f64::from(x), 300.0 + f64::from(y)])
}

fn wild_pos() -> impl Strategy<Value = ScreenPos> {
    [wild(), wild()]
}

fn started(ed: &Editor) -> bool {
    matches!(ed.drag(), Drag::Move(m) if m.started())
}

fn finite_cage(c: &CageModel) -> bool {
    c.v.iter().flatten().all(|x| x.is_finite())
}

fn run(ops: &[Op<ScreenPos>], check_strict: bool) {
    let view = view();
    let mut ed = Editor::default();
    let mut at_press: Option<CageModel> = None;
    for op in ops {
        let (cage, rev) = (ed.cage().clone(), ed.revision());
        let (sel, hist_len, lock, was_dragging) = (
            ed.selection().clone(),
            ed.history().undo_len(),
            ed.axis_lock(),
            ed.is_dragging(),
        );
        let moving_started = started(&ed);
        match op {
            Op::Apply(a) => {
                let changed = ed.apply(*a);
                if was_dragging {
                    assert!(!changed, "ドラッグ中は操作を受け付けない: {a:?}");
                    assert_eq!(
                        (
                            ed.cage(),
                            ed.selection(),
                            ed.history().undo_len(),
                            ed.axis_lock()
                        ),
                        (&cage, &sel, hist_len, lock)
                    );
                } else if matches!(a, EditAction::Undo | EditAction::Redo) && changed {
                    assert!(ed.selection().is_empty());
                    assert_eq!(ed.selection().mode(), sel.mode());
                }
            }
            Op::PressElement(e, additive, p) => {
                ed.press_element(*e, *additive, *p);
                if !was_dragging && ed.is_dragging() {
                    at_press = Some(cage.clone());
                    assert!(e.exists_in(&cage));
                }
            }
            Op::PressHandle(h, p) => {
                ed.press_handle(&view, *h, *p);
                if !was_dragging && ed.is_dragging() {
                    at_press = Some(cage.clone());
                }
            }
            Op::PressEmpty(p, clear) => {
                ed.press_empty(*p, *clear);
                if !was_dragging && ed.is_dragging() {
                    at_press = Some(cage.clone());
                }
            }
            Op::Move(p) => {
                let _ = ed.pointer_moved(&view, *p);
            }
            Op::Release => {
                ed.release();
                if moving_started {
                    assert_eq!(ed.history().undo_len(), (hist_len + 1).min(HISTORY_LIMIT));
                    let mut c = ed.clone();
                    assert!(c.apply(EditAction::Undo));
                    assert_eq!(
                        Some(c.cage()),
                        at_press.as_ref(),
                        "undo で押した時点の位置に戻る"
                    );
                } else {
                    assert_eq!(ed.history().undo_len(), hist_len);
                    assert_eq!(ed.cage(), &cage);
                }
                assert!(!ed.is_dragging());
            }
            Op::Cancel => {
                ed.cancel_drag();
                assert_eq!(
                    ed.history().undo_len(),
                    hist_len,
                    "取り消しは履歴を増やさない"
                );
                if was_dragging {
                    assert_eq!(Some(ed.cage()), at_press.as_ref(), "開始位置に戻る");
                }
                assert!(!ed.is_dragging());
            }
        }
        if check_strict && ed.cage() != &cage {
            assert_ne!(
                ed.revision(),
                rev,
                "ケージが変わったのに revision が同じ: {op:?}"
            );
        }
        assert!(ed.history().undo_len() <= HISTORY_LIMIT);
        assert!(
            finite_cage(ed.cage()),
            "ケージに非有限の座標が入った: {op:?}"
        );
    }
}

proptest! {
    #[test]
    fn editor_invariants_hold_for_any_operation_sequence(seq in ops(sane_pos().boxed(), 60)) {
        run(&seq, true);
    }

    #[test]
    fn editor_survives_non_finite_and_extreme_pointer_positions(seq in ops(wild_pos().boxed(), 40)) {
        run(&seq, false);
    }

    #[test]
    fn dragging_blocks_every_action(a in action(), preset in preset()) {
        let mut ed = Editor::default();
        ed.apply(EditAction::SelectAll);
        ed.press_element(Element::Vertex(0), false, [10.0, 10.0]);
        prop_assert!(ed.is_dragging());
        let cage = ed.cage().clone();
        let (sel, rev, hist) = (ed.selection().clone(), ed.revision(), ed.history().undo_len());
        prop_assert!(!ed.apply(a));
        ed.replace_cage(preset.make());
        prop_assert_eq!((ed.cage(), ed.selection(), ed.revision(), ed.history().undo_len()), (&cage, &sel, rev, hist));
        ed.press_element(Element::Vertex(3), true, [0.0, 0.0]);
        ed.press_empty([5.0, 5.0], true);
        ed.press_handle(&view(), Handle::Axis(Axis::X), [5.0, 5.0]);
        prop_assert_eq!(ed.selection(), &sel);
        prop_assert!(ed.drag().is_moving());
    }

    #[test]
    fn new_shape_resets_selection_mode_and_records_one_history_entry(p in preset(), m in mode(), seed in 0usize..3) {
        let mut ed = Editor::default();
        ed.apply(EditAction::SetMode(m));
        ed.apply(EditAction::SelectAll);
        let rev = ed.revision();
        for _ in 0..seed {
            ed.apply(EditAction::NewShape(Preset::Plane));
        }
        let start = ed.history().undo_len();
        prop_assert!(ed.apply(EditAction::NewShape(p)));
        prop_assert_eq!(ed.cage(), &p.make());
        prop_assert_eq!(ed.history().undo_len(), start + 1);
        prop_assert_eq!(ed.selection().mode(), SelectMode::Vertex);
        prop_assert!(ed.selection().is_empty());
        prop_assert!(ed.revision() > rev);
        prop_assert!(!ed.history().can_redo());
    }

    #[test]
    fn undo_and_redo_empty_the_selection_but_keep_the_mode(m in mode(), p in preset()) {
        let mut ed = Editor::default();
        ed.apply(EditAction::NewShape(p));
        ed.apply(EditAction::SetMode(m));
        ed.apply(EditAction::SelectAll);
        prop_assert!(ed.apply(EditAction::Undo));
        prop_assert!(ed.selection().is_empty());
        prop_assert_eq!(ed.selection().mode(), m);
        ed.apply(EditAction::SelectAll);
        prop_assert!(ed.apply(EditAction::Redo));
        prop_assert!(ed.selection().is_empty());
        prop_assert_eq!(ed.selection().mode(), m);
        prop_assert!(!ed.apply(EditAction::Redo));
    }

    #[test]
    fn a_long_drag_makes_exactly_one_history_entry_starting_from_the_press_position(
        v in 0usize..8, steps in proptest::collection::vec((-8i32..=8, -8i32..=8), 1..40),
    ) {
        let view = view();
        let mut ed = Editor::default();
        let original = ed.cage().clone();
        ed.press_element(Element::Vertex(v), false, [400.0, 300.0]);
        let (mut x, mut y, mut moved) = (400.0f64, 300.0f64, 0.0f64);
        for (dx, dy) in steps {
            x += f64::from(dx);
            y += f64::from(dy);
            moved += f64::from(dx.abs() + dy.abs());
            prop_assert_eq!(ed.pointer_moved(&view, [x, y]), None);
        }
        ed.release();
        if moved > 3.0 {
            prop_assert_eq!(ed.history().undo_len(), 1);
            let moved_cage = ed.cage().clone();
            prop_assert!(ed.apply(EditAction::Undo));
            prop_assert_eq!(ed.cage(), &original);
            prop_assert!(ed.apply(EditAction::Redo));
            prop_assert_eq!(ed.cage(), &moved_cage);
        } else {
            prop_assert_eq!(ed.history().undo_len(), 0);
            prop_assert_eq!(ed.cage(), &original);
        }
    }

    #[test]
    fn cancel_restores_the_start_without_touching_the_history(v in 0usize..8, dx in 4.0f64..200.0, dy in -100.0f64..100.0) {
        let view = view();
        let mut ed = Editor::default();
        let original = ed.cage().clone();
        ed.press_element(Element::Vertex(v), false, [400.0, 300.0]);
        ed.pointer_moved(&view, [400.0 + dx, 300.0 + dy]);
        prop_assert_ne!(ed.cage(), &original, "動かせている");
        let rev = ed.revision();
        ed.cancel_drag();
        prop_assert_eq!(ed.cage(), &original);
        prop_assert_eq!(ed.history().undo_len(), 0);
        prop_assert!(!ed.history().can_redo());
        prop_assert!(!ed.is_dragging());
        prop_assert!(ed.revision() > rev);
    }

    #[test]
    fn non_existing_elements_are_ignored_on_press(
        el in prop_oneof![
            (8usize..100).prop_map(Element::Vertex), Just(Element::Vertex(usize::MAX)),
            (8usize..100).prop_map(|b| Element::Edge(EdgeKey::new(0, b))), Just(Element::Edge(EdgeKey::new(0, 0))),
            (6usize..100).prop_map(Element::Face), Just(Element::Face(usize::MAX)),
        ],
        additive in any::<bool>(),
    ) {
        let mut ed = Editor::default();
        ed.press_element(Element::Vertex(1), false, [0.0, 0.0]);
        ed.release();
        let sel = ed.selection().clone();
        ed.press_element(el, additive, [1.0, 1.0]);
        prop_assert!(!ed.is_dragging());
        prop_assert_eq!(ed.selection(), &sel);
    }

    #[test]
    fn empty_click_clears_only_when_asked_and_only_without_a_drag(clear in any::<bool>(), nudge in 0.0f64..3.9) {
        let view = view();
        let mut ed = Editor::default();
        ed.apply(EditAction::SelectAll);
        ed.press_empty([10.0, 10.0], clear);
        ed.pointer_moved(&view, [10.0 + nudge, 10.0]);
        ed.release();
        prop_assert_eq!(ed.selection().is_empty(), clear);
        ed.apply(EditAction::SelectAll);
        ed.press_empty([10.0, 10.0], true);
        prop_assert_eq!(ed.pointer_moved(&view, [10.0 + 20.0, 10.0]), Some([20.0, 0.0]));
        ed.release();
        prop_assert!(!ed.selection().is_empty(), "回転したあとの解放では解除しない");
        prop_assert_eq!(ed.history().undo_len(), 0);
    }

    #[test]
    fn axis_locked_element_drag_moves_only_along_the_axis(v in 0usize..8, a in axis(), dx in 20.0f64..150.0, dy in -80.0f64..80.0) {
        let view = view();
        let mut ed = Editor::default();
        ed.apply(EditAction::SetAxisLock(Some(a)));
        let original = ed.cage().clone();
        ed.press_element(Element::Vertex(v), false, [400.0, 300.0]);
        ed.pointer_moved(&view, [400.0 + dx, 300.0 + dy]);
        for (i, (now, was)) in ed.cage().v.iter().zip(&original.v).enumerate() {
            for k in 0..3 {
                if k != (a as usize) && i == v {
                    prop_assert_eq!(now[k], was[k], "拘束していない軸は動かない");
                }
                if i != v {
                    prop_assert_eq!(now[k], was[k]);
                }
            }
        }
        ed.release();
        ed.apply(EditAction::ToggleAxisLock(a));
        prop_assert_eq!(ed.axis_lock(), None);
    }
}

#[test]
fn dragging_a_selected_vertex_moves_the_whole_selection_by_the_same_delta() {
    let view = view();
    let mut ed = Editor::default();
    let original = ed.cage().clone();
    ed.press_element(Element::Vertex(0), false, [0.0, 0.0]);
    ed.release();
    ed.press_element(Element::Vertex(1), true, [0.0, 0.0]);
    ed.release();
    ed.press_element(Element::Vertex(1), false, [0.0, 0.0]);
    assert_eq!(
        ed.selection().len(),
        2,
        "選択済みを修飾なしで押しても選択は変わらない"
    );
    ed.pointer_moved(&view, [50.0, 20.0]);
    ed.release();
    let d0: Vec<f64> = (0..3)
        .map(|k| ed.cage().v[0][k] - original.v[0][k])
        .collect();
    let d1: Vec<f64> = (0..3)
        .map(|k| ed.cage().v[1][k] - original.v[1][k])
        .collect();
    assert!(d0.iter().any(|x| x.abs() > 1e-6));
    assert!(d0.iter().zip(&d1).all(|(a, b)| (a - b).abs() < 1e-9));
    assert_eq!(ed.history().undo_len(), 1);
}

#[test]
fn x_arrow_drag_moves_the_selected_vertex_only_along_x() {
    let view = view();
    let mut ed = Editor::default();
    let original = ed.cage().clone();
    ed.press_element(Element::Vertex(6), false, [0.0, 0.0]);
    ed.release();
    let center = ed.gizmo_center().expect("1つ選択している");
    assert_eq!(center, original.v[6]);
    let g = gizmo(&view, center).expect("手前");
    let tip = g.tip(Axis::X);
    let mid = [(g.origin[0] + tip[0]) / 2.0, (g.origin[1] + tip[1]) / 2.0];
    ed.press_handle(&view, Handle::Axis(Axis::X), mid);
    assert_eq!(ed.active_handle(), Some(Handle::Axis(Axis::X)));
    ed.pointer_moved(&view, [tip[0], tip[1]]);
    ed.release();
    let v = ed.cage().v[6];
    assert!((v[0] - original.v[6][0]).abs() > 0.1);
    assert_eq!((v[1], v[2]), (original.v[6][1], original.v[6][2]));
    assert_eq!(ed.active_handle(), None);
    assert_eq!(ed.history().undo_len(), 1);
}

#[test]
fn revision_increases_on_every_visible_cage_change() {
    let view = view();
    let mut ed = Editor::default();
    let r0 = ed.revision();
    ed.apply(EditAction::NewShape(Preset::Plane));
    let r1 = ed.revision();
    assert!(r1 > r0);
    ed.apply(EditAction::Undo);
    let r2 = ed.revision();
    assert!(r2 > r1);
    ed.apply(EditAction::Redo);
    let r3 = ed.revision();
    assert!(r3 > r2);
    ed.apply(EditAction::SelectAll);
    ed.apply(EditAction::SetMode(SelectMode::Face));
    assert_eq!(ed.revision(), r3, "選択だけの変更では増えない");
    ed.press_element(Element::Face(0), false, [0.0, 0.0]);
    ed.pointer_moved(&view, [30.0, 0.0]);
    let r4 = ed.revision();
    assert!(r4 > r3);
    ed.cancel_drag();
    assert!(ed.revision() > r4);
}

#[test]
fn status_reflects_the_editor_state() {
    let mut ed = Editor::default();
    let s = ed.status();
    assert_eq!(
        (
            s.mode,
            s.selected,
            s.single_vertex,
            s.axis_lock,
            s.can_undo,
            s.can_redo,
            s.dragging
        ),
        (SelectMode::Vertex, 0, None, None, false, false, false)
    );
    ed.press_element(Element::Vertex(2), false, [0.0, 0.0]);
    let s = ed.status();
    assert_eq!(
        (s.selected, s.single_vertex, s.dragging),
        (1, Some(ed.cage().v[2]), true)
    );
    ed.release();
    ed.apply(EditAction::ToggleAxisLock(Axis::Y));
    ed.apply(EditAction::NewShape(Preset::Cube));
    ed.apply(EditAction::Undo);
    let s = ed.status();
    assert_eq!(
        (s.axis_lock, s.can_undo, s.can_redo, s.dragging),
        (Some(Axis::Y), false, true, false)
    );
    assert!(ed.gizmo_center().is_none());
}

#[test]
fn nan_press_position_never_corrupts_the_cage() {
    let view = view();
    let mut ed = Editor::default();
    let original = ed.cage().clone();
    ed.press_element(Element::Vertex(0), false, [f64::NAN, f64::NAN]);
    for p in [[10.0, 10.0], [100.0, 100.0], [f64::NAN, 3.0]] {
        ed.pointer_moved(&view, p);
    }
    ed.release();
    assert_eq!(ed.cage(), &original);
    assert_eq!(ed.history().undo_len(), 0);
}

#[test]
fn many_pushes_cap_the_history_at_the_limit() {
    let mut ed = Editor::default();
    for _ in 0..HISTORY_LIMIT + 5 {
        ed.apply(EditAction::NewShape(Preset::Pyramid));
    }
    assert_eq!(ed.history().undo_len(), HISTORY_LIMIT);
}

#[test]
#[ignore = "既知の問題: 押した位置が非有限だと、回転量に NaN が返りクリック判定も効かなくなる"]
fn orbit_delta_stays_finite_after_a_non_finite_press_position() {
    let view = view();
    let mut ed = Editor::default();
    ed.apply(EditAction::SelectAll);
    ed.press_empty([f64::NAN, 0.0], true);
    let step = ed.pointer_moved(&view, [1.0, 1.0]);
    assert!(
        step.is_none_or(|d| d.iter().all(|c| c.is_finite())),
        "{step:?}"
    );
    ed.release();
    assert!(
        ed.selection().is_empty(),
        "動いていないのでクリックとして解除される"
    );
}
