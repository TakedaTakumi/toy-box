mod common;

use common::{close, close3};
use gregory_core::*;
use proptest::prelude::*;
use std::collections::HashSet;

// ---- 入力の生成 ----

fn jp(x: f64, z: f64, y: f64) -> Vec3 {
    // JS の PRESETS が通す変換 [x, y, z] -> [x, z, -y] を、あらかじめ適用した形で持つ
    [x, z, -y]
}

const CUBE_V: [[f64; 3]; 8] = [
    [-1., -1., -1.],
    [1., -1., -1.],
    [1., 1., -1.],
    [-1., 1., -1.],
    [-1., -1., 1.],
    [1., -1., 1.],
    [1., 1., 1.],
    [-1., 1., 1.],
];

fn preset(i: usize) -> (Vec<Vec3>, Vec<Vec<usize>>) {
    match i {
        0 => (
            CUBE_V.iter().map(|p| jp(p[0], p[2], p[1])).collect(),
            vec![
                vec![4, 5, 6, 7],
                vec![3, 2, 1, 0],
                vec![0, 1, 5, 4],
                vec![2, 3, 7, 6],
                vec![1, 2, 6, 5],
                vec![3, 0, 4, 7],
            ],
        ),
        1 => {
            let mut v = Vec::new();
            for j in 0..3 {
                for i in 0..3 {
                    v.push(jp(i as f64 - 1.0, 0.0, j as f64 - 1.0));
                }
            }
            let mut f = Vec::new();
            for j in 0..2 {
                for i in 0..2 {
                    f.push(vec![
                        j * 3 + i,
                        j * 3 + i + 1,
                        (j + 1) * 3 + i + 1,
                        (j + 1) * 3 + i,
                    ]);
                }
            }
            (v, f)
        }
        2 => (
            [
                [-1., -1., -1.],
                [1., -1., -1.],
                [1., 1., -1.],
                [-1., 1., -1.],
                [0., 0., 1.2],
            ]
            .iter()
            .map(|p| jp(p[0], p[2], p[1]))
            .collect(),
            vec![
                vec![3, 2, 1, 0],
                vec![0, 1, 4],
                vec![1, 2, 4],
                vec![2, 3, 4],
                vec![3, 0, 4],
            ],
        ),
        _ => (
            [
                [-1.1, -0.7, -1.],
                [1.1, -0.7, -1.],
                [0., 1., -1.],
                [-1.1, -0.7, 1.],
                [1.1, -0.7, 1.],
                [0., 1., 1.],
            ]
            .iter()
            .map(|p| jp(p[0], p[2], p[1]))
            .collect(),
            vec![
                vec![3, 4, 5],
                vec![2, 1, 0],
                vec![0, 1, 4, 3],
                vec![1, 2, 5, 4],
                vec![2, 0, 3, 5],
            ],
        ),
    }
}

