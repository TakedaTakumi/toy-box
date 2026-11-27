use gregory_core::{Vec3, add, dot, mul, sub};
use gregory_editor::camera::{CameraFrame, NEAR, OrbitCamera, Ray, ScreenView};
use gregory_editor::editing::gizmo::{
    AXIS_PICK_RADIUS, Axis, Constraint, GIZMO_PX, Gizmo, Handle, PLANE_INNER, PLANE_OUTER, Plane,
    ScreenPos, constraint_for_lock, gizmo, move_delta, pick_handle, plane_point, point_in_quad,
    ray_plane, segment_distance, selection_center,
};
use proptest::prelude::*;

fn camera() -> impl Strategy<Value = OrbitCamera> {
    (-500.0f32..500.0, -500.0f32..500.0, 0.2f64..5.0).prop_map(|(dx, dy, z)| {
        let mut c = OrbitCamera::default();
        c.orbit(dx, dy);
        c.zoom(z);
        c
    })
}

fn size() -> impl Strategy<Value = (f32, f32)> {
    (400.0f32..1600.0, 400.0f32..1600.0)
}

fn center() -> impl Strategy<Value = Vec3> {
    [-2.0f64..2.0, -2.0f64..2.0, -2.0f64..2.0]
}

fn bad() -> impl Strategy<Value = f64> {
    prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)]
}

fn wild() -> impl Strategy<Value = f64> {
    prop_oneof![
        6 => -100.0f64..100.0,
        1 => bad(),
        1 => Just(f64::MAX),
        1 => Just(f64::MIN),
        1 => Just(1e300),
    ]
}

fn any_constraint() -> impl Strategy<Value = Constraint> {
    prop_oneof![
        Just(Constraint::Free),
        Just(Constraint::Axis(Axis::X)),
        Just(Constraint::Axis(Axis::Y)),
        Just(Constraint::Axis(Axis::Z)),
        Just(Constraint::Plane(Plane::XY)),
        Just(Constraint::Plane(Plane::YZ)),
        Just(Constraint::Plane(Plane::ZX)),
    ]
}

fn any_plane() -> impl Strategy<Value = Plane> {
    prop_oneof![Just(Plane::XY), Just(Plane::YZ), Just(Plane::ZX)]
}

fn any_axis() -> impl Strategy<Value = Axis> {
    prop_oneof![Just(Axis::X), Just(Axis::Y), Just(Axis::Z)]
}

