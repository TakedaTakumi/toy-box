use std::collections::HashSet;

use gregory_core::{EdgeKey, PRESETS, Preset, SelectMode, Selection};
use gregory_editor::mesh_buffers::{
    Highlight, POINT_SIZE_OTHER, POINT_SIZE_SELECTED, POINT_SIZE_VERTEX_MODE, cage_lines,
    cage_points, flat_surface, scene_geometry, smooth_surface,
};
use gregory_editor::palette;
use gregory_editor::scene::{Scene, build_scene};
use gregory_editor::settings::Shading;
use proptest::prelude::*;

fn scene_of(p: Preset) -> Scene {
    build_scene(&p.make(), 3).expect("プリセットは組み立てられる")
}

fn preset() -> impl Strategy<Value = Preset> {
    (0..PRESETS.len()).prop_map(|i| PRESETS[i])
}

fn shading() -> impl Strategy<Value = Shading> {
    prop_oneof![Just(Shading::Smooth), Just(Shading::Flat)]
}

fn pick<T: Clone + std::fmt::Debug + 'static>(items: Vec<T>, picks: Vec<usize>) -> Vec<T> {
    if items.is_empty() {
        return vec![];
    }
    picks
        .into_iter()
        .map(|i| items[i % items.len()].clone())
        .collect()
}

/// どのモードの集合も、範囲外の番号を混ぜて作る。
fn selection(mode: SelectMode, scene: &Scene, picks: Vec<usize>, junk: Vec<usize>) -> Selection {
    match mode {
        SelectMode::Vertex => {
            let mut v = pick((0..scene.cage.v.len()).collect(), picks);
            v.extend(junk.iter().map(|j| scene.cage.v.len() + j));
            Selection::Vertices(v)
        }
        SelectMode::Edge => {
            let keys: Vec<EdgeKey> = scene
                .model
                .edges
                .iter()
                .map(|e| EdgeKey::new(e.a, e.b))
                .collect();
            let mut e = pick(keys, picks);
            e.extend(junk.iter().map(|j| EdgeKey::new(0, scene.cage.v.len() + j)));
            Selection::Edges(e)
        }
        SelectMode::Face => {
            let mut f = pick((0..scene.cage.f.len()).collect(), picks);
            f.extend(junk.iter().map(|j| scene.cage.f.len() + j));
            Selection::Faces(f)
        }
    }
}

fn mode() -> impl Strategy<Value = SelectMode> {
    prop_oneof![
        Just(SelectMode::Vertex),
        Just(SelectMode::Edge),
        Just(SelectMode::Face)
    ]
}

