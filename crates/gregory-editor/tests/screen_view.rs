use gregory_core::{Vec3, add, mul};
use gregory_editor::camera::{NEAR, OrbitCamera, PITCH_LIMIT, ScreenView};
use proptest::prelude::*;

fn camera() -> impl Strategy<Value = OrbitCamera> {
    (
        -500.0f32..500.0,
        -500.0f32..500.0,
        0.1f64..10.0,
        0.1f64..10.0,
    )
        .prop_map(|(dx, dy, z1, z2)| {
            let mut c = OrbitCamera::default();
            c.orbit(dx, dy);
            c.zoom(z1);
            c.zoom(z2);
            c
        })
}

fn view_size() -> impl Strategy<Value = (f32, f32)> {
    (50.0f32..2000.0, 50.0f32..2000.0)
}

fn bad_f64() -> impl Strategy<Value = f64> {
    prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)]
}

fn extreme_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        -10.0f64..10.0,
        bad_f64(),
        Just(f64::MAX),
        Just(f64::MIN),
        Just(1e300),
        Just(-1e300),
        Just(f64::MIN_POSITIVE),
    ]
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

proptest! {
    #[test]
    fn ray_through_projection_returns_the_point(
        cam in camera(),
        (w, h) in view_size(),
        p in [-2.0f64..2.0, -2.0f64..2.0, -2.0f64..2.0],
    ) {
        let view = ScreenView::of(&cam, w, h);
        let s = view.project(p).expect("半径2の球はカメラ距離3以上なら手前にある");
        let q = view.ray(s.x, s.y).expect("有限").at(s.depth);
        for i in 0..3 {
            prop_assert!(near(q[i], p[i], 1e-9), "{q:?} != {p:?}");
        }
    }

    #[test]
    fn projection_of_ray_point_returns_the_pixel(
        cam in camera(),
        (w, h) in view_size(),
        x in -3000.0f64..3000.0,
        y in -3000.0f64..3000.0,
        t in NEAR..60.0f64,
    ) {
        let view = ScreenView::of(&cam, w, h);
        let p = view.ray(x, y).expect("有限").at(t);
        let s = view.project(p).expect("t >= NEAR なら射影できる");
        prop_assert!(near(s.x, x, 1e-9) && near(s.y, y, 1e-9), "{s:?} vs ({x}, {y})");
        prop_assert!(near(s.depth, t, 1e-9), "depth {} vs t {}", s.depth, t);
        prop_assert!(s.depth >= NEAR - 1e-9);
    }

    #[test]
    fn matches_view_projection(
        cam in camera(),
        (w, h) in view_size(),
        p in [-2.0f64..2.0, -2.0f64..2.0, -2.0f64..2.0],
    ) {
        let view = ScreenView::of(&cam, w, h);
        let s = view.project(p).expect("手前にある");
        let m = cam.view_projection(w, h);
        let clip: Vec<f64> = (0..4)
            .map(|row| (0..3).map(|c| f64::from(m[c][row]) * p[c]).sum::<f64>() + f64::from(m[3][row]))
            .collect();
        prop_assert!(clip[3] > 0.0);
        let x = clip[0] / clip[3] * f64::from(w) / 2.0 + f64::from(w) / 2.0;
        let y = -clip[1] / clip[3] * f64::from(h) / 2.0 + f64::from(h) / 2.0;
        let tol = 0.05 + 1e-4 * (s.x.abs() + s.y.abs());
        prop_assert!((x - s.x).abs() < tol && (y - s.y).abs() < tol, "({x}, {y}) vs {s:?}");
    }

    #[test]
    fn invalid_size_is_treated_as_one(
        cam in camera(),
        w in prop_oneof![Just(0.0f32), Just(-0.0f32), -1e6f32..=0.0, Just(f32::NAN), Just(f32::INFINITY), Just(f32::NEG_INFINITY), Just(f32::MIN)],
        h in prop_oneof![Just(0.0f32), -1e6f32..=0.0, Just(f32::NAN), Just(f32::INFINITY), Just(f32::NEG_INFINITY)],
        p in [-2.0f64..2.0, -2.0f64..2.0, -2.0f64..2.0],
    ) {
        let bad = ScreenView::of(&cam, w, h);
        let one = ScreenView::of(&cam, 1.0, 1.0);
        prop_assert_eq!(bad.size(), [1.0, 1.0]);
        prop_assert_eq!(bad.focal(), one.focal());
        prop_assert_eq!(bad.project(p), one.project(p));
    }

    #[test]
    fn mixed_valid_and_invalid_size_keeps_the_valid_side(cam in camera(), w in 1.0f32..2000.0, bad in prop_oneof![Just(0.0f32), Just(f32::NAN), Just(-5.0f32)]) {
        let view = ScreenView::of(&cam, w, bad);
        prop_assert_eq!(view.size(), [f64::from(w), 1.0]);
        let view = ScreenView::of(&cam, bad, w);
        prop_assert_eq!(view.size(), [1.0, f64::from(w)]);
    }

    #[test]
    fn behind_the_near_plane_is_none(
        cam in camera(),
        (w, h) in view_size(),
        depth in -100.0f64..(NEAR - 1e-6),
        side in [-5.0f64..5.0, -5.0f64..5.0],
    ) {
        let view = ScreenView::of(&cam, w, h);
        let f = view.frame();
        let p = add(add(f.eye, mul(f.forward, depth)), add(mul(f.right, side[0]), mul(f.up, side[1])));
        prop_assert!(view.project(p).is_none(), "{:?}", view.project(p));
    }

    #[test]
    fn non_finite_point_is_never_projected(
        cam in camera(),
        (w, h) in view_size(),
        mut p in [-2.0f64..2.0, -2.0f64..2.0, -2.0f64..2.0],
        axis in 0usize..3,
        bad in bad_f64(),
    ) {
        p[axis] = bad;
        prop_assert!(ScreenView::of(&cam, w, h).project(p).is_none());
    }

    #[test]
    fn project_result_is_always_finite_and_in_front(
        cam in camera(),
        (w, h) in view_size(),
        p in [extreme_f64(), extreme_f64(), extreme_f64()],
    ) {
        if let Some(s) = ScreenView::of(&cam, w, h).project(p) {
            prop_assert!(s.x.is_finite() && s.y.is_finite() && s.depth.is_finite());
            prop_assert!(s.depth >= NEAR);
        }
    }

    #[test]
    fn ray_rejects_non_finite_input(
        cam in camera(),
        (w, h) in view_size(),
        good in -1000.0f64..1000.0,
        bad in bad_f64(),
        first in any::<bool>(),
    ) {
        let view = ScreenView::of(&cam, w, h);
        let (x, y) = if first { (bad, good) } else { (good, bad) };
        prop_assert!(view.ray(x, y).is_none());
        prop_assert!(view.ray(bad, bad).is_none());
    }

    #[test]
    fn ray_with_extreme_input_is_none_or_finite(
        cam in camera(),
        (w, h) in view_size(),
        x in extreme_f64(),
        y in extreme_f64(),
    ) {
        if let Some(r) = ScreenView::of(&cam, w, h).ray(x, y) {
            prop_assert!(r.dir.iter().chain(r.origin.iter()).all(|c| c.is_finite()));
        }
    }
}

#[test]
fn default_camera_projects_origin_to_the_view_center() {
    let view = ScreenView::of(&OrbitCamera::default(), 1000.0, 600.0);
    let s = view.project([0.0, 0.0, 0.0]).expect("原点は手前");
    assert!((s.x - 500.0).abs() < 1e-9 && (s.y - 300.0).abs() < 1e-9);
    assert!((s.depth - OrbitCamera::default().distance()).abs() < 1e-9);
}

#[test]
fn extreme_pitch_keeps_the_round_trip() {
    let mut cam = OrbitCamera::default();
    cam.orbit(0.0, 1e6);
    assert!((cam.pitch() - PITCH_LIMIT).abs() < 1e-12);
    let view = ScreenView::of(&cam, 800.0, 600.0);
    let p: Vec3 = [1.0, -1.0, 0.5];
    let s = view.project(p).expect("手前");
    let q = view.ray(s.x, s.y).expect("有限").at(s.depth);
    assert!(q.iter().zip(p).all(|(a, b)| (a - b).abs() < 1e-9));
}