fn edge_keys(f: &[Vec<usize>]) -> Vec<EdgeKey> {
    let mut keys = Vec::new();
    for face in f {
        for k in 0..face.len() {
            let key = EdgeKey::new(face[k], face[(k + 1) % face.len()]);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    keys
}

fn opts(tau: f64, mode: Mode, n: usize, creases: HashSet<EdgeKey>) -> BuildOptions {
    BuildOptions {
        tau,
        mode,
        n,
        creases,
    }
}

/// プリセットに ±0.15 のゆらぎと折れ目をつけたケージ。
#[derive(Clone, Debug)]
struct Shaken {
    is_plane: bool,
    v: Vec<Vec3>,
    f: Vec<Vec<usize>>,
    creases: HashSet<EdgeKey>,
}

fn shaken() -> impl Strategy<Value = Shaken> {
    (
        0..4usize,
        prop::collection::vec(-0.15f64..=0.15, 30),
        prop::collection::vec(any::<bool>(), 40),
    )
        .prop_map(|(i, j, flags)| {
            let (mut v, f) = preset(i);
            for (vi, p) in v.iter_mut().enumerate() {
                for k in 0..3 {
                    p[k] += j[(vi * 3 + k) % j.len()];
                }
            }
            let creases = edge_keys(&f)
                .into_iter()
                .enumerate()
                .filter(|(i, _)| flags[i % flags.len()])
                .map(|(_, k)| k)
                .collect();
            Shaken {
                is_plane: i == 1,
                v,
                f,
                creases,
            }
        })
}

fn any_coord() -> BoxedStrategy<f64> {
    prop_oneof![
        3 => -10.0f64..10.0,
        1 => any::<f64>(),
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
    ]
    .boxed()
}

fn any_tau() -> impl Strategy<Value = f64> {
    prop_oneof![3 => -3.0f64..3.0, 1 => any::<f64>(), 1 => Just(f64::NAN), 1 => Just(f64::INFINITY), 1 => Just(0.0)]
}

fn any_mode() -> impl Strategy<Value = Mode> {
    prop_oneof![Just(Mode::G1), Just(Mode::Linear)]
}

fn vec3_of(c: impl Strategy<Value = f64> + Clone) -> impl Strategy<Value = Vec3> {
    [c.clone(), c.clone(), c].prop_map(|a| a)
}

/// 頂点数と、面（長さ 0〜6、番号は範囲外を含む）。
fn arbitrary_cage() -> impl Strategy<Value = (Vec<Vec3>, Vec<Vec<usize>>)> {
    (0..8usize).prop_flat_map(|vn| {
        (
            prop::collection::vec(vec3_of(any_coord()), vn),
            prop::collection::vec(prop::collection::vec(0..vn + 3, 0..=6), 0..6),
        )
    })
}

fn any_creases() -> impl Strategy<Value = HashSet<EdgeKey>> {
    prop::collection::vec((0..12usize, 0..12usize), 0..8)
        .prop_map(|p| p.into_iter().map(|(a, b)| EdgeKey::new(a, b)).collect())
}

/// 3〜4頂点の面だけで、番号が範囲内のケージ。座標は有限。
fn valid_cage(coord: std::ops::Range<f64>) -> impl Strategy<Value = (Vec<Vec3>, Vec<Vec<usize>>)> {
    (3..9usize).prop_flat_map(move |vn| {
        (
            prop::collection::vec(vec3_of(coord.clone()), vn),
            prop::collection::vec(prop::collection::vec(0..vn, 3..=4), 0..6),
        )
    })
}

fn fingerprint(r: &Result<Model, GregoryError>) -> String {
    format!("{r:?}")
}

// ---- 1. panic しない ----

proptest! {
    #[test]
    fn build_model_never_panics_and_returns_ok_or_err(
        (v, f) in arbitrary_cage(), tau in any_tau(), mode in any_mode(), n in 0..=6usize, creases in any_creases(),
    ) {
        let _ = build_model(&v, &f, &opts(tau, mode, n, creases));
    }
}

// ---- 2. Err の条件 ----

proptest! {
    #[test]
    fn out_of_range_vertex_always_gives_vertex_out_of_range(
        (v, f) in arbitrary_cage(), tau in any_tau(), n in 0..=3usize,
    ) {
        let out_of_range = f.iter().any(|face| face.iter().any(|&x| x >= v.len()));
        let r = build_model(&v, &f, &opts(tau, Mode::G1, n, HashSet::new()));
        if out_of_range {
            let is_oor = matches!(r, Err(GregoryError::VertexOutOfRange { .. }));
            prop_assert!(is_oor);
        } else if f.iter().any(|face| face.len() < 3) {
            let is_few = matches!(r, Err(GregoryError::TooFewCorners { .. }));
            prop_assert!(is_few);
        }
    }

    #[test]
    fn faces_of_three_or_four_with_valid_indices_always_succeed(
        (v, f) in valid_cage(-10.0..10.0), tau in any_tau(), mode in any_mode(), n in 0..=4usize, creases in any_creases(),
    ) {
        prop_assert!(build_model(&v, &f, &opts(tau, mode, n, creases)).is_ok());
    }

    #[test]
    fn nonfinite_coordinates_with_valid_faces_still_succeed(
        vn in 3..8usize, seed in prop::collection::vec(any_coord(), 24), faces in prop::collection::vec(prop::collection::vec(0..8usize, 3..=4), 0..5),
    ) {
        let v: Vec<Vec3> = (0..vn).map(|i| [seed[i * 3], seed[i * 3 + 1], seed[i * 3 + 2]]).collect();
        let f: Vec<Vec<usize>> = faces.into_iter().map(|face| face.into_iter().map(|x| x % vn).collect()).collect();
        prop_assert!(build_model(&v, &f, &opts(1.0, Mode::G1, 2, HashSet::new())).is_ok());
    }
}

// ---- 3. メッシュの個数 ----

proptest! {
    #[test]
    fn mesh_sizes_follow_patch_kind_and_n(
        (v, f) in valid_cage(-10.0..10.0), tau in -2.0f64..2.0, mode in any_mode(), n in 0..=6usize, creases in any_creases(),
    ) {
        let m = build_model(&v, &f, &opts(tau, mode, n, creases)).unwrap();
        let (mut pts, mut tris) = (0usize, 0usize);
        for face in &f {
            if face.len() == 4 {
                pts += (n + 1) * (n + 1);
                tris += 2 * n * n;
            } else {
                pts += (n + 1) * (n + 2) / 2;
                tris += n * n;
            }
        }
        prop_assert_eq!(m.mesh.pos.len(), pts);
        prop_assert_eq!(m.mesh.nrm.len(), pts);
        prop_assert_eq!(m.mesh.tris.len(), tris);
        prop_assert_eq!(m.mesh.tri_face.len(), m.mesh.tris.len());
        prop_assert!(m.mesh.tris.iter().all(|t| t.iter().all(|&i| i < m.mesh.pos.len())));
        prop_assert!(m.mesh.tri_face.iter().all(|&fi| fi < f.len()));
        prop_assert_eq!(m.patches.len(), f.len());
        prop_assert_eq!(m.count.q + m.count.t, f.len());
    }

    #[test]
    fn n_zero_gives_one_point_per_patch_and_no_triangles(
        (v, f) in valid_cage(-10.0..10.0), mode in any_mode(),
    ) {
        let m = build_model(&v, &f, &opts(1.0, mode, 0, HashSet::new())).unwrap();
        prop_assert_eq!(m.mesh.pos.len(), f.len());
        prop_assert!(m.mesh.tris.is_empty());
    }
}

// ---- 4. 境界と辺曲線の一致 ----

fn scale_of(v: &[Vec3]) -> f64 {
    1.0 + v.iter().flatten().fold(0.0f64, |m, x| m.max(x.abs()))
}

proptest! {
    #[test]
    fn patch_boundary_equals_edge_curve_on_random_cages(
        (v, f) in valid_cage(-10.0..10.0), tau in 0.0f64..2.0, mode in any_mode(), t in 0.0f64..=1.0,
    ) {
        let m = build_model(&v, &f, &opts(tau, mode, 1, HashSet::new())).unwrap();
        let tol = 1e-12 * scale_of(&v);
        for p in &m.patches {
            let want = bezier3(&p.ed()[0].e, t);
            prop_assert!(close3(p.at(t, 0.0), want, tol), "got {:?} want {:?}", p.at(t, 0.0), want);
        }
    }

    #[test]
    fn patch_boundary_equals_edge_curve_on_shaken_presets(c in shaken(), t in 0.0f64..=1.0) {
        let m = build_model(&c.v, &c.f, &opts(1.0, Mode::G1, 1, c.creases.clone())).unwrap();
        let tol = 1e-12 * scale_of(&c.v);
        for p in &m.patches {
            let want = bezier3(&p.ed()[0].e, t);
            prop_assert!(close3(p.at(t, 0.0), want, tol), "got {:?} want {:?}", p.at(t, 0.0), want);
        }
    }

    #[test]
    fn bezier_endpoints_are_the_end_control_points(e in prop::array::uniform4(vec3_of(-1e6f64..1e6))) {
        prop_assert_eq!(bezier3(&e, 0.0), e[0]);
        prop_assert_eq!(bezier3(&e, 1.0), e[3]);
    }
}

// ---- 5. G0 / G1 ----

proptest! {
    #[test]
    fn shaken_presets_are_g0_and_smooth_edges_are_g1(c in shaken()) {
        let m = build_model(&c.v, &c.f, &opts(1.0, Mode::G1, 8, c.creases.clone())).unwrap();
        prop_assert!(m.check.g0 < 1e-9, "g0={}", m.check.g0);
        // 平面にほぼ重なる面の間の折れ目では、JS 版でも G1 の検証が 180 度になることがある
        prop_assert!(c.is_plane || m.check.pairs == 0 || m.check.g1 < 0.05, "g1={}", m.check.g1);
    }

    #[test]
    fn linear_mode_is_still_g0(c in shaken()) {
        let m = build_model(&c.v, &c.f, &opts(1.0, Mode::Linear, 8, c.creases.clone())).unwrap();
        prop_assert!(m.check.g0 < 1e-9, "g0={}", m.check.g0);
    }
}

// ---- 6, 7. 平行移動・一様拡大 ----

fn vec_close(a: &[Vec3], b: &[Vec3], tol: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| close3(*x, *y, tol))
}

/// 長さが 1e-12 未満で 0 にされる法線は、絶対値の閾値のため拡大縮小で変わりうるので除いて比べる。
fn nonzero_normals_close(a: &[Vec3], b: &[Vec3], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| *x == [0.0; 3] || *y == [0.0; 3] || close3(*x, *y, tol))
}

