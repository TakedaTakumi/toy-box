use gregory_core::{CageModel, Edge, EdgeKey, Preset, SelectMode, Vec3, bezier3, dot, norm, sub};
use gregory_editor::camera::{OrbitCamera, Projected, ScreenView};
use gregory_editor::editing::gizmo::{ScreenPos, segment_distance};
use gregory_editor::editing::pick::{
    BACK_FACE_LIMIT, EDGE_PICK_RADIUS, EDGE_SAMPLES, EdgeRuns, SurfaceScreen, VERTEX_PICK_RADIUS,
    edge_screen_runs, facing, front_face, nearest_edge, nearest_point, pick_element,
    surface_screen, vertex_screen_points,
};
use gregory_editor::editing::selection::Element;
use gregory_editor::scene::{Scene, build_scene};
use proptest::prelude::*;

fn bad() -> impl Strategy<Value = f64> {
    prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)]
}

fn wild() -> impl Strategy<Value = f64> {
    prop_oneof![6 => -3.0f64..3.0, 1 => bad(), 1 => Just(f64::MAX), 1 => Just(1e300)]
}

fn idx() -> impl Strategy<Value = usize> {
    prop_oneof![6 => 0usize..6, 1 => 6usize..20, 1 => Just(usize::MAX)]
}

fn pos_strategy() -> impl Strategy<Value = ScreenPos> {
    [-50.0f64..450.0, -50.0f64..450.0]
}

fn dist(a: ScreenPos, b: ScreenPos) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn cube() -> (Scene, ScreenView) {
    let scene = build_scene(&Preset::Cube.make(), 6).expect("立方体は組み立てられる");
    let view = ScreenView::of(&OrbitCamera::default(), 800.0, 600.0);
    (scene, view)
}

fn proj(x: f64, y: f64, depth: f64) -> Option<Projected> {
    Some(Projected { x, y, depth })
}

