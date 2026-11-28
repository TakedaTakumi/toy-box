use std::collections::HashSet;

use gregory_core::{CageModel, EdgeKey, PRESETS, Preset, SelectMode, Selection, edge_map};
use gregory_editor::editing::selection::{EditSelection, Element, mode_label};
use proptest::prelude::*;

fn mode() -> impl Strategy<Value = SelectMode> {
    prop_oneof![
        Just(SelectMode::Vertex),
        Just(SelectMode::Edge),
        Just(SelectMode::Face)
    ]
}

fn preset() -> impl Strategy<Value = Preset> {
    (0..PRESETS.len()).prop_map(|i| PRESETS[i])
}

fn edge_keys(cage: &CageModel) -> Vec<EdgeKey> {
    edge_map(&cage.f).keys().collect()
}

fn element_of(cage: &CageModel, mode: SelectMode, seed: usize) -> Element {
    match mode {
        SelectMode::Vertex => Element::Vertex(seed % cage.v.len()),
        SelectMode::Edge => {
            let keys = edge_keys(cage);
            Element::Edge(keys[seed % keys.len()])
        }
        SelectMode::Face => Element::Face(seed % cage.f.len()),
    }
}

fn build(cage: &CageModel, mode: SelectMode, seeds: &[usize]) -> EditSelection {
    let mut s = EditSelection::new(mode);
    for &seed in seeds {
        s.click(element_of(cage, mode, seed), true);
    }
    s
}

fn elements_in(cage: &CageModel, mode: SelectMode) -> Vec<Element> {
    match mode {
        SelectMode::Vertex => (0..cage.v.len()).map(Element::Vertex).collect(),
        SelectMode::Edge => edge_keys(cage).into_iter().map(Element::Edge).collect(),
        SelectMode::Face => (0..cage.f.len()).map(Element::Face).collect(),
    }
}

fn as_set(cage: &CageModel, s: &EditSelection) -> HashSet<usize> {
    s.vertices(cage).into_iter().collect()
}

proptest! {
    #[test]
    fn additive_click_twice_restores(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..12), seed in 0usize..1000) {
        let cage = p.make();
        let before = build(&cage, m, &seeds);
        let e = element_of(&cage, m, seed);
        let mut after = before.clone();
        after.click(e, true);
        prop_assert_ne!(after.contains(e), before.contains(e));
        after.click(e, true);
        prop_assert_eq!(after.len(), before.len());
        for x in elements_in(&cage, m) {
            prop_assert_eq!(after.contains(x), before.contains(x));
        }
        if !before.contains(e) {
            prop_assert_eq!(after, before);
        }
    }

    #[test]
    fn plain_click_selects_only_that_element(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..12), seed in 0usize..1000) {
        let cage = p.make();
        let mut s = build(&cage, m, &seeds);
        let e = element_of(&cage, m, seed);
        s.click(e, false);
        prop_assert_eq!(s.len(), 1);
        prop_assert!(s.contains(e));
        prop_assert_eq!(s.mode(), m);
    }

    #[test]
    fn clear_keeps_the_mode(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..12)) {
        let cage = p.make();
        let mut s = build(&cage, m, &seeds);
        s.clear();
        prop_assert!(s.is_empty());
        prop_assert_eq!(s.mode(), m);
        prop_assert_eq!(s, EditSelection::new(m));
    }

    #[test]
    fn set_mode_to_the_same_mode_changes_nothing(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..12)) {
        let cage = p.make();
        let before = build(&cage, m, &seeds);
        let mut after = before.clone();
        after.set_mode(&cage, m);
        prop_assert_eq!(after, before);
    }

    #[test]
    fn set_mode_switches_mode_and_keeps_only_consistent_elements(p in preset(), from in mode(), to in mode(), seeds in proptest::collection::vec(0usize..1000, 0..12)) {
        let cage = p.make();
        let mut s = build(&cage, from, &seeds);
        let verts = as_set(&cage, &s);
        s.set_mode(&cage, to);
        prop_assert_eq!(s.mode(), to);
        if to == SelectMode::Vertex {
            prop_assert_eq!(as_set(&cage, &s), verts);
        } else {
            prop_assert!(as_set(&cage, &s).is_subset(&verts));
        }
    }

    #[test]
    fn element_of_another_mode_is_ignored(p in preset(), mi in 0usize..3, off in 1usize..3, seeds in proptest::collection::vec(0usize..1000, 0..12), seed in 0usize..1000, additive in any::<bool>()) {
        let modes = [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face];
        let (m, other) = (modes[mi], modes[(mi + off) % 3]);
        let cage = p.make();
        let before = build(&cage, m, &seeds);
        let e = element_of(&cage, other, seed);
        let mut s = before.clone();
        s.click(e, additive);
        prop_assert_eq!(&s, &before);
        s.press(e, additive);
        prop_assert_eq!(&s, &before);
        prop_assert!(!s.contains(e));
    }

    #[test]
    fn plain_press_on_a_selected_element_keeps_the_selection(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 1..12)) {
        let cage = p.make();
        let before = build(&cage, m, &seeds);
        prop_assume!(!before.is_empty());
        let selected = elements_in(&cage, m).into_iter().find(|&e| before.contains(e)).expect("1件以上選んである");
        let mut s = before.clone();
        s.press(selected, false);
        prop_assert_eq!(s, before);
    }

    #[test]
    fn plain_press_on_an_unselected_element_selects_only_it(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..4)) {
        let cage = p.make();
        let before = build(&cage, m, &seeds);
        let Some(e) = elements_in(&cage, m).into_iter().find(|&e| !before.contains(e)) else { return Ok(()) };
        let mut s = before;
        s.press(e, false);
        prop_assert_eq!(s.len(), 1);
        prop_assert!(s.contains(e));
    }

    #[test]
    fn additive_press_behaves_like_click(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..8), seed in 0usize..1000) {
        let cage = p.make();
        let before = build(&cage, m, &seeds);
        let e = element_of(&cage, m, seed);
        let (mut a, mut b) = (before.clone(), before);
        a.press(e, true);
        b.click(e, true);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn select_all_selects_every_element(p in preset(), m in mode(), seeds in proptest::collection::vec(0usize..1000, 0..6)) {
        let cage = p.make();
        let mut s = build(&cage, m, &seeds);
        s.select_all(&cage);
        prop_assert_eq!(s.mode(), m);
        prop_assert_eq!(s.len(), elements_in(&cage, m).len());
        for e in elements_in(&cage, m) {
            prop_assert!(s.contains(e));
        }
        let all: HashSet<usize> = (0..cage.v.len()).collect();
        prop_assert_eq!(as_set(&cage, &s), all);
    }

    #[test]
    fn vertices_drops_out_of_range_and_follows_toggle_parity(extra in proptest::collection::vec(prop_oneof![0usize..8, 8usize..100, Just(usize::MAX)], 0..20)) {
        let cage = Preset::Cube.make();
        let mut s = EditSelection::new(SelectMode::Vertex);
        for &i in &extra {
            s.click(Element::Vertex(i), true);
        }
        let got = s.vertices(&cage);
        prop_assert!(got.iter().all(|&v| v < cage.v.len()));
        let unique: HashSet<usize> = got.iter().copied().collect();
        prop_assert_eq!(unique.len(), got.len());
        let want: HashSet<usize> = (0..8).filter(|&v| extra.iter().filter(|&&i| i == v).count() % 2 == 1).collect();
        prop_assert_eq!(unique, want);
    }

    #[test]
    fn out_of_range_elements_never_panic(a in prop_oneof![Just(usize::MAX), 0usize..1000], b in prop_oneof![Just(usize::MAX), 0usize..1000], m in mode(), additive in any::<bool>()) {
        let cage = Preset::Cube.make();
        let mut s = EditSelection::new(m);
        for e in [Element::Vertex(a), Element::Edge(EdgeKey::new(a, b)), Element::Face(b)] {
            s.click(e, additive);
            s.press(e, additive);
            let _ = e.exists_in(&cage);
        }
        let verts = s.vertices(&cage);
        prop_assert!(verts.iter().all(|&v| v < cage.v.len()));
        s.set_mode(&cage, SelectMode::Face);
        s.set_mode(&cage, SelectMode::Edge);
        s.select_all(&cage);
    }
}

