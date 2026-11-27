use std::collections::HashSet;

use gregory_core::{CageModel, Edge, Model, TriMesh, Vec3, cross, sub};
use gregory_editor::mesh_buffers::{
    Highlight, cage_lines, cage_points, flat_surface, scene_geometry, smooth_surface, surface_mesh,
};
use gregory_editor::palette;
use gregory_editor::scene::build_scene;
use gregory_editor::settings::Shading;
use proptest::prelude::*;

fn coord() -> impl Strategy<Value = f64> {
    prop_oneof![
        8 => -100.0f64..100.0,
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => prop_oneof![Just(1e300), Just(-1e300), Just(f64::MAX), Just(3.5e38)],
    ]
}

fn vec3() -> impl Strategy<Value = Vec3> {
    [coord(), coord(), coord()]
}

fn index(n: usize) -> impl Strategy<Value = usize> {
    prop_oneof![8 => 0..n.max(1), 1 => n..n + 3, 1 => Just(usize::MAX)]
}

fn mesh() -> impl Strategy<Value = TriMesh> {
    proptest::collection::vec(vec3(), 0..12).prop_flat_map(|pos| {
        let n = pos.len();
        (
            Just(pos),
            proptest::collection::vec(vec3(), 0..14),
            proptest::collection::vec([index(n), index(n), index(n)], 0..20),
        )
            .prop_map(|(pos, nrm, tris)| TriMesh {
                pos,
                nrm,
                tris,
                tri_face: vec![],
            })
    })
}

fn finite32(p: Vec3) -> Option<[f32; 3]> {
    let f = p.map(|x| x as f32);
    f.iter().all(|x| x.is_finite()).then_some(f)
}

fn valid_tris(m: &TriMesh) -> Vec<[usize; 3]> {
    m.tris
        .iter()
        .copied()
        .filter(|t| {
            t.iter()
                .all(|&i| m.pos.get(i).is_some_and(|p| finite32(*p).is_some()))
        })
        .collect()
}

