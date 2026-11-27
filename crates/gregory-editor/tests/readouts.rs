use gregory_core::{ContinuityCheck, Preset};
use gregory_editor::readouts::{
    FrameTimer, G1Verdict, SceneStats, backend_label, format_g0, format_g1, g1_verdict,
};
use gregory_editor::scene::{BuildKey, build_scene};
use proptest::prelude::*;

fn check(g1: f64, pairs: usize) -> ContinuityCheck {
    ContinuityCheck {
        g0: 0.0,
        g1,
        pairs,
        creased: 0,
        samples: vec![],
    }
}

proptest! {
    #[test]
    fn verdict_by_threshold(g1 in prop_oneof![-10.0f64..10.0, Just(f64::NAN), Just(f64::INFINITY), Just(f64::NEG_INFINITY)], pairs in 1usize..100) {
        let want = if g1 < 0.5 { G1Verdict::Satisfied } else { G1Verdict::Violated };
        prop_assert_eq!(g1_verdict(&check(g1, pairs)), want);
    }

    #[test]
    fn verdict_no_pairs(g1 in any::<f64>()) {
        prop_assert_eq!(g1_verdict(&check(g1, 0)), G1Verdict::NoSharedEdges);
        prop_assert_eq!(format_g1(&check(g1, 0)), "対象の辺なし");
    }

    #[test]
    fn g0_below_floor(g0 in prop_oneof![-1e300f64..1e-9, Just(f64::NEG_INFINITY), Just(-0.0)]) {
        prop_assume!(g0 < 1e-9);
        prop_assert_eq!(format_g0(g0), "< 1e-9");
    }

    #[test]
    fn g0_scientific(g0 in 1e-9f64..1e12) {
        prop_assert_eq!(format_g0(g0), format!("{g0:.1e}"));
    }

    #[test]
    fn g1_three_decimals(g1 in -1000.0f64..1000.0, pairs in 1usize..9) {
        let s = format_g1(&check(g1, pairs));
        prop_assert_eq!(s.clone(), format!("{g1:.3}°"));
        prop_assert_eq!(s.trim_end_matches('°').split('.').nth(1).unwrap().len(), 3);
    }

    #[test]
    fn frame_timer_is_bounded_ema(xs in proptest::collection::vec(0.0f32..1.0, 1..40)) {
        let mut t = FrameTimer::default();
        let mut expected: Option<f32> = None;
        for &x in &xs {
            t.record(Some(x));
            let ms = x * 1000.0;
            expected = Some(expected.map_or(ms, |a| a + (ms - a) * 0.1));
            prop_assert_eq!(t.average_ms(), expected);
        }
        let ms: Vec<f32> = xs.iter().map(|x| x * 1000.0).collect();
        let (lo, hi) = (ms.iter().cloned().fold(f32::MAX, f32::min), ms.iter().cloned().fold(0.0, f32::max));
        let avg = t.average_ms().unwrap();
        prop_assert!(avg >= lo - 1e-3 && avg <= hi + 1e-3);
    }

    #[test]
    fn frame_timer_ignores_invalid(
        first in 0.0f32..1.0,
        bad in prop_oneof![Just(None), Just(Some(f32::NAN)), Just(Some(f32::INFINITY)),
            Just(Some(f32::NEG_INFINITY)), Just(Some(-0.001f32)), Just(Some(f32::MIN)),
            Just(Some(f32::MAX))],
    ) {
        let mut t = FrameTimer::default();
        t.record(bad);
        prop_assert_eq!(t.average_ms(), None);
        t.record(Some(first));
        let before = t;
        t.record(bad);
        prop_assert_eq!(t, before);
    }
}

#[test]
fn g0_edge_cases() {
    assert_eq!(format_g0(f64::NAN), "NaN");
    assert_eq!(format_g0(0.0), "< 1e-9");
    assert_eq!(format_g0(-1.0), "< 1e-9");
    assert_eq!(format_g0(1e-9), "1.0e-9");
    assert_eq!(format_g0(f64::INFINITY), format!("{:.1e}", f64::INFINITY));
}

#[test]
fn nan_g1_is_violated_when_pairs_exist() {
    assert_eq!(g1_verdict(&check(f64::NAN, 3)), G1Verdict::Violated);
    assert_eq!(g1_verdict(&check(0.5, 3)), G1Verdict::Violated);
    assert_eq!(g1_verdict(&check(0.4999, 3)), G1Verdict::Satisfied);
}

#[test]
fn first_record_is_raw_then_ema() {
    let mut t = FrameTimer::default();
    t.record(Some(0.010));
    assert_eq!(t.average_ms(), Some(10.0));
    t.record(Some(0.020));
    assert!((t.average_ms().unwrap() - 11.0).abs() < 1e-4);
}

#[test]
fn backend_labels() {
    use wgpu::Backend::*;
    assert_eq!(backend_label(BrowserWebGpu, true), "WebGPU");
    assert_eq!(backend_label(BrowserWebGpu, false), "WebGPU");
    assert_eq!(backend_label(Gl, true), "WebGL2");
    assert_eq!(backend_label(Gl, false), "OpenGL");
    assert_eq!(backend_label(Vulkan, false), "Vulkan");
    assert_eq!(backend_label(Metal, false), "Metal");
    assert_eq!(backend_label(Dx12, false), "DirectX 12");
    assert_eq!(backend_label(Noop, false), "なし");
}

#[test]
fn cube_stats() {
    let scene = build_scene(&BuildKey {
        preset: Preset::Cube,
        subdivisions: 8,
    })
    .unwrap();
    let s = SceneStats::of(&scene);
    assert_eq!((s.vertices, s.faces, s.edges), (8, 6, 12));
    assert_eq!((s.patches, s.quads, s.tris), (6, 6, 0));
    assert_eq!(s.triangles, scene.model.mesh.tris.len());
}