#[test]
fn non_existing_elements_are_not_reported_as_existing() {
    let cage = Preset::Cube.make();
    assert!(Element::Vertex(7).exists_in(&cage));
    assert!(!Element::Vertex(8).exists_in(&cage));
    assert!(!Element::Vertex(usize::MAX).exists_in(&cage));
    assert!(Element::Face(5).exists_in(&cage));
    assert!(!Element::Face(6).exists_in(&cage));
    assert!(!Element::Edge(EdgeKey::new(0, usize::MAX)).exists_in(&cage));
    assert!(!Element::Edge(EdgeKey::new(0, 0)).exists_in(&cage));
    for key in edge_keys(&cage) {
        assert!(Element::Edge(key).exists_in(&cage));
    }
}

#[test]
fn out_of_range_edge_selection_yields_no_vertices() {
    let cage = Preset::Cube.make();
    let mut s = EditSelection::new(SelectMode::Edge);
    s.click(Element::Edge(EdgeKey::new(3, 99)), true);
    assert_eq!(s.len(), 1);
    assert!(s.vertices(&cage).is_empty());
}

#[test]
fn face_selection_with_a_bad_vertex_index_is_filtered_by_vertices() {
    let cage = CageModel {
        v: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        f: vec![vec![0, 1, 2, 77]],
        creases: HashSet::new(),
    };
    let mut s = EditSelection::new(SelectMode::Face);
    s.click(Element::Face(0), false);
    assert_eq!(s.vertices(&cage), vec![0, 1, 2]);
}

#[test]
fn empty_cage_survives_every_operation() {
    let cage = CageModel {
        v: vec![],
        f: vec![],
        creases: HashSet::new(),
    };
    for m in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
        let mut s = EditSelection::new(m);
        s.select_all(&cage);
        assert!(s.is_empty());
        assert!(s.vertices(&cage).is_empty());
        s.set_mode(&cage, SelectMode::Face);
    }
}

#[test]
fn single_vertex_only_for_exactly_one_vertex() {
    let mut s = EditSelection::default();
    assert_eq!(s.single_vertex(), None);
    s.click(Element::Vertex(3), true);
    assert_eq!(s.single_vertex(), Some(3));
    s.click(Element::Vertex(4), true);
    assert_eq!(s.single_vertex(), None);
    let mut e = EditSelection::new(SelectMode::Edge);
    e.click(Element::Edge(EdgeKey::new(0, 1)), true);
    assert_eq!(e.single_vertex(), None);
}

#[test]
fn default_is_empty_vertex_mode_and_labels() {
    let s = EditSelection::default();
    assert!(s.is_empty());
    assert_eq!(s.mode(), SelectMode::Vertex);
    assert_eq!(s.items(), &Selection::Vertices(vec![]));
    assert_eq!(mode_label(SelectMode::Vertex), "点");
    assert_eq!(mode_label(SelectMode::Edge), "辺");
    assert_eq!(mode_label(SelectMode::Face), "面");
}