proptest! {
    #[test]
    fn translation_shifts_positions_and_keeps_normals(c in shaken(), d in vec3_of(-50.0f64..50.0)) {
        let o = opts(1.0, Mode::G1, 3, c.creases.clone());
        let base = build_model(&c.v, &c.f, &o).unwrap();
        let moved_v: Vec<Vec3> = c.v.iter().map(|p| [p[0] + d[0], p[1] + d[1], p[2] + d[2]]).collect();
        let moved = build_model(&moved_v, &c.f, &o).unwrap();
        let expect: Vec<Vec3> = base.mesh.pos.iter().map(|p| [p[0] + d[0], p[1] + d[1], p[2] + d[2]]).collect();
        prop_assert!(vec_close(&moved.mesh.pos, &expect, 1e-9 * 60.0));
        // 角では差分が小さくなり、桁落ちが 1/h 倍に増幅されるため、法線の許容は緩い
        prop_assert!(nonzero_normals_close(&moved.mesh.nrm, &base.mesh.nrm, 1e-4));
        prop_assert!(vec_close(&moved.nrm, &base.nrm, 1e-9));
    }

    #[test]
    fn uniform_scaling_scales_positions_and_keeps_normals(c in shaken(), s in 0.1f64..10.0) {
        let o = opts(1.0, Mode::G1, 3, c.creases.clone());
        let base = build_model(&c.v, &c.f, &o).unwrap();
        let big_v: Vec<Vec3> = c.v.iter().map(|p| [p[0] * s, p[1] * s, p[2] * s]).collect();
        let big = build_model(&big_v, &c.f, &o).unwrap();
        let expect: Vec<Vec3> = base.mesh.pos.iter().map(|p| [p[0] * s, p[1] * s, p[2] * s]).collect();
        prop_assert!(vec_close(&big.mesh.pos, &expect, 1e-9 * 10.0));
        prop_assert!(nonzero_normals_close(&big.mesh.nrm, &base.mesh.nrm, 1e-4));
        prop_assert!(vec_close(&big.nrm, &base.nrm, 1e-9));
    }
}

