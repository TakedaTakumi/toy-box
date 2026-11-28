use gregory_core::PRESETS;
use gregory_editor::scene::build_scene;
use proptest::prelude::*;

#[test]
fn all_presets_and_subdivisions_build() {
    for preset in PRESETS {
        for subdivisions in 4..=16 {
            let scene = build_scene(&preset.make(), subdivisions)
                .unwrap_or_else(|e| panic!("{preset:?} n={subdivisions}: {e:?}"));
            assert_eq!(scene.cage, preset.make());
            assert!(!scene.model.mesh.tris.is_empty());
        }
    }
}

#[test]
fn out_of_range_subdivisions_are_clamped() {
    for preset in PRESETS {
        let at = |subdivisions| build_scene(&preset.make(), subdivisions).unwrap();
        for (bad, clamped) in [
            (0, 4),
            (1, 4),
            (3, 4),
            (17, 16),
            (100, 16),
            (usize::MAX, 16),
        ] {
            assert_eq!(at(bad).model, at(clamped).model, "{preset:?} {bad}");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    #[test]
    fn any_subdivisions_never_fails(idx in 0usize..4, n in prop_oneof![any::<usize>(), 0usize..40]) {
        let scene = build_scene(&PRESETS[idx].make(), n);
        prop_assert!(scene.is_ok());
        prop_assert!(!scene.unwrap().model.mesh.tris.is_empty());
    }
}