/// 世界の X が画面の右、Y が上、Z が奥行きになる視点。
fn straight_view(center: Vec3, depth: f64) -> ScreenView {
    let frame = CameraFrame {
        eye: sub(center, [0.0, 0.0, depth]),
        right: [1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        forward: [0.0, 0.0, 1.0],
    };
    ScreenView::new(frame, 800.0, 600.0)
}

fn dist(a: ScreenPos, b: ScreenPos) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

proptest! {
    #[test]
    fn free_move_moves_the_center_on_screen_by_total(
        cam in camera(), (w, h) in size(), c in center(),
        total in [-300.0f64..300.0, -300.0f64..300.0],
    ) {
        let view = ScreenView::of(&cam, w, h);
        let d = move_delta(&view, Constraint::Free, c, total, [0.0, 0.0], None);
        let (Some(p0), Some(p1)) = (view.project(c), view.project(add(c, d))) else {
            return Err(TestCaseError::reject("カメラの後ろ"));
        };
        prop_assert!(close(p1.x - p0.x, total[0], 1e-9) && close(p1.y - p0.y, total[1], 1e-9), "{:?} vs {total:?}", [p1.x - p0.x, p1.y - p0.y]);
    }

    #[test]
    fn axis_move_is_parallel_to_the_axis_and_follows_the_screen_direction(
        cam in camera(), (w, h) in size(), c in center(), axis in any_axis(),
        lambda in -3.0f64..3.0, mu in -300.0f64..300.0,
    ) {
        let view = ScreenView::of(&cam, w, h);
        let (Some(p0), Some(p1)) = (view.project(c), view.project(add(c, axis.unit()))) else {
            return Err(TestCaseError::reject("カメラの後ろ"));
        };
        let sv = [p1.x - p0.x, p1.y - p0.y];
        prop_assume!(sv[0].hypot(sv[1]) > 1.0);
        let total = [lambda * sv[0] - mu * sv[1] / sv[0].hypot(sv[1]), lambda * sv[1] + mu * sv[0] / sv[0].hypot(sv[1])];
        let d = move_delta(&view, Constraint::Axis(axis), c, total, [0.0, 0.0], None);
        for other in Axis::ALL.into_iter().filter(|&a| a != axis) {
            prop_assert_eq!(dot(d, other.unit()), 0.0);
        }
        prop_assert!(close(dot(d, axis.unit()), lambda, 1e-9), "{d:?} vs {lambda}");
    }

    #[test]
    fn axis_move_ignores_screen_motion_perpendicular_to_the_axis(
        cam in camera(), (w, h) in size(), c in center(), axis in any_axis(), mu in -300.0f64..300.0,
    ) {
        let view = ScreenView::of(&cam, w, h);
        let (Some(p0), Some(p1)) = (view.project(c), view.project(add(c, axis.unit()))) else {
            return Err(TestCaseError::reject("カメラの後ろ"));
        };
        let sv = [p1.x - p0.x, p1.y - p0.y];
        prop_assume!(sv[0].hypot(sv[1]) > 1.0);
        let total = [-sv[1] * mu, sv[0] * mu];
        let d = move_delta(&view, Constraint::Axis(axis), c, total, [0.0, 0.0], None);
        prop_assert!(d.iter().all(|x| x.abs() < 1e-9 * (1.0 + mu.abs() * 100.0)), "{d:?}");
    }

    #[test]
    fn axis_collapsed_to_a_point_on_screen_gives_zero(
        c in center(), depth in 1.0f64..20.0, total in [-500.0f64..500.0, -500.0f64..500.0],
    ) {
        let view = straight_view(c, depth);
        prop_assert_eq!(move_delta(&view, Constraint::Axis(Axis::Z), c, total, [0.0, 0.0], None), [0.0; 3]);
    }

    #[test]
    fn plane_move_has_no_normal_component_and_zero_at_the_press_position(
        cam in camera(), (w, h) in size(), c in center(), plane in any_plane(),
        press in [0.0f64..400.0, 0.0f64..400.0], now in [0.0f64..400.0, 0.0f64..400.0],
    ) {
        let view = ScreenView::of(&cam, w, h);
        let Some(origin) = plane_point(&view, plane, c, press) else { return Ok(()) };
        let c_ = Constraint::Plane(plane);
        prop_assert_eq!(move_delta(&view, c_, c, [10.0, 10.0], press, Some(origin)), [0.0; 3]);
        let d = move_delta(&view, c_, c, [0.0, 0.0], now, Some(origin));
        prop_assert!(dot(d, plane.normal().unit()).abs() < 1e-6 * (1.0 + d.iter().map(|x| x.abs()).sum::<f64>()));
        if plane_point(&view, plane, c, now).is_some() {
            let s = view.project(add(origin, d));
            prop_assume!(s.is_some());
            let s = s.expect("直前で確認済み");
            prop_assert!(close(s.x, now[0], 1e-6) && close(s.y, now[1], 1e-6), "{s:?} vs {now:?}");
        }
    }

    #[test]
    fn plane_move_without_origin_is_zero(cam in camera(), (w, h) in size(), c in center(), plane in any_plane(), pos in [0.0f64..400.0, 0.0f64..400.0]) {
        let view = ScreenView::of(&cam, w, h);
        prop_assert_eq!(move_delta(&view, Constraint::Plane(plane), c, [5.0, 5.0], pos, None), [0.0; 3]);
    }

    #[test]
    fn move_delta_is_always_finite_even_for_garbage_input(
        cam in camera(), (w, h) in size(), constraint in any_constraint(),
        c in [wild(), wild(), wild()],
        total in [wild(), wild()],
        pos in [wild(), wild()],
        origin in proptest::option::of([wild(), wild(), wild()]),
    ) {
        let view = ScreenView::of(&cam, w, h);
        let d = move_delta(&view, constraint, c, total, pos, origin);
        prop_assert!(d.iter().all(|x| x.is_finite()), "{d:?}");
    }

    #[test]
    fn gizmo_tips_stay_within_a_sane_length(cam in camera(), (w, h) in size(), c in center()) {
        let view = ScreenView::of(&cam, w, h);
        let Some(g) = gizmo(&view, c) else { return Ok(()) };
        let limit = 3.0 * GIZMO_PX;
        for axis in Axis::ALL {
            let d = dist(g.origin, g.tip(axis));
            prop_assert!(d.is_finite() && d <= limit, "{d} > {limit}");
        }
        prop_assert_eq!(g.center, c);
    }

    #[test]
    fn gizmo_axes_facing_the_screen_are_exactly_84_points(c in center(), depth in 1.0f64..30.0) {
        let view = straight_view(c, depth);
        let g = gizmo(&view, c).expect("手前");
        prop_assert!(close(dist(g.origin, g.tip(Axis::X)), GIZMO_PX, 1e-9));
        prop_assert!(close(dist(g.origin, g.tip(Axis::Y)), GIZMO_PX, 1e-9));
        prop_assert!(dist(g.origin, g.tip(Axis::Z)) < 1e-9);
    }

    #[test]
    fn gizmo_is_none_for_a_center_that_is_too_close_or_not_finite(cam in camera(), (w, h) in size(), t in -50.0f64..(NEAR - 1e-6), b in bad(), axis in 0usize..3) {
        let view = ScreenView::of(&cam, w, h);
        let f = view.frame();
        let behind = add(f.eye, mul(f.forward, t));
        prop_assert!(gizmo(&view, behind).is_none());
        let mut c = [0.0; 3];
        c[axis] = b;
        prop_assert!(gizmo(&view, c).is_none());
    }

    #[test]
    fn pick_handle_matches_the_documented_priority(
        cam in camera(), (w, h) in size(), c in center(), dx in -140.0f64..140.0, dy in -140.0f64..140.0,
    ) {
        let view = ScreenView::of(&cam, w, h);
        let Some(g) = gizmo(&view, c) else { return Ok(()) };
        let pos = [g.origin[0] + dx, g.origin[1] + dy];
        let want = if let Some(i) = g.planes.iter().position(|q| point_in_quad(q, pos)) {
            Some(Handle::Plane(Plane::ALL[i]))
        } else {
            let mut best = None;
            let mut best_d = AXIS_PICK_RADIUS;
            for axis in Axis::ALL {
                let d = segment_distance(g.origin, g.tip(axis), pos);
                if d < best_d {
                    best_d = d;
                    best = Some(Handle::Axis(axis));
                }
            }
            best
        };
        prop_assert_eq!(pick_handle(&g, pos), want);
    }

    #[test]
    fn pick_handle_rejects_non_finite_positions(cam in camera(), (w, h) in size(), c in center(), b in bad(), first in any::<bool>()) {
        let view = ScreenView::of(&cam, w, h);
        let Some(g) = gizmo(&view, c) else { return Ok(()) };
        let pos = if first { [b, g.origin[1]] } else { [g.origin[0], b] };
        prop_assert_eq!(pick_handle(&g, pos), None);
    }

    #[test]
    fn plane_quads_are_picked_at_their_centers_in_a_straight_view(c in center(), depth in 2.0f64..20.0) {
        let view = straight_view(c, depth);
        let g = gizmo(&view, c).expect("手前");
        let mid = (PLANE_INNER + PLANE_OUTER) / 2.0 * GIZMO_PX;
        // 正面視点で見える平面は XY だけ（YZ・ZX は線に潰れる）。
        let pos = [g.origin[0] + mid, g.origin[1] - mid];
        prop_assert_eq!(pick_handle(&g, pos), Some(Handle::Plane(Plane::XY)));
    }

    #[test]
    fn ray_plane_hits_are_on_the_plane_and_on_the_ray(
        origin in [-5.0f64..5.0, -5.0f64..5.0, -5.0f64..5.0], dir in [-1.0f64..1.0, -1.0f64..1.0, -1.0f64..1.0],
        axis in any_axis(), point in [-5.0f64..5.0, -5.0f64..5.0, -5.0f64..5.0],
    ) {
        let ray = Ray { origin, dir };
        let n = axis.unit();
        let den = dot(n, dir);
        let t = dot(n, sub(point, origin)) / den;
        match ray_plane(&ray, n, point) {
            Some(p) => {
                prop_assert!(den.abs() >= 1e-6 && t > 0.0);
                prop_assert!(dot(n, sub(p, point)).abs() < 1e-9 * (1.0 + t.abs()));
                prop_assert!(p.iter().zip(ray.at(t)).all(|(a, b)| (a - b).abs() < 1e-9 * (1.0 + t.abs())));
            }
            None => prop_assert!(den.abs() < 1e-6 || t.is_nan() || t <= 0.0),
        }
    }

    #[test]
    fn ray_plane_is_none_for_parallel_and_backward_rays(
        origin in [-5.0f64..5.0, -5.0f64..5.0, -5.0f64..5.0], s in -5.0f64..5.0, tiny in -9e-7f64..9e-7, h in 0.5f64..5.0,
    ) {
        let n = [0.0, 1.0, 0.0];
        let parallel = Ray { origin, dir: [1.0, tiny, s] };
        prop_assert!(ray_plane(&parallel, n, [0.0, origin[1] + h, 0.0]).is_none());
        let away = Ray { origin, dir: [s, -1.0, 0.0] };
        prop_assert!(ray_plane(&away, n, [0.0, origin[1] + h, 0.0]).is_none());
        let on_plane = Ray { origin, dir: [s, 1.0, 0.0] };
        prop_assert!(ray_plane(&on_plane, n, [0.0, origin[1], 0.0]).is_none(), "t = 0 は交点なし");
    }

    #[test]
    fn ray_plane_rejects_non_finite_input(b in bad(), axis in 0usize..3) {
        let mut dir = [0.3, 0.4, 0.5];
        dir[axis] = b;
        let hit = ray_plane(&Ray { origin: [0.0; 3], dir }, [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]);
        prop_assert!(hit.is_none());
        let mut point = [0.0, 1.0, 0.0];
        point[axis] = b;
        let hit = ray_plane(&Ray { origin: [0.0; 3], dir: [0.0, 1.0, 0.0] }, [0.0, 1.0, 0.0], point);
        prop_assert!(hit.is_none_or(|p| p.iter().all(|c| c.is_finite())));
    }

    #[test]
    fn selection_center_is_the_mean_of_the_valid_picks(
        pts in proptest::collection::vec([-50.0f64..50.0, -50.0f64..50.0, -50.0f64..50.0], 0..8),
        idx in proptest::collection::vec(prop_oneof![0usize..10, 10usize..100, Just(usize::MAX)], 0..10),
    ) {
        let valid: Vec<Vec3> = idx.iter().filter_map(|&i| pts.get(i).copied()).collect();
        match selection_center(&pts, &idx) {
            None => prop_assert!(valid.is_empty()),
            Some(c) => {
                for k in 0..3 {
                    let mean = valid.iter().map(|p| p[k]).sum::<f64>() / valid.len() as f64;
                    prop_assert!((c[k] - mean).abs() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn selection_center_is_none_when_the_mean_is_not_finite(b in bad(), extra in 0usize..3) {
        let pts = vec![[b, 0.0, 0.0], [0.0; 3], [1.0; 3]];
        prop_assert_eq!(selection_center(&pts, &[0, 1, 2][..1 + extra]), None);
        prop_assert_eq!(selection_center(&[[f64::MAX; 3], [f64::MAX; 3]], &[0, 1]), None);
    }

    #[test]
    fn segment_distance_properties(
        a in [-100.0f64..100.0, -100.0f64..100.0], b in [-100.0f64..100.0, -100.0f64..100.0], p in [-100.0f64..100.0, -100.0f64..100.0], t in 0.0f64..=1.0,
    ) {
        let d = segment_distance(a, b, p);
        prop_assert!(d >= 0.0);
        prop_assert!(close(d, segment_distance(b, a, p), 1e-9));
        prop_assert!(d <= dist(a, p).min(dist(b, p)) + 1e-9);
        let on = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        prop_assert!(segment_distance(a, b, on) < 1e-6);
        prop_assert!(close(segment_distance(a, a, p), dist(a, p), 1e-9));
    }

    #[test]
    fn point_in_quad_for_axis_aligned_squares(x0 in -50.0f64..50.0, y0 in -50.0f64..50.0, side in 1.0f64..50.0, ccw in any::<bool>(), p in [-120.0f64..120.0, -120.0f64..120.0]) {
        let mut q = [[x0, y0], [x0 + side, y0], [x0 + side, y0 + side], [x0, y0 + side]];
        if !ccw {
            q.reverse();
        }
        let inside = p[0] > x0 + 1e-6 && p[0] < x0 + side - 1e-6 && p[1] > y0 + 1e-6 && p[1] < y0 + side - 1e-6;
        let outside = p[0] < x0 - 1e-6 || p[0] > x0 + side + 1e-6 || p[1] < y0 - 1e-6 || p[1] > y0 + side + 1e-6;
        if inside {
            prop_assert!(point_in_quad(&q, p));
        }
        if outside {
            prop_assert!(!point_in_quad(&q, p));
        }
    }

    #[test]
    fn point_in_quad_is_false_for_non_finite_positions(b in bad(), first in any::<bool>()) {
        let q = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let p = if first { [b, 5.0] } else { [5.0, b] };
        prop_assert!(!point_in_quad(&q, p));
    }
}

#[test]
fn plane_beats_axis_when_they_overlap() {
    let g = Gizmo {
        center: [0.0; 3],
        origin: [0.0, 0.0],
        tips: [[100.0, 0.0], [0.0, -100.0], [0.0, 0.0]],
        planes: [
            [[40.0, -5.0], [60.0, -5.0], [60.0, 5.0], [40.0, 5.0]],
            [
                [500.0, 500.0],
                [510.0, 500.0],
                [510.0, 510.0],
                [500.0, 510.0],
            ],
            [
                [600.0, 600.0],
                [610.0, 600.0],
                [610.0, 610.0],
                [600.0, 610.0],
            ],
        ],
    };
    assert_eq!(pick_handle(&g, [50.0, 0.0]), Some(Handle::Plane(Plane::XY)));
    assert_eq!(pick_handle(&g, [80.0, 0.0]), Some(Handle::Axis(Axis::X)));
    assert_eq!(pick_handle(&g, [80.0, 9.0]), None);
    assert_eq!(pick_handle(&g, [80.0, 8.9]), Some(Handle::Axis(Axis::X)));
}

#[test]
fn locks_and_handles_map_to_constraints() {
    assert_eq!(constraint_for_lock(None), Constraint::Free);
    for a in Axis::ALL {
        assert_eq!(constraint_for_lock(Some(a)), Constraint::Axis(a));
        assert_eq!(Constraint::from(Handle::Axis(a)), Constraint::Axis(a));
    }
    for p in Plane::ALL {
        assert_eq!(Constraint::from(Handle::Plane(p)), Constraint::Plane(p));
        let [a, b, n] = p.axes();
        assert_eq!(p.normal(), n);
        assert!(a != b && b != n && a != n);
    }
    assert_eq!(Plane::XY.normal(), Axis::Z);
    assert_eq!(Plane::YZ.normal(), Axis::X);
    assert_eq!(Plane::ZX.normal(), Axis::Y);
}
