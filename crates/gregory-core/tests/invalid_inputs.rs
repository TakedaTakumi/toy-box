use gregory_core::*;
use std::collections::HashSet;

const TRI_V: [Vec3; 3] = [[0., 0., 0.], [1., 0., 0.], [0., 0., -1.]];

fn quad_v() -> Vec<Vec3> {
    vec![[0., 0., 0.], [1., 0., 0.], [1., 0.2, -1.], [0., 0., -1.]]
}

fn opts() -> BuildOptions {
    BuildOptions {
        tau: 1.0,
        mode: Mode::G1,
        n: 2,
        creases: HashSet::new(),
    }
}

fn cube() -> (Vec<Vec3>, Vec<Vec<usize>>) {
    let v = [
        [-1., -1., -1.],
        [1., -1., -1.],
        [1., 1., -1.],
        [-1., 1., -1.],
        [-1., -1., 1.],
        [1., -1., 1.],
        [1., 1., 1.],
        [-1., 1., 1.],
    ];
    let f = vec![
        vec![4, 5, 6, 7],
        vec![3, 2, 1, 0],
        vec![0, 1, 5, 4],
        vec![2, 3, 7, 6],
        vec![1, 2, 6, 5],
        vec![3, 0, 4, 7],
    ];
    (v.to_vec(), f)
}

#[test]
fn nan_and_infinite_coordinates_are_ok() {
    let mut v = quad_v();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        v[1][1] = bad;
        assert!(
            build_model(&v, &[vec![0, 1, 2, 3]], &opts()).is_ok(),
            "{bad}"
        );
    }
}

#[test]
fn len_with_infinity_and_nan_is_infinity() {
    assert_eq!(len([f64::INFINITY, f64::NAN, 0.0]), f64::INFINITY);
    assert_eq!(len([f64::NAN, f64::NEG_INFINITY, 1.0]), f64::INFINITY);
    assert!(len([f64::NAN, 1.0, 2.0]).is_nan());
}

#[test]
fn norm_of_vector_shorter_than_1e_12_is_zero() {
    assert_eq!(norm([1e-13, 0.0, 0.0]), [0.0; 3]);
    assert_eq!(norm([0.0; 3]), [0.0; 3]);
    assert_eq!(norm([5e-324, 0.0, 0.0]), [0.0; 3]);
}

#[test]
fn bezier3_outside_unit_range_extrapolates_and_nan_propagates() {
    let e = [[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]];
    assert!((bezier3(&e, 2.0)[0] - 6.0).abs() < 1e-12);
    assert!((bezier3(&e, -1.0)[0] + 3.0).abs() < 1e-12);
    assert!(bezier3(&e, f64::NAN).iter().all(|x| x.is_nan()));
}

#[test]
fn vertex_index_out_of_range_is_an_error() {
    let r = build_model(&TRI_V, &[vec![0, 1, 3]], &opts());
    assert!(matches!(
        r,
        Err(GregoryError::VertexOutOfRange {
            face: 0,
            corner: 2,
            vertex: 3,
            vertex_count: 3
        })
    ));
    assert!(matches!(
        build_model(&[], &[vec![0, 1, 2]], &opts()),
        Err(GregoryError::VertexOutOfRange { .. })
    ));
    assert!(matches!(
        build_model(&TRI_V, &[vec![0, 1, usize::MAX]], &opts()),
        Err(GregoryError::VertexOutOfRange { .. })
    ));
}

#[test]
fn face_with_fewer_than_three_corners_is_an_error() {
    for face in [vec![], vec![0], vec![0, 1]] {
        let r = build_model(&TRI_V, &[vec![0, 1, 2], face.clone()], &opts());
        assert!(
            matches!(r, Err(GregoryError::TooFewCorners { face: 1, .. })),
            "{face:?}"
        );
    }
}

#[test]
fn pentagon_is_a_triangle_patch_and_not_counted() {
    let v: Vec<Vec3> = (0..5)
        .map(|i| [(i as f64).cos(), 0.0, (i as f64).sin()])
        .collect();
    let m = build_model(&v, &[vec![0, 1, 2, 3, 4]], &opts()).unwrap();
    assert_eq!(m.patches[0].kind(), PatchKind::Tri);
    assert_eq!(m.count, FaceCount { q: 0, t: 0 });
}

#[test]
fn pentagon_sharing_its_fourth_edge_is_an_error() {
    let v: Vec<Vec3> = (0..8)
        .map(|i| [(i as f64).cos(), 0.1 * i as f64, (i as f64).sin()])
        .collect();
    let r = build_model(&v, &[vec![0, 1, 2, 3, 4], vec![4, 3, 5, 6, 7]], &opts());
    assert!(matches!(r, Err(GregoryError::EdgeOutsidePatch { .. })));
}

