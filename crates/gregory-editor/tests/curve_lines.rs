use std::collections::HashSet;

use gregory_core::{Edge, EdgeKey, Preset, SelectMode, Vec3, bezier3};
use gregory_editor::editing::pick::EDGE_SAMPLES;
use gregory_editor::mesh_buffers::{
    Highlight, LineVertex, cage_lines, curve_lines, scene_geometry,
};
use gregory_editor::palette;
use gregory_editor::scene::build_scene;
use gregory_editor::settings::{DisplaySettings, Shading};
use proptest::prelude::*;

fn mode() -> impl Strategy<Value = SelectMode> {
    prop_oneof![
        Just(SelectMode::Vertex),
        Just(SelectMode::Edge),
        Just(SelectMode::Face)
    ]
}

fn finite_vec() -> impl Strategy<Value = Vec3> {
    [-100.0f64..100.0, -100.0f64..100.0, -100.0f64..100.0]
}

/// 有限・NaN・±無限大・f32 を超える巨大値を混ぜた座標。
fn wild_vec() -> impl Strategy<Value = Vec3> {
    let c = prop_oneof![
        4 => -100.0f64..100.0,
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => Just(1e300),
    ];
    [c.clone(), c.clone(), c]
}

fn edge_of(a: usize, b: usize, e: [Vec3; 4], crease: bool) -> Edge {
    Edge {
        a,
        b,
        faces: vec![],
        e,
        crease,
    }
}

fn finite_edge() -> impl Strategy<Value = Edge> {
    (
        0usize..8,
        0usize..8,
        [finite_vec(), finite_vec(), finite_vec(), finite_vec()],
        any::<bool>(),
    )
        .prop_map(|(a, b, e, crease)| edge_of(a, b, e, crease))
}

fn wild_edge() -> impl Strategy<Value = Edge> {
    (
        0usize..8,
        0usize..8,
        [wild_vec(), wild_vec(), wild_vec(), wild_vec()],
        any::<bool>(),
    )
        .prop_map(|(a, b, e, crease)| edge_of(a, b, e, crease))
}

fn highlight(mode: SelectMode, edges: &[EdgeKey]) -> Highlight {
    Highlight {
        mode,
        vertices: HashSet::new(),
        edges: edges.iter().copied().collect(),
        faces: HashSet::new(),
    }
}

fn f32_of(v: Vec3) -> [f32; 3] {
    v.map(|x| x as f32)
}

fn expected_color(edge: &Edge, h: &Highlight) -> [f32; 3] {
    if h.mode == SelectMode::Edge && h.edges.contains(&EdgeKey::new(edge.a, edge.b)) {
        palette::SELECTION
    } else if edge.crease {
        palette::CREASE_LINE
    } else {
        palette::CURVE_LINE
    }
}

fn all_finite(lines: &[LineVertex]) -> bool {
    lines
        .iter()
        .all(|l| l.position.iter().chain(&l.color).all(|x| x.is_finite()))
}

