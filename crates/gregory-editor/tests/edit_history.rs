use gregory_core::{CageModel, PRESETS, Preset, Vec3};
use gregory_editor::editing::history::{Command, HISTORY_LIMIT, History, set_positions};
use proptest::prelude::*;

fn finite() -> impl Strategy<Value = f64> {
    -50.0f64..50.0
}

fn vec3() -> impl Strategy<Value = Vec3> {
    [finite(), finite(), finite()]
}

fn preset() -> impl Strategy<Value = Preset> {
    (0..PRESETS.len()).prop_map(|i| PRESETS[i])
}

fn weird_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        finite(),
        Just(f64::NAN),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        Just(f64::MAX)
    ]
}

fn weird_index() -> impl Strategy<Value = usize> {
    prop_oneof![0usize..8, 8usize..50, Just(usize::MAX)]
}

/// 現在のケージの位置を before にした移動。範囲外の番号は除く。
fn move_from(cage: &CageModel, indices: &[usize], after: &[Vec3]) -> Command {
    let indices: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&i| i < cage.v.len())
        .collect();
    let before = indices.iter().map(|&i| cage.v[i]).collect();
    let after = (0..indices.len()).map(|k| after[k % after.len()]).collect();
    Command::MoveVertices {
        indices,
        before,
        after,
    }
}

#[derive(Clone, Debug)]
enum Op {
    Move(Vec<usize>, Vec<Vec3>),
    Replace(Preset),
    Undo,
    Redo,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (proptest::collection::vec(0usize..8, 1..5), proptest::collection::vec(vec3(), 1..5)).prop_map(|(i, a)| Op::Move(i, a)),
        1 => preset().prop_map(Op::Replace),
        3 => Just(Op::Undo),
        3 => Just(Op::Redo),
    ]
}

fn run(ops: &[Op]) {
    let initial = Preset::Cube.make();
    let mut cage = initial.clone();
    let mut history = History::default();
    let mut states = vec![initial];
    let mut pos = 0usize;
    for op in ops {
        match op {
            Op::Move(idx, after) => {
                let mut c = move_from(&cage, idx, after);
                if matches!(c, Command::MoveVertices { ref indices, .. } if indices.is_empty()) {
                    c = move_from(&cage, &[0], after);
                }
                c.apply(&mut cage);
                history.push(c);
                states.truncate(pos + 1);
                states.push(cage.clone());
                if states.len() > HISTORY_LIMIT + 1 {
                    states.remove(0);
                }
                pos = states.len() - 1;
            }
            Op::Replace(p) => {
                let c = Command::ReplaceCage {
                    before: Box::new(cage.clone()),
                    after: Box::new(p.make()),
                };
                c.apply(&mut cage);
                history.push(c);
                states.truncate(pos + 1);
                states.push(cage.clone());
                if states.len() > HISTORY_LIMIT + 1 {
                    states.remove(0);
                }
                pos = states.len() - 1;
            }
            Op::Undo => {
                let ok = history.undo(&mut cage);
                assert_eq!(ok, pos > 0);
                pos = pos.saturating_sub(1);
            }
            Op::Redo => {
                let ok = history.redo(&mut cage);
                assert_eq!(ok, pos + 1 < states.len());
                if ok {
                    pos += 1;
                }
            }
        }
        assert_eq!(cage, states[pos]);
        assert_eq!(history.undo_len(), pos);
        assert_eq!(history.redo_len(), states.len() - pos - 1);
        assert_eq!(history.can_undo(), pos > 0);
        assert_eq!(history.can_redo(), pos + 1 < states.len());
    }
}