proptest! {
    #[test]
    fn smooth_surface_invariants(m in mesh()) {
        let s = smooth_surface(&m, &HashSet::new());
        prop_assert_eq!(s.vertices.len(), m.pos.len());
        prop_assert_eq!(s.indices.len() % 3, 0);
        prop_assert!(s.indices.iter().all(|&i| (i as usize) < s.vertices.len()));
        for &i in &s.indices {
            prop_assert!(s.vertices[i as usize].position.iter().all(|x| x.is_finite()));
        }
        for v in &s.vertices {
            prop_assert!(v.normal.iter().all(|x| x.is_finite()));
        }
        let expected: Vec<u32> = valid_tris(&m).iter().flatten().map(|&i| i as u32).collect();
        prop_assert_eq!(&s.indices, &expected);
        for (i, v) in s.vertices.iter().enumerate() {
            let want = m.nrm.get(i).and_then(|n| finite32(*n)).unwrap_or([0.0; 3]);
            prop_assert_eq!(v.normal, want);
        }
    }

    #[test]
    fn flat_surface_invariants(m in mesh()) {
        let s = flat_surface(&m, &HashSet::new());
        let valid = valid_tris(&m);
        prop_assert_eq!(s.vertices.len(), valid.len() * 3);
        prop_assert_eq!(s.indices.len(), valid.len() * 3);
        prop_assert!(s.indices.iter().enumerate().all(|(k, &i)| i as usize == k));
        for (t, tri) in valid.iter().enumerate() {
            let vs = &s.vertices[t * 3..t * 3 + 3];
            prop_assert_eq!(vs[0].normal, vs[1].normal);
            prop_assert_eq!(vs[1].normal, vs[2].normal);
            prop_assert!(vs.iter().all(|v| v.position.iter().all(|x| x.is_finite())));
            let n = vs[0].normal.map(f64::from);
            let l = n.iter().map(|x| x * x).sum::<f64>().sqrt();
            prop_assert!(l == 0.0 || (l - 1.0).abs() < 1e-5, "len {l}");
            let [a, b, c] = tri.map(|i| m.pos[i]);
            let cr = cross(sub(b, a), sub(c, a));
            if l > 0.0 && cr.iter().all(|x| x.is_finite()) {
                let d: f64 = n.iter().zip(cr).map(|(x, y)| x * y).sum();
                prop_assert!(d > 0.0, "dot {d}");
            }
        }
    }

    #[test]
    fn surface_mesh_dispatch(m in mesh()) {
        prop_assert_eq!(surface_mesh(&m, Shading::Smooth, &HashSet::new()), smooth_surface(&m, &HashSet::new()));
        prop_assert_eq!(surface_mesh(&m, Shading::Flat, &HashSet::new()), flat_surface(&m, &HashSet::new()));
    }

    #[test]
    fn cage_lines_and_points(
        v in proptest::collection::vec(vec3(), 0..10),
        raw in proptest::collection::vec((0usize..14, 0usize..14, any::<bool>()), 0..20),
    ) {
        let edges: Vec<Edge> = raw
            .into_iter()
            .map(|(a, b, crease)| Edge { a, b, faces: vec![], e: [[0.0; 3]; 4], crease })
            .collect();
        let lines = cage_lines(&v, &edges, &Highlight::default());
        prop_assert_eq!(lines.len() % 2, 0);
        let ok: Vec<&Edge> = edges
            .iter()
            .filter(|e| {
                v.get(e.a).and_then(|p| finite32(*p)).is_some()
                    && v.get(e.b).and_then(|p| finite32(*p)).is_some()
            })
            .collect();
        prop_assert_eq!(lines.len(), ok.len() * 2);
        for (pair, e) in lines.chunks(2).zip(&ok) {
            let color = if e.crease { palette::CREASE_LINE } else { palette::CAGE_LINE };
            prop_assert!(pair.iter().all(|l| l.color == color));
            prop_assert!(pair.iter().all(|l| l.position.iter().all(|x| x.is_finite())));
        }
        let pts = cage_points(&v, &Highlight::default());
        prop_assert_eq!(pts.len(), v.iter().filter(|p| finite32(**p).is_some()).count());
        prop_assert!(pts.iter().all(|p| p.color == palette::CAGE_POINT));
    }
}

#[test]
fn empty_inputs() {
    let m = TriMesh::default();
    assert!(smooth_surface(&m, &HashSet::new()).vertices.is_empty());
    assert!(flat_surface(&m, &HashSet::new()).indices.is_empty());
    assert!(cage_lines(&[], &[], &Highlight::default()).is_empty());
    assert!(cage_points(&[], &Highlight::default()).is_empty());
}

#[test]
fn nan_triangle_dropped_but_valid_kept_in_order() {
    let m = TriMesh {
        pos: vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [f64::NAN, 0.0, 0.0],
        ],
        nrm: vec![],
        tris: vec![[0, 1, 3], [0, 1, 2], [0, 1, 9], [2, 1, 0]],
        tri_face: vec![],
    };
    assert_eq!(
        smooth_surface(&m, &HashSet::new()).indices,
        vec![0, 1, 2, 2, 1, 0]
    );
    assert_eq!(flat_surface(&m, &HashSet::new()).vertices.len(), 6);
}

#[test]
fn cube_geometry_counts() {
    let scene = build_scene(&gregory_core::Preset::Cube.make(), 4).unwrap();
    let CageModel { .. } = scene.cage;
    let Model { .. } = scene.model;
    for shading in Shading::ALL {
        let g = scene_geometry(&scene.cage, &scene.model, shading, &Highlight::default());
        assert_eq!(g.cage_lines.len(), 24);
        assert_eq!(g.cage_points.len(), 8);
        assert_eq!(g.surface.indices.len() % 3, 0);
    }
}
