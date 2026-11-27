use gregory_editor::panels::{MAX_PANEL_WIDTH_FRACTION, PanelLayout, panel_width_range};
use proptest::prelude::*;

fn bad_f32() -> impl Strategy<Value = f32> {
    prop_oneof![
        Just(f32::NAN),
        Just(f32::INFINITY),
        Just(f32::NEG_INFINITY),
        Just(0.0f32),
        Just(-0.0f32),
        Just(-1.0f32),
        (-1.0e30f32..=0.0f32),
    ]
}

proptest! {
    #[test]
    fn invalid_width_is_none(w in bad_f32()) {
        prop_assert_eq!(PanelLayout::for_width(w), None);
    }

    #[test]
    fn narrow_width_closes_panels(w in 0.001f32..PanelLayout::NARROW_WIDTH) {
        prop_assert_eq!(
            PanelLayout::for_width(w),
            Some(PanelLayout { tools_open: false, properties_open: false })
        );
    }

    #[test]
    fn wide_width_opens_panels(w in prop_oneof![PanelLayout::NARROW_WIDTH..1.0e9f32, Just(f32::MAX)]) {
        prop_assert_eq!(
            PanelLayout::for_width(w),
            Some(PanelLayout { tools_open: true, properties_open: true })
        );
    }

    #[test]
    fn range_is_ordered_and_bounded(
        screen in 1.0f32..10000.0,
        min in 1.0f32..500.0,
        extra in 0.0f32..500.0,
    ) {
        let max = min + extra;
        let (lo, hi) = panel_width_range(screen, min, max);
        prop_assert!(lo <= hi);
        prop_assert!(hi <= max);
        prop_assert!(lo <= min);
        prop_assert!(2.0 * hi <= screen * 0.5 * (1.0 + 1e-6));
        prop_assert_eq!(hi, max.min(screen * MAX_PANEL_WIDTH_FRACTION));
    }

    #[test]
    fn invalid_screen_returns_original_range(
        screen in bad_f32(),
        min in 1.0f32..500.0,
        extra in 0.0f32..500.0,
    ) {
        let max = min + extra;
        prop_assert_eq!(panel_width_range(screen, min, max), (min.min(max), max));
    }

    #[test]
    fn extreme_inputs_never_panic_and_keep_order(
        screen in prop_oneof![bad_f32(), any::<f32>(), Just(f32::MAX), Just(f32::MIN_POSITIVE)],
        min in prop_oneof![1.0f32..500.0, Just(f32::MAX), Just(0.0f32)],
        max in prop_oneof![1.0f32..500.0, Just(f32::MAX), Just(0.0f32)],
    ) {
        let (lo, hi) = panel_width_range(screen, min, max);
        prop_assert!(lo <= hi || lo.is_nan() || hi.is_nan());
    }
}

#[test]
fn layout_boundaries() {
    let closed = Some(PanelLayout {
        tools_open: false,
        properties_open: false,
    });
    let open = Some(PanelLayout {
        tools_open: true,
        properties_open: true,
    });
    assert_eq!(PanelLayout::NARROW_WIDTH, 720.0);
    assert_eq!(PanelLayout::for_width(719.99), closed);
    assert_eq!(PanelLayout::for_width(720.0), open);
    assert_eq!(PanelLayout::for_width(1.0e30), open);
    assert_eq!(PanelLayout::for_width(f32::NAN), None);
    assert_eq!(PanelLayout::for_width(f32::INFINITY), None);
    assert_eq!(PanelLayout::for_width(f32::NEG_INFINITY), None);
    assert_eq!(PanelLayout::for_width(0.0), None);
    assert_eq!(PanelLayout::for_width(-5.0), None);
}

#[test]
fn width_range_examples() {
    assert_eq!(panel_width_range(360.0, 150.0, 260.0), (90.0, 90.0));
    assert_eq!(panel_width_range(1280.0, 150.0, 260.0), (150.0, 260.0));
    assert_eq!(panel_width_range(f32::NAN, 150.0, 260.0), (150.0, 260.0));
    assert_eq!(
        panel_width_range(f32::INFINITY, 150.0, 260.0),
        (150.0, 260.0)
    );
    assert_eq!(panel_width_range(0.0, 150.0, 260.0), (150.0, 260.0));
    assert_eq!(panel_width_range(-100.0, 150.0, 260.0), (150.0, 260.0));
    assert_eq!(panel_width_range(f32::NAN, 300.0, 260.0), (260.0, 260.0));
}