proptest! {
    #[test]
    fn nearest_point_is_the_closest_below_the_radius(
        pts in proptest::collection::vec(proptest::option::of(pos_strategy()), 0..20),
        pos in pos_strategy(), radius in 0.0f64..60.0,
    ) {
        let mut best: Option<(usize, f64)> = None;
        for (i, p) in pts.iter().enumerate() {
            if let Some(p) = p {
                let d = dist(*p, pos);
                if d < radius && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
        }
        prop_assert_eq!(nearest_point(&pts, pos, radius), best.map(|b| b.0));
    }

    #[test]
    fn nearest_point_respects_the_vertex_radius_exactly(angle in 0.0f64..std::f64::consts::TAU, margin in 0.001f64..5.0, inside in any::<bool>()) {
        let center = [200.0, 150.0];
        let r = if inside { VERTEX_PICK_RADIUS - margin } else { VERTEX_PICK_RADIUS + margin };
        let pos = [center[0] + r * angle.cos(), center[1] + r * angle.sin()];
        prop_assert_eq!(nearest_point(&[Some(center)], pos, VERTEX_PICK_RADIUS).is_some(), inside);
    }

    #[test]
    fn nearest_point_rejects_non_finite_positions_and_skips_non_finite_points(
        b in bad(), first in any::<bool>(), pts in proptest::collection::vec(proptest::option::of(pos_strategy()), 0..6),
    ) {
        let pos = if first { [b, 1.0] } else { [1.0, b] };
        prop_assert_eq!(nearest_point(&pts, pos, 1e9), None);
        let with_bad = [Some([b, 0.0]), Some([0.0, b]), Some([5.0, 5.0])];
        prop_assert_eq!(nearest_point(&with_bad, [6.0, 5.0], 13.0), Some(2));
    }

    #[test]
    fn nearest_edge_is_the_closest_run_below_the_radius(
        runs in proptest::collection::vec(proptest::collection::vec(proptest::collection::vec(pos_strategy(), 0..5), 0..3), 0..5),
        pos in pos_strategy(), radius in 0.0f64..60.0,
    ) {
        let edges: Vec<EdgeRuns> = runs.into_iter().enumerate().map(|(i, r)| EdgeRuns { key: EdgeKey::new(i, i + 100), runs: r }).collect();
        let mut best: Option<(EdgeKey, f64)> = None;
        for e in &edges {
            for run in &e.runs {
                for w in run.windows(2) {
                    let d = segment_distance(w[0], w[1], pos);
                    if d < radius && best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((e.key, d));
                    }
                }
            }
        }
        prop_assert_eq!(nearest_edge(&edges, pos, radius), best.map(|b| b.0));
    }

    #[test]
    fn nearest_edge_rejects_non_finite_positions(b in bad(), first in any::<bool>()) {
        let edges = [EdgeRuns { key: EdgeKey::new(0, 1), runs: vec![vec![[0.0, 0.0], [10.0, 0.0]]] }];
        let pos = if first { [b, 0.0] } else { [5.0, b] };
        prop_assert_eq!(nearest_edge(&edges, pos, 1e9), None);
    }

    #[test]
    fn front_face_prefers_the_nearest_front_facing_triangle(d1 in 1.0f64..50.0, d2 in 1.0f64..50.0, d3 in 1.0f64..50.0, u in 0.05f64..0.45, v in 0.05f64..0.45) {
        prop_assume!((d1 - d2).abs() > 1e-6);
        let (px, py) = (u * 10.0, v * 10.0);
        let tri = |d: f64| [proj(0.0, 0.0, d), proj(10.0, 0.0, d), proj(0.0, 10.0, d)];
        let mut points = Vec::new();
        for d in [d1, d2, d3] {
            points.extend(tri(d));
        }
        let surface = SurfaceScreen { points, facing_z: vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 0.9, 0.9, 0.9] };
        let tris = [[0, 1, 2], [3, 4, 5], [6, 7, 8]];
        let got = front_face(&surface, &tris, &[10, 20, 30], [px, py]);
        prop_assert_eq!(got, Some(if d1 < d2 { 10 } else { 20 }), "3つ目は裏向きなので手前でも選ばれない");
    }

    #[test]
    fn front_face_back_face_limit_is_exclusive_and_nan_is_back(z in -1.0f64..1.0) {
        let surface = SurfaceScreen {
            points: vec![proj(0.0, 0.0, 1.0), proj(10.0, 0.0, 1.0), proj(0.0, 10.0, 1.0)],
            facing_z: vec![z; 3],
        };
        let hit = front_face(&surface, &[[0, 1, 2]], &[0], [2.0, 2.0]);
        if z < BACK_FACE_LIMIT - 1e-9 {
            prop_assert_eq!(hit, Some(0));
        } else if z > BACK_FACE_LIMIT + 1e-9 {
            prop_assert_eq!(hit, None);
        }
        let nan = SurfaceScreen { facing_z: vec![f64::NAN; 3], ..surface };
        prop_assert_eq!(front_face(&nan, &[[0, 1, 2]], &[0], [2.0, 2.0]), None);
    }

    #[test]
    fn front_face_never_panics_on_malformed_input(
        tris in proptest::collection::vec([idx(), idx(), idx()], 0..8),
        tri_face in proptest::collection::vec(0usize..5, 0..8),
        npoints in 0usize..6, nz in 0usize..6,
        pos in [wild(), wild()],
        depth in prop_oneof![1.0f64..5.0, bad()],
    ) {
        let points = (0..npoints).map(|i| if i % 3 == 2 { None } else { proj(i as f64 * 3.0, (i * i) as f64, depth) }).collect();
        let surface = SurfaceScreen { points, facing_z: vec![-0.5; nz] };
        let got = front_face(&surface, &tris, &tri_face, pos);
        if !pos.iter().all(|c| c.is_finite()) {
            prop_assert_eq!(got, None);
        }
    }

    #[test]
    fn vertex_screen_points_handles_short_normals_and_bad_positions(
        pts in proptest::collection::vec([wild(), wild(), wild()], 0..8),
        nrm in proptest::collection::vec([wild(), wild(), wild()], 0..8),
    ) {
        let view = ScreenView::of(&OrbitCamera::default(), 800.0, 600.0);
        let out = vertex_screen_points(&view, &pts, &nrm);
        prop_assert_eq!(out.len(), pts.len());
        for (i, o) in out.iter().enumerate() {
            if i >= nrm.len() || !pts[i].iter().all(|c| c.is_finite()) {
                prop_assert!(o.is_none());
            }
            if let Some(p) = o {
                prop_assert!(p.iter().all(|c| c.is_finite()));
            }
        }
    }

    #[test]
    fn edge_runs_never_panic_and_only_hold_finite_points(
        e in proptest::array::uniform4([wild(), wild(), wild()]),
        a in prop_oneof![0usize..3, Just(usize::MAX)], b in 0usize..5,
    ) {
        let view = ScreenView::of(&OrbitCamera::default(), 800.0, 600.0);
        let nrm = [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let edge = Edge { a, b, faces: vec![], e, crease: false };
        let runs = edge_screen_runs(&view, &[edge], &nrm);
        prop_assert_eq!(runs.len(), 1);
        prop_assert_eq!(runs[0].key, EdgeKey::new(a, b));
        for run in &runs[0].runs {
            prop_assert!(!run.is_empty() && run.len() <= EDGE_SAMPLES + 1);
            prop_assert!(run.iter().flatten().all(|c| c.is_finite()));
        }
    }

    #[test]
    fn facing_matches_the_dot_product_sign(p in [-3.0f64..3.0, -3.0f64..3.0, -3.0f64..3.0], n in [-1.0f64..1.0, -1.0f64..1.0, -1.0f64..1.0]) {
        let view = ScreenView::of(&OrbitCamera::default(), 800.0, 600.0);
        prop_assert_eq!(facing(&view, p, n), dot(n, sub(view.frame().eye, p)) > 0.0);
    }

    #[test]
    fn every_mode_rejects_non_finite_positions(b in bad(), first in any::<bool>()) {
        let (scene, view) = cube();
        let pos = if first { [b, 300.0] } else { [400.0, b] };
        for mode in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
            prop_assert_eq!(pick_element(&view, &scene.cage, &scene.model, mode, pos), None);
        }
    }

    #[test]
    fn picking_never_panics_for_any_view_and_position(
        dx in -800.0f32..800.0, dy in -800.0f32..800.0, w in 1.0f32..2000.0, h in 1.0f32..2000.0,
        x in -3000.0f64..3000.0, y in -3000.0f64..3000.0,
    ) {
        let (scene, _) = cube();
        let mut cam = OrbitCamera::default();
        cam.orbit(dx, dy);
        let view = ScreenView::of(&cam, w, h);
        for mode in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
            let _ = pick_element(&view, &scene.cage, &scene.model, mode, [x, y]);
        }
    }

    #[test]
    fn picked_vertex_is_within_the_radius_and_the_closest_visible(
        dx in -800.0f32..800.0, dy in -300.0f32..300.0, x in 0.0f64..800.0, y in 0.0f64..600.0,
    ) {
        let (scene, _) = cube();
        let mut cam = OrbitCamera::default();
        cam.orbit(dx, dy);
        let view = ScreenView::of(&cam, 800.0, 600.0);
        let pts = vertex_screen_points(&view, &scene.cage.v, &scene.model.nrm);
        let got = pick_element(&view, &scene.cage, &scene.model, SelectMode::Vertex, [x, y]);
        let closest = pts.iter().flatten().map(|p| dist(*p, [x, y])).fold(f64::INFINITY, f64::min);
        match got {
            Some(Element::Vertex(i)) => {
                let d = dist(pts[i].expect("表向きの頂点だけ選ばれる"), [x, y]);
                prop_assert!(d < VERTEX_PICK_RADIUS && d <= closest);
            }
            None => prop_assert!(closest >= VERTEX_PICK_RADIUS),
            other => prop_assert!(false, "{other:?}"),
        }
    }
}