proptest! {
    #[test]
    fn only_tinted_faces_get_tint_in_flat_shading(
        p in preset(), picks in proptest::collection::vec(0usize..20, 0..6), junk in proptest::collection::vec(0usize..5, 0..3),
    ) {
        let scene = scene_of(p);
        let h = Highlight::of(&selection(SelectMode::Face, &scene, picks, junk));
        let mesh = flat_surface(&scene.model.mesh, &h.faces);
        let valid: Vec<usize> = scene.model.mesh.tris.iter().enumerate().filter(|(t, _)| scene.model.mesh.tri_face.get(*t).is_some()).map(|(t, _)| t).collect();
        prop_assert_eq!(mesh.vertices.len(), valid.len() * 3);
        for (k, &t) in valid.iter().enumerate() {
            let want = if h.faces.contains(&scene.model.mesh.tri_face[t]) { 1.0 } else { 0.0 };
            for c in 0..3 {
                prop_assert_eq!(mesh.vertices[k * 3 + c].tint, want);
            }
        }
    }

    #[test]
    fn smooth_tint_marks_exactly_the_vertices_of_tinted_triangles(
        p in preset(), picks in proptest::collection::vec(0usize..20, 0..6),
    ) {
        let scene = scene_of(p);
        let h = Highlight::of(&selection(SelectMode::Face, &scene, picks, vec![]));
        let mesh = smooth_surface(&scene.model.mesh, &h.faces);
        let mut want = vec![0.0f32; scene.model.mesh.pos.len()];
        for (tri, face) in scene.model.mesh.tris.iter().zip(&scene.model.mesh.tri_face) {
            if h.faces.contains(face) {
                for &i in tri {
                    want[i] = 1.0;
                }
            }
        }
        let got: Vec<f32> = mesh.vertices.iter().map(|v| v.tint).collect();
        prop_assert_eq!(got, want);
    }

    #[test]
    fn face_tint_only_applies_in_face_mode(
        p in preset(), m in mode(), sh in shading(), picks in proptest::collection::vec(0usize..20, 1..6),
    ) {
        let scene = scene_of(p);
        let mut h = Highlight::of(&selection(SelectMode::Face, &scene, picks, vec![]));
        h.mode = m;
        let g = scene_geometry(&scene.cage, &scene.model, sh, &h);
        let any_tint = g.surface.vertices.iter().any(|v| v.tint == 1.0);
        prop_assert_eq!(any_tint, m == SelectMode::Face);
        prop_assert!(g.surface.vertices.iter().all(|v| v.tint == 0.0 || v.tint == 1.0));
    }

    #[test]
    fn only_selected_edges_use_the_selection_color_in_edge_mode(
        p in preset(), m in mode(), picks in proptest::collection::vec(0usize..40, 0..8), junk in proptest::collection::vec(0usize..5, 0..3),
    ) {
        let scene = scene_of(p);
        let mut h = Highlight::of(&selection(SelectMode::Edge, &scene, picks, junk));
        h.mode = m;
        let lines = cage_lines(&scene.cage.v, &scene.model.edges, &h);
        prop_assert_eq!(lines.len(), scene.model.edges.len() * 2);
        for (i, edge) in scene.model.edges.iter().enumerate() {
            let selected = m == SelectMode::Edge && h.edges.contains(&EdgeKey::new(edge.a, edge.b));
            for end in 0..2 {
                prop_assert_eq!(lines[i * 2 + end].color == palette::SELECTION, selected, "辺 {}-{}", edge.a, edge.b);
            }
        }
    }

    #[test]
    fn point_sizes_and_colors_follow_the_mode(
        p in preset(), m in mode(), picks in proptest::collection::vec(0usize..40, 0..8), junk in proptest::collection::vec(0usize..5, 0..3),
    ) {
        let scene = scene_of(p);
        let mut h = Highlight::of(&selection(SelectMode::Vertex, &scene, picks, junk));
        h.mode = m;
        let points = cage_points(&scene.cage.v, &h);
        prop_assert_eq!(points.len(), scene.cage.v.len());
        for (i, pt) in points.iter().enumerate() {
            let (size, color) = match m {
                SelectMode::Vertex if h.vertices.contains(&i) => (POINT_SIZE_SELECTED, palette::SELECTION),
                SelectMode::Vertex => (POINT_SIZE_VERTEX_MODE, palette::CAGE_POINT),
                _ => (POINT_SIZE_OTHER, palette::CAGE_POINT),
            };
            prop_assert_eq!((pt.size, pt.color), (size, color), "頂点 {}", i);
        }
    }

    #[test]
    fn highlight_of_fills_only_the_set_for_its_mode(
        p in preset(), m in mode(), picks in proptest::collection::vec(0usize..40, 0..8), junk in proptest::collection::vec(0usize..5, 0..3),
    ) {
        let scene = scene_of(p);
        let sel = selection(m, &scene, picks, junk);
        let h = Highlight::of(&sel);
        prop_assert_eq!(h.mode, m);
        let (v, e, f) = (h.vertices.len(), h.edges.len(), h.faces.len());
        match sel {
            Selection::Vertices(x) => prop_assert_eq!((v, e, f), (x.iter().collect::<HashSet<_>>().len(), 0, 0)),
            Selection::Edges(x) => prop_assert_eq!((v, e, f), (0, x.iter().collect::<HashSet<_>>().len(), 0)),
            Selection::Faces(x) => prop_assert_eq!((v, e, f), (0, 0, x.iter().collect::<HashSet<_>>().len())),
        }
    }

    #[test]
    fn out_of_range_highlight_never_changes_the_drawing(
        p in preset(), sh in shading(), m in mode(), junk in proptest::collection::vec(0usize..1000, 1..5),
    ) {
        let scene = scene_of(p);
        let mut h = Highlight::of(&selection(m, &scene, vec![], junk));
        h.vertices.insert(usize::MAX);
        h.faces.insert(usize::MAX);
        h.edges.insert(EdgeKey::new(usize::MAX, 0));
        let plain = Highlight { mode: m, ..Highlight::default() };
        let a = scene_geometry(&scene.cage, &scene.model, sh, &h);
        let b = scene_geometry(&scene.cage, &scene.model, sh, &plain);
        prop_assert_eq!(a, b);
    }
}

#[test]
fn a_selected_crease_edge_uses_the_selection_color_over_the_crease_color() {
    let mut cage = Preset::Cube.make();
    let key = *cage.creases.iter().next().unwrap_or(&EdgeKey::new(0, 1));
    cage.creases.insert(key);
    let scene = build_scene(&cage, 3).expect("組み立てられる");
    let crease_idx = scene
        .model
        .edges
        .iter()
        .position(|e| e.crease)
        .expect("折れ目の辺がある");
    let e = &scene.model.edges[crease_idx];
    let plain = Highlight::default();
    assert_eq!(
        cage_lines(&scene.cage.v, &scene.model.edges, &plain)[crease_idx * 2].color,
        palette::CREASE_LINE
    );
    let h = Highlight::of(&Selection::Edges(vec![EdgeKey::new(e.a, e.b)]));
    assert_eq!(
        cage_lines(&scene.cage.v, &scene.model.edges, &h)[crease_idx * 2].color,
        palette::SELECTION
    );
}

#[test]
fn point_size_constants() {
    assert_eq!(
        (
            POINT_SIZE_SELECTED,
            POINT_SIZE_VERTEX_MODE,
            POINT_SIZE_OTHER
        ),
        (11.0, 8.0, 5.0)
    );
}

#[test]
fn default_highlight_is_empty_in_vertex_mode() {
    let h = Highlight::default();
    assert_eq!(h.mode, SelectMode::Vertex);
    assert!(h.vertices.is_empty() && h.edges.is_empty() && h.faces.is_empty());
}
