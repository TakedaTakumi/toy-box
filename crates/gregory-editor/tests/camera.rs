use std::f64::consts::TAU;

use gregory_core::{Vec3, dot, len};
use gregory_editor::camera::{
    MAX_DISTANCE, MIN_DISTANCE, OrbitCamera, PITCH_LIMIT, view_projection,
};
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum Op {
    Orbit(f32, f32),
    Zoom(f64),
}

fn apply(c: &mut OrbitCamera, op: &Op) {
    match *op {
        Op::Orbit(dx, dy) => c.orbit(dx, dy),
        Op::Zoom(f) => c.zoom(f),
    }
}

fn ops() -> impl Strategy<Value = Vec<Op>> {
    let drag = prop_oneof![
        -2000.0f32..2000.0,
        Just(0.0),
        Just(93.75),
        Just(f32::MAX),
        Just(f32::MIN)
    ];
    let op = prop_oneof![
        (drag.clone(), drag).prop_map(|(x, y)| Op::Orbit(x, y)),
        prop_oneof![0.01f64..100.0, Just(1e-300), Just(1e300), Just(f64::MAX)].prop_map(Op::Zoom),
    ];
    proptest::collection::vec(op, 0..40)
}

fn run(ops: &[Op]) -> OrbitCamera {
    let mut c = OrbitCamera::default();
    ops.iter().for_each(|o| apply(&mut c, o));
    c
}

fn clip(m: &[[f32; 4]; 4], p: Vec3) -> [f64; 4] {
    std::array::from_fn(|row| {
        (0..3).map(|c| f64::from(m[c][row]) * p[c]).sum::<f64>() + f64::from(m[3][row])
    })
}

fn add_scaled(a: Vec3, b: Vec3, s: f64) -> Vec3 {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

proptest! {
    #[test]
    fn state_stays_in_range(ops in ops()) {
        let c = run(&ops);
        prop_assert!(c.pitch().is_finite() && (-PITCH_LIMIT..=PITCH_LIMIT).contains(&c.pitch()));
        prop_assert!(c.distance().is_finite() && (MIN_DISTANCE..=MAX_DISTANCE).contains(&c.distance()));
        prop_assert!(c.yaw().is_finite() && (0.0..TAU).contains(&c.yaw()), "yaw {}", c.yaw());
    }

    #[test]
    fn non_finite_orbit_ignored(
        ops in ops(),
        bad in prop_oneof![Just(f32::NAN), Just(f32::INFINITY), Just(f32::NEG_INFINITY)],
        other in -50.0f32..50.0,
        swap in any::<bool>(),
    ) {
        let mut c = run(&ops);
        let before = c;
        if swap { c.orbit(bad, other) } else { c.orbit(other, bad) }
        c.orbit(bad, bad);
        prop_assert_eq!(c, before);
    }

    #[test]
    fn invalid_zoom_ignored(
        ops in ops(),
        bad in prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY),
            Just(0.0), Just(-0.0), Just(-1.0), -1e300f64..=0.0],
    ) {
        let mut c = run(&ops);
        let before = c;
        c.zoom(bad);
        prop_assert_eq!(c, before);
    }

    #[test]
    fn reset_restores_default(ops in ops()) {
        let mut c = run(&ops);
        c.reset();
        prop_assert_eq!(c, OrbitCamera::default());
    }

    #[test]
    fn frame_is_orthonormal(ops in ops()) {
        let c = run(&ops);
        let f = c.frame();
        let eps = 1e-9;
        for v in [f.right, f.up, f.forward] {
            prop_assert!((len(v) - 1.0).abs() < eps);
        }
        prop_assert!(dot(f.right, f.up).abs() < eps);
        prop_assert!(dot(f.right, f.forward).abs() < eps);
        prop_assert!(dot(f.up, f.forward).abs() < eps);
        prop_assert!((len(f.eye) - c.distance()).abs() < eps);
        for k in 0..3 {
            prop_assert!((f.forward[k] + f.eye[k] / c.distance()).abs() < eps);
        }
        prop_assert!(f.up[1] >= 0.0);
    }

    #[test]
    fn projection_geometry(ops in ops(), w in 1.0f32..4000.0, h in 1.0f32..4000.0, s in 0.1f64..5.0, k in -2.0f64..=2.0) {
        let c = run(&ops);
        let fr = c.frame();
        let m = view_projection(&fr, w, h);
        let o = clip(&m, [0.0; 3]);
        prop_assert!(o[3] > 0.0);
        prop_assert!((o[0] / o[3]).abs() < 1e-4 && (o[1] / o[3]).abs() < 1e-4);
        let z = o[2] / o[3];
        prop_assert!(z > 0.0 && z < 1.0, "z {z}");
        let p = add_scaled(fr.eye, fr.forward, c.distance() + k);
        let q = clip(&m, p);
        let zq = q[2] / q[3];
        prop_assert!((0.0..=1.0).contains(&zq), "zq {zq}");
        let r = clip(&m, add_scaled([0.0; 3], fr.right, s));
        prop_assert!(r[0] / r[3] > 0.0);
        let u = clip(&m, add_scaled([0.0; 3], fr.up, s));
        prop_assert!(u[1] / u[3] > 0.0);
        prop_assert_eq!(c.view_projection(w, h), m);
    }

    #[test]
    fn projection_finite_for_any_size(
        ops in ops(),
        w in prop_oneof![Just(0.0f32), Just(-0.0), Just(-1.0), Just(f32::NAN), Just(f32::INFINITY),
            Just(f32::NEG_INFINITY), Just(f32::MIN_POSITIVE), Just(f32::MAX), Just(f32::MIN), 1.0f32..100.0],
        h in prop_oneof![Just(0.0f32), Just(-5.0), Just(f32::NAN), Just(f32::INFINITY),
            Just(f32::MAX), Just(1e-30), 1.0f32..100.0],
    ) {
        let m = run(&ops).view_projection(w, h);
        prop_assert!(m.iter().flatten().all(|x| x.is_finite()));
    }
}

#[test]
fn orbit_exact_wrap_keeps_yaw_below_tau() {
    // 初期 yaw 0.75 をちょうど 0 付近まで戻すドラッグ量
    let mut c = OrbitCamera::default();
    c.orbit(93.75, 0.0);
    assert!((0.0..TAU).contains(&c.yaw()), "yaw {}", c.yaw());
}