#[test]
fn clicking_each_visible_cube_vertex_in_the_default_view_picks_it() {
    let (scene, view) = cube();
    let pts = vertex_screen_points(&view, &scene.cage.v, &scene.model.nrm);
    let visible: Vec<usize> = (0..pts.len()).filter(|&i| pts[i].is_some()).collect();
    assert!(
        visible.len() >= 4,
        "既定の視点では立方体の頂点が4つ以上見える: {visible:?}"
    );
    for &i in &visible {
        let p = pts[i].expect("visible");
        for (ox, oy) in [(0.0, 0.0), (5.0, 5.0), (-8.0, 3.0)] {
            let got = pick_element(
                &view,
                &scene.cage,
                &scene.model,
                SelectMode::Vertex,
                [p[0] + ox, p[1] + oy],
            );
            assert_eq!(
                got,
                Some(Element::Vertex(i)),
                "頂点 {i} を ({ox}, {oy}) ずれでクリック"
            );
        }
        let far = pick_element(
            &view,
            &scene.cage,
            &scene.model,
            SelectMode::Vertex,
            [p[0] + 14.0, p[1]],
        );
        assert_ne!(
            far,
            Some(Element::Vertex(i)),
            "半径 13 以上離れたら選ばれない"
        );
    }
}

#[test]
fn hidden_cube_vertices_are_not_picked_at_their_own_position() {
    let (scene, view) = cube();
    let pts = vertex_screen_points(&view, &scene.cage.v, &scene.model.nrm);
    for (i, v) in scene
        .cage
        .v
        .iter()
        .enumerate()
        .filter(|(i, _)| pts[*i].is_none())
    {
        let s = view.project(*v).expect("画面には射影できる");
        let got = pick_element(&view, &scene.cage, &scene.model, SelectMode::Vertex, s.xy());
        assert_ne!(got, Some(Element::Vertex(i)), "裏向きの頂点 {i}");
    }
}