proptest! {
    #[test]
    fn finite_edges_make_continuous_polylines_between_their_ends(
        edges in proptest::collection::vec(finite_edge(), 0..6),
        m in mode(),
    ) {
        let lines = curve_lines(&edges, &highlight(m, &[]));
        prop_assert_eq!(lines.len(), edges.len() * EDGE_SAMPLES * 2);
        prop_assert!(all_finite(&lines));
        for (edge, chunk) in edges.iter().zip(lines.chunks(EDGE_SAMPLES * 2)) {
            for k in 0..EDGE_SAMPLES - 1 {
                prop_assert_eq!(chunk[k * 2 + 1].position, chunk[(k + 1) * 2].position);
            }
            prop_assert_eq!(chunk[0].position, f32_of(edge.e[0]));
            prop_assert_eq!(chunk[chunk.len() - 1].position, f32_of(edge.e[3]));
        }
    }

    #[test]
    fn only_segments_with_non_finite_ends_are_dropped(
        edges in proptest::collection::vec(wild_edge(), 0..6),
        m in mode(),
    ) {
        let lines = curve_lines(&edges, &highlight(m, &[]));
        prop_assert!(all_finite(&lines));
        let finite_sample = |e: &Edge, i: usize| {
            let p = f32_of(bezier3(&e.e, i as f64 / EDGE_SAMPLES as f64));
            p.iter().all(|x| x.is_finite())
        };
        let expected: usize = edges
            .iter()
            .map(|e| (0..EDGE_SAMPLES).filter(|&k| finite_sample(e, k) && finite_sample(e, k + 1)).count())
            .sum();
        prop_assert_eq!(lines.len(), expected * 2);
    }

    #[test]
    fn a_non_finite_control_point_never_yields_non_finite_vertices(
        mut edge in finite_edge(),
        which in 0usize..4,
        axis in 0usize..3,
        bad in prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)],
    ) {
        edge.e[which][axis] = bad;
        let lines = curve_lines(&[edge], &Highlight::default());
        prop_assert!(all_finite(&lines));
        prop_assert!(lines.len() < EDGE_SAMPLES * 2);
    }

    #[test]
    fn colors_follow_selection_and_crease(
        edges in proptest::collection::vec(finite_edge(), 0..6),
        m in mode(),
        picks in proptest::collection::vec((0usize..8, 0usize..8), 0..5),
    ) {
        let keys: Vec<EdgeKey> = picks.iter().map(|&(a, b)| EdgeKey::new(a, b)).collect();
        let h = highlight(m, &keys);
        let lines = curve_lines(&edges, &h);
        for (edge, chunk) in edges.iter().zip(lines.chunks(EDGE_SAMPLES * 2)) {
            let color = expected_color(edge, &h);
            prop_assert!(chunk.iter().all(|l| l.color == color));
            if m != SelectMode::Edge {
                prop_assert!(chunk.iter().all(|l| l.color != palette::SELECTION));
            }
        }
    }

    #[test]
    fn out_of_range_edge_keys_do_not_change_the_result(
        edges in proptest::collection::vec(finite_edge(), 0..6),
        m in mode(),
        junk in proptest::collection::vec((8usize..usize::MAX, 8usize..usize::MAX), 0..5),
    ) {
        let keys: Vec<EdgeKey> = junk.iter().map(|&(a, b)| EdgeKey::new(a, b)).collect();
        prop_assert_eq!(
            curve_lines(&edges, &highlight(m, &keys)),
            curve_lines(&edges, &highlight(m, &[]))
        );
    }

    #[test]
    fn cage_lines_ignore_selection_and_only_crease_changes_color(
        edges in proptest::collection::vec(finite_edge(), 0..6),
    ) {
        let verts: Vec<Vec3> = (0..8).map(|i| [i as f64, 0.0, 1.0]).collect();
        let lines = cage_lines(&verts, &edges);
        prop_assert_eq!(lines.len(), edges.len() * 2);
        for (edge, pair) in edges.iter().zip(lines.chunks(2)) {
            let color = if edge.crease { palette::CREASE_LINE } else { palette::CAGE_LINE };
            prop_assert!(pair.iter().all(|l| l.color == color));
        }
    }
}

#[test]
fn empty_edges_make_no_lines() {
    assert!(curve_lines(&[], &Highlight::default()).is_empty());
}

#[test]
fn cube_scene_has_twelve_edges_of_curve_segments() {
    let scene = build_scene(&Preset::Cube.make(), 3).expect("立方体は組み立てられる");
    let geo = scene_geometry(
        &scene.cage,
        &scene.model,
        Shading::Smooth,
        &Highlight::default(),
    );
    assert_eq!(scene.cage.v.len(), 8);
    assert_eq!(geo.curve_lines.len(), 12 * 48);
}

#[test]
fn curves_are_shown_by_default() {
    assert!(DisplaySettings::default().show_curves);
}
