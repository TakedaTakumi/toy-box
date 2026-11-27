use gregory_editor::settings::{
    DEFAULT_SUBDIVISIONS, DisplaySettings, SUBDIVISION_RANGE, Shading, clamp_subdivisions,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn clamp_always_in_range(n in any::<usize>()) {
        prop_assert!(SUBDIVISION_RANGE.contains(&clamp_subdivisions(n)));
    }

    #[test]
    fn clamp_keeps_in_range_values(n in 4usize..=16) {
        prop_assert_eq!(clamp_subdivisions(n), n);
    }

    #[test]
    fn set_subdivisions_matches_clamp(n in prop_oneof![any::<usize>(), 0usize..40, Just(usize::MAX), Just(0)]) {
        let mut s = DisplaySettings::default();
        s.set_subdivisions(n);
        prop_assert_eq!(s.subdivisions(), clamp_subdivisions(n));
        prop_assert_eq!(s.build_key().subdivisions, clamp_subdivisions(n));
    }
}

#[test]
fn clamp_extremes() {
    assert_eq!(clamp_subdivisions(0), 4);
    assert_eq!(clamp_subdivisions(3), 4);
    assert_eq!(clamp_subdivisions(17), 16);
    assert_eq!(clamp_subdivisions(usize::MAX), 16);
}

#[test]
fn default_is_valid() {
    assert_eq!(
        DisplaySettings::default().subdivisions(),
        DEFAULT_SUBDIVISIONS
    );
}

#[test]
fn shading_toggle_twice_is_identity_and_flips() {
    for s in Shading::ALL {
        assert_eq!(s.toggled().toggled(), s);
        assert_ne!(s.toggled(), s);
    }
}