#[test]
fn clicking_the_middle_of_a_visible_edge_picks_an_edge_touching_that_point() {
    let (scene, view) = cube();
    let runs = edge_screen_runs(&view, &scene.model.edges, &scene.model.nrm);
    let mut tried = 0;
    for er in runs.iter().filter(|r| !r.runs.is_empty()) {
        let longest = er.runs.iter().max_by_key(|r| r.len()).expect("非空");
        let p = longest[longest.len() / 2];
        let got = pick_element(&view, &scene.cage, &scene.model, SelectMode::Edge, p);
        let Some(Element::Edge(key)) = got else {
            panic!("辺 {} の中点 {p:?} で辺が選ばれない: {got:?}", er.key)
        };
        let touching = runs.iter().find(|r| r.key == key).expect("実在する辺");
        let d = touching
            .runs
            .iter()
            .flat_map(|r| r.windows(2))
            .map(|w| segment_distance(w[0], w[1], p))
            .fold(f64::INFINITY, f64::min);
        assert!(d < 1e-6, "選ばれた辺 {key} は点を通る");
        tried += 1;
    }
    assert!(tried >= 3, "既定の視点では辺が3本以上見える");
}

#[test]
fn clicking_the_projected_center_of_a_facing_cube_face_picks_that_face() {
    let (scene, view) = cube();
    let eye = view.frame().eye;
    let mut tried = 0;
    for (fi, face) in scene.cage.f.iter().enumerate() {
        let c: Vec3 = std::array::from_fn(|k| {
            face.iter().map(|&v| scene.cage.v[v][k]).sum::<f64>() / face.len() as f64
        });
        let outward = norm(c);
        if dot(outward, norm(sub(eye, c))) < 0.3 {
            continue;
        }
        let s = view.project(c).expect("手前");
        let got = pick_element(&view, &scene.cage, &scene.model, SelectMode::Face, s.xy());
        assert_eq!(got, Some(Element::Face(fi)), "面 {fi}");
        tried += 1;
    }
    assert!(
        tried >= 2,
        "既定の視点では立方体の面が2つ以上はっきり見える"
    );
}

#[test]
fn clicking_empty_space_picks_nothing_in_every_mode() {
    let (scene, view) = cube();
    for mode in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
        for pos in [[2.0, 2.0], [790.0, 590.0], [-500.0, 300.0], [400.0, 1e9]] {
            assert_eq!(
                pick_element(&view, &scene.cage, &scene.model, mode, pos),
                None,
                "{mode:?} {pos:?}"
            );
        }
    }
}

#[test]
fn malformed_model_does_not_panic() {
    let (mut scene, view) = cube();
    scene.model.mesh.tris.push([999, 1000, usize::MAX]);
    scene.model.mesh.tri_face.truncate(2);
    scene.model.mesh.nrm.truncate(3);
    scene.model.nrm.truncate(2);
    let mut cage: CageModel = scene.cage.clone();
    cage.v[0] = [f64::NAN, 0.0, 0.0];
    for mode in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
        for pos in [[400.0, 300.0], [0.0, 0.0], [f64::NAN, 1.0]] {
            let _ = pick_element(&view, &cage, &scene.model, mode, pos);
        }
    }
    let surface = surface_screen(&view, &scene.model.mesh);
    assert_eq!(surface.points.len(), scene.model.mesh.pos.len());
    assert_eq!(surface.facing_z.len(), scene.model.mesh.pos.len());
    assert!(
        surface.facing_z[3..].iter().all(|z| z.is_nan()),
        "法線のない頂点の向きは NaN"
    );
}

#[test]
fn edge_curve_samples_are_bezier_points() {
    let (scene, view) = cube();
    let e = &scene.model.edges[0];
    let runs = edge_screen_runs(&view, std::slice::from_ref(e), &scene.model.nrm);
    for p in runs[0].runs.iter().flatten() {
        assert!((0..=EDGE_SAMPLES).any(|i| {
            let s = view
                .project(bezier3(&e.e, i as f64 / EDGE_SAMPLES as f64))
                .expect("手前");
            dist(s.xy(), *p) < 1e-9
        }));
    }
    assert_eq!(
        (VERTEX_PICK_RADIUS, EDGE_PICK_RADIUS, BACK_FACE_LIMIT),
        (13.0, 10.0, 0.06)
    );
}