#[test]
fn repeated_vertex_in_a_face_is_ok() {
    assert!(build_model(&TRI_V, &[vec![0, 1, 2, 2]], &opts()).is_ok());
}

#[test]
fn edge_shared_by_three_faces_ignores_its_crease() {
    let v: Vec<Vec3> = vec![
        [0., 0., 0.],
        [1., 0., 0.],
        [0.5, 0., -1.],
        [0.5, 0., 1.],
        [0.5, 1., 0.],
    ];
    let f = vec![vec![0, 1, 2], vec![1, 0, 3], vec![0, 1, 4]];
    let creased = BuildOptions {
        creases: [EdgeKey::new(0, 1)].into(),
        ..opts()
    };
    let (a, b) = (
        build_model(&v, &f, &opts()).unwrap(),
        build_model(&v, &f, &creased).unwrap(),
    );
    assert_eq!((b.check.pairs, b.check.creased), (0, 0));
    assert_eq!(a.check.pairs, b.check.pairs);
    let shared = b.edges.iter().find(|e| (e.a, e.b) == (0, 1)).unwrap();
    assert_eq!(shared.faces.len(), 3);
}

#[test]
fn empty_faces_give_empty_model_with_zero_normals() {
    let (v, _) = cube();
    let m = build_model(&v, &[], &opts()).unwrap();
    assert_eq!(m.nrm, vec![[0.0; 3]; 8]);
    assert!(
        m.patches.is_empty()
            && m.edges.is_empty()
            && m.mesh.pos.is_empty()
            && m.mesh.tris.is_empty()
    );
    assert_eq!(
        (m.check.pairs, m.check.creased, m.check.g0, m.check.g1),
        (0, 0, 0.0, 0.0)
    );
}

#[test]
fn empty_vertices_and_faces_give_empty_model() {
    let m = build_model(&[], &[], &opts()).unwrap();
    assert!(
        m.nrm.is_empty() && m.patches.is_empty() && m.edges.is_empty() && m.mesh.pos.is_empty()
    );
    assert_eq!(m.count, FaceCount { q: 0, t: 0 });
}

#[test]
fn unused_vertex_has_zero_normal() {
    let (mut v, f) = cube();
    v.push([5., 5., 5.]);
    let m = build_model(&v, &f, &opts()).unwrap();
    assert_eq!(m.nrm[8], [0.0; 3]);
}

#[test]
fn nan_infinite_and_negative_tau_are_ok() {
    let (v, f) = cube();
    for tau in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        assert!(
            build_model(&v, &f, &BuildOptions { tau, ..opts() }).is_ok(),
            "{tau}"
        );
    }
}

#[test]
fn unknown_mode_names_are_linear() {
    for name in ["", "G1", "linear", "foo", "g1 ", "g"] {
        assert_eq!(Mode::from_js_name(name), Mode::Linear, "{name:?}");
    }
    assert_eq!(Mode::from_js_name("g1"), Mode::G1);
}

#[test]
fn n_zero_gives_one_nan_point_per_patch_and_no_triangles() {
    let (v, f) = cube();
    let m = build_model(&v, &f, &BuildOptions { n: 0, ..opts() }).unwrap();
    assert_eq!(m.mesh.pos.len(), 6);
    assert!(m.mesh.tris.is_empty());
    assert!(m.mesh.pos.iter().flatten().all(|x| x.is_nan()));
}

#[test]
fn creases_on_missing_edges_are_ignored() {
    let (v, f) = cube();
    let plain = build_model(&v, &f, &opts()).unwrap();
    let extra = BuildOptions {
        creases: [EdgeKey::new(0, 6), EdgeKey::new(2, 99)].into(),
        ..opts()
    };
    assert_eq!(plain, build_model(&v, &f, &extra).unwrap());
}

#[test]
fn edge_point_with_out_of_range_edge_is_none() {
    let m = build_model(&quad_v(), &[vec![0, 1, 2, 3]], &opts()).unwrap();
    assert!(m.patches[0].edge_point(3, 0.5, 1e-6).is_some());
    assert!(m.patches[0].edge_point(4, 0.5, 1e-6).is_none());
    let t = build_model(&TRI_V, &[vec![0, 1, 2]], &opts()).unwrap();
    assert!(t.patches[0].edge_point(2, 0.5, 1e-6).is_some());
    assert!(t.patches[0].edge_point(3, 0.5, 1e-6).is_none());
    assert!(t.patches[0].edge_point(usize::MAX, 0.5, 1e-6).is_none());
}
