use gregory_core::SelectMode;
use gregory_editor::readouts::{format_position, selection_summary};
use proptest::prelude::*;

fn mode() -> impl Strategy<Value = (SelectMode, &'static str)> {
    prop_oneof![
        Just((SelectMode::Vertex, "点")),
        Just((SelectMode::Edge, "辺")),
        Just((SelectMode::Face, "面")),
    ]
}

fn wild() -> impl Strategy<Value = f64> {
    prop_oneof![
        6 => -1000.0f64..1000.0,
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => Just(f64::MAX),
        1 => Just(f64::MIN_POSITIVE),
    ]
}

proptest! {
    #[test]
    fn summary_for_a_selection(
        (m, label) in mode(),
        count in prop_oneof![1usize..10_000, Just(usize::MAX)],
    ) {
        prop_assert_eq!(selection_summary(m, count), format!("{label} {count} 個を選択中"));
        prop_assert_eq!(selection_summary(m, 0), format!("{label}モード（未選択）"));
    }

    #[test]
    fn position_has_two_decimals_and_parses_back(p in [-1e6f64..1e6, -1e6f64..1e6, -1e6f64..1e6]) {
        let s = format_position(p);
        prop_assert!(s.starts_with('(') && s.ends_with(')'));
        let parts: Vec<&str> = s[1..s.len() - 1].split(", ").collect();
        prop_assert_eq!(parts.len(), 3);
        for (part, x) in parts.iter().zip(p) {
            let (_, frac) = part.split_once('.').expect("小数点がある");
            prop_assert_eq!(frac.len(), 2);
            let back: f64 = part.parse().expect("数値に戻せる");
            prop_assert!((back - x).abs() <= 0.005 + 1e-9, "{part} vs {x}");
        }
    }

    #[test]
    fn position_never_panics_for_non_finite_or_extreme_values(p in [wild(), wild(), wild()]) {
        let s = format_position(p);
        prop_assert!(s.starts_with('(') && s.ends_with(')') && s.matches(", ").count() == 2, "{s}");
    }
}

#[test]
fn position_examples() {
    assert_eq!(format_position([1.0, -2.5, 0.0]), "(1.00, -2.50, 0.00)");
    assert_eq!(format_position([0.126, 0.124, -1.0]), "(0.13, 0.12, -1.00)");
    assert_eq!(
        format_position([f64::NAN, f64::INFINITY, f64::NEG_INFINITY]),
        "(NaN, inf, -inf)"
    );
}

#[test]
fn summary_examples() {
    assert_eq!(
        selection_summary(SelectMode::Vertex, 0),
        "点モード（未選択）"
    );
    assert_eq!(selection_summary(SelectMode::Edge, 3), "辺 3 個を選択中");
    assert_eq!(selection_summary(SelectMode::Face, 1), "面 1 個を選択中");
}