// ---- 8. 決定性 ----

proptest! {
    #[test]
    fn same_input_gives_identical_output(
        (v, f) in arbitrary_cage(), tau in any_tau(), mode in any_mode(), n in 0..=4usize, creases in any_creases(),
    ) {
        let o = opts(tau, mode, n, creases);
        // NaN を含むため、Debug 表記の一致で完全一致を見る
        prop_assert_eq!(fingerprint(&build_model(&v, &f, &o)), fingerprint(&build_model(&v, &f, &o)));
    }
}

// ---- 9. ベクトル ----

fn finite_any() -> BoxedStrategy<f64> {
    (prop::num::f64::NORMAL | prop::num::f64::SUBNORMAL | prop::num::f64::ZERO).boxed()
}

proptest! {
    #[test]
    fn norm_is_unit_or_zero(a in vec3_of(-1e300f64..1e300)) {
        let n = norm(a);
        let l = len(n);
        let ok = if len(a) < 1e-12 { n == [0.0; 3] } else { close(l, 1.0, 1e-12) };
        prop_assert!(ok, "a={a:?} norm={n:?} len={l}");
    }

    #[test]
    fn norm_of_tiny_or_zero_vector_is_zero(a in vec3_of(-1e-13f64..1e-13)) {
        prop_assert_eq!(norm(a), [0.0; 3]);
    }

    #[test]
    fn cross_is_orthogonal_to_both_inputs(a in vec3_of(-1e50f64..1e50), b in vec3_of(-1e50f64..1e50)) {
        let c = cross(a, b);
        let (la, lb) = (len(a), len(b));
        prop_assert!(dot(c, a).abs() <= 1e-12 * la * la * lb + f64::MIN_POSITIVE);
        prop_assert!(dot(c, b).abs() <= 1e-12 * la * lb * lb + f64::MIN_POSITIVE);
    }

    #[test]
    fn dot_is_symmetric(a in vec3_of(-1e100f64..1e100), b in vec3_of(-1e100f64..1e100)) {
        prop_assert_eq!(dot(a, b), dot(b, a));
    }

    #[test]
    fn len_does_not_overflow_for_large_finite_coordinates(a in vec3_of(-1e300f64..1e300)) {
        let l = len(a);
        let m = a.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        prop_assert!(l.is_finite());
        prop_assert!(l >= m * (1.0 - 1e-15) && l <= m * 1.7321 * (1.0 + 1e-15), "l={l} m={m}");
    }

    #[test]
    fn len_is_never_nan_for_finite_inputs(a in vec3_of(finite_any())) {
        prop_assert!(!len(a).is_nan());
    }

    #[test]
    fn len_with_an_infinity_is_infinite_even_with_nan(
        k in 0..3usize, sign in prop::bool::ANY, other in vec3_of(prop_oneof![Just(f64::NAN), -1e3f64..1e3]),
    ) {
        let mut a = other;
        a[k] = if sign { f64::INFINITY } else { f64::NEG_INFINITY };
        prop_assert_eq!(len(a), f64::INFINITY);
    }
}

// ---- 10. Mode::from_js_name ----

proptest! {
    #[test]
    fn only_g1_name_gives_g1(s in prop_oneof![any::<String>(), "[gG1 ]{0,4}", Just("g1".to_string())]) {
        prop_assert_eq!(Mode::from_js_name(&s) == Mode::G1, s == "g1");
    }
}

// ---- 11. 辺の表にない折れ目は無視される ----

proptest! {
    #[test]
    fn creases_not_on_any_edge_are_ignored(c in shaken(), extra in prop::collection::vec((0..30usize, 0..30usize), 1..6)) {
        let table: HashSet<EdgeKey> = edge_keys(&c.f).into_iter().collect();
        let mut more = c.creases.clone();
        more.extend(extra.into_iter().map(|(a, b)| EdgeKey::new(a, b)).filter(|k| !table.contains(k)));
        let a = build_model(&c.v, &c.f, &opts(1.0, Mode::G1, 3, c.creases.clone()));
        let b = build_model(&c.v, &c.f, &opts(1.0, Mode::G1, 3, more));
        prop_assert_eq!(fingerprint(&a), fingerprint(&b));
    }
}