proptest! {
    #[test]
    fn history_matches_a_snapshot_model(ops in proptest::collection::vec(op(), 0..60)) {
        run(&ops);
    }

    #[test]
    fn long_history_matches_a_snapshot_model(ops in proptest::collection::vec(prop_oneof![
        5 => (0usize..8, vec3()).prop_map(|(i, p)| Op::Move(vec![i], vec![p])),
        1 => Just(Op::Undo),
        1 => Just(Op::Redo),
    ], 90..260)) {
        run(&ops);
    }

    #[test]
    fn undo_then_redo_and_redo_then_undo_round_trip(moves in proptest::collection::vec((proptest::collection::vec(0usize..8, 1..5), proptest::collection::vec(vec3(), 1..5)), 1..8), preset_after in proptest::option::of(preset())) {
        let mut cage = Preset::Cube.make();
        let mut history = History::default();
        for (idx, after) in &moves {
            let c = move_from(&cage, idx, after);
            c.apply(&mut cage);
            history.push(c);
        }
        if let Some(p) = preset_after {
            let c = Command::ReplaceCage { before: Box::new(cage.clone()), after: Box::new(p.make()) };
            c.apply(&mut cage);
            history.push(c);
        }
        let top = cage.clone();
        prop_assert!(history.undo(&mut cage));
        let below = cage.clone();
        prop_assert!(history.redo(&mut cage));
        prop_assert_eq!(&cage, &top);
        prop_assert!(history.undo(&mut cage));
        prop_assert_eq!(&cage, &below);
        prop_assert!(history.redo(&mut cage));
        prop_assert_eq!(&cage, &top);
    }

    #[test]
    fn exceeding_the_limit_drops_the_oldest(extra in 1usize..40, p in vec3()) {
        let initial = Preset::Cube.make();
        let mut cage = initial.clone();
        let mut history = History::default();
        let total = HISTORY_LIMIT + extra;
        let mut snapshots = vec![cage.clone()];
        for k in 0..total {
            let target = [p[0] + k as f64, p[1], p[2]];
            let c = move_from(&cage, &[0], &[target]);
            c.apply(&mut cage);
            history.push(c);
            snapshots.push(cage.clone());
        }
        prop_assert_eq!(history.undo_len(), HISTORY_LIMIT);
        while history.undo(&mut cage) {}
        prop_assert_eq!(&cage, &snapshots[extra]);
        prop_assert_ne!(&cage, &initial);
    }

    #[test]
    fn push_clears_redo(n in 1usize..6, undone in 1usize..6, p in vec3()) {
        let mut cage = Preset::Cube.make();
        let mut history = History::default();
        for k in 0..n {
            let c = move_from(&cage, &[k % 8], &[p]);
            c.apply(&mut cage);
            history.push(c);
        }
        for _ in 0..undone.min(n) {
            history.undo(&mut cage);
        }
        prop_assert!(history.can_redo());
        let c = move_from(&cage, &[0], &[[9.0, 9.0, 9.0]]);
        c.apply(&mut cage);
        history.push(c);
        prop_assert_eq!(history.redo_len(), 0);
        let before = cage.clone();
        prop_assert!(!history.redo(&mut cage));
        prop_assert_eq!(cage, before);
    }

    #[test]
    fn empty_history_does_nothing(preset in preset()) {
        let mut cage = preset.make();
        let before = cage.clone();
        let mut h = History::default();
        prop_assert!(!h.undo(&mut cage));
        prop_assert!(!h.redo(&mut cage));
        prop_assert_eq!(cage, before);
        prop_assert_eq!((h.undo_len(), h.redo_len(), h.can_undo(), h.can_redo()), (0, 0, false, false));
    }

    #[test]
    fn malformed_commands_never_panic(
        indices in proptest::collection::vec(weird_index(), 0..8),
        before in proptest::collection::vec([weird_f64(), weird_f64(), weird_f64()], 0..8),
        after in proptest::collection::vec([weird_f64(), weird_f64(), weird_f64()], 0..8),
        on_empty_cage in any::<bool>(),
    ) {
        let mut cage = if on_empty_cage { CageModel::default() } else { Preset::Cube.make() };
        let len = cage.v.len();
        let nf = cage.f.clone();
        let c = Command::MoveVertices { indices: indices.clone(), before, after };
        let mut h = History::default();
        c.apply(&mut cage);
        c.revert(&mut cage);
        h.push(c);
        prop_assert!(h.undo(&mut cage));
        prop_assert!(h.redo(&mut cage));
        prop_assert_eq!(cage.v.len(), len);
        prop_assert_eq!(cage.f, nf);
    }

    #[test]
    fn only_the_shorter_of_indices_and_positions_is_written(
        indices in proptest::collection::vec(0usize..8, 0..8),
        positions in proptest::collection::vec(vec3(), 0..8),
    ) {
        let original = Preset::Cube.make();
        let mut cage = original.clone();
        set_positions(&mut cage, &indices, &positions);
        let n = indices.len().min(positions.len());
        let mut want = original.clone();
        for k in 0..n {
            want.v[indices[k]] = positions[k];
        }
        prop_assert_eq!(cage, want);
    }

    #[test]
    fn out_of_range_indices_leave_the_cage_untouched(indices in proptest::collection::vec(prop_oneof![8usize..1000, Just(usize::MAX)], 0..8), positions in proptest::collection::vec(vec3(), 0..8)) {
        let original = Preset::Cube.make();
        let mut cage = original.clone();
        set_positions(&mut cage, &indices, &positions);
        prop_assert_eq!(cage, original);
    }
}

#[test]
fn replace_cage_swaps_the_whole_cage_both_ways() {
    let (a, b) = (Preset::Cube.make(), Preset::Plane.make());
    let c = Command::ReplaceCage {
        before: Box::new(a.clone()),
        after: Box::new(b.clone()),
    };
    let mut cage = a.clone();
    c.apply(&mut cage);
    assert_eq!(cage, b);
    c.revert(&mut cage);
    assert_eq!(cage, a);
}

#[test]
fn limit_is_one_hundred() {
    assert_eq!(HISTORY_LIMIT, 100);
}
