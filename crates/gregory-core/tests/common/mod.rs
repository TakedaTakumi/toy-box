#![allow(dead_code)]

use serde::Deserialize;

/// JSON では非有限数が文字列で出るため、数値か文字列を受ける。
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Num {
    Val(f64),
    Text(String),
}

impl Num {
    pub fn get(&self) -> f64 {
        match self {
            Num::Val(x) => *x,
            Num::Text(s) => match s.as_str() {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                other => panic!("想定外の数値文字列: {other}"),
            },
        }
    }
}

pub type V3 = [Num; 3];

pub fn v3(a: &V3) -> [f64; 3] {
    [a[0].get(), a[1].get(), a[2].get()]
}

pub fn v3s(a: &[V3]) -> Vec<[f64; 3]> {
    a.iter().map(v3).collect()
}

pub fn v4(a: &[V3; 4]) -> [[f64; 3]; 4] {
    [v3(&a[0]), v3(&a[1]), v3(&a[2]), v3(&a[3])]
}

/// 両方 NaN、または等しい（無限大を含む）か、差が tol 以下なら一致。
pub fn close(a: f64, b: f64, tol: f64) -> bool {
    if a.is_nan() || b.is_nan() {
        return a.is_nan() && b.is_nan();
    }
    a == b || (a.is_finite() && b.is_finite() && (a - b).abs() <= tol)
}

/// JSON は -0 を 0 で出すため、0 の符号は区別しない。
pub fn same(a: f64, b: f64) -> bool {
    close(a, b, 0.0)
}

pub fn close3(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    (0..3).all(|k| close(a[k], b[k], tol))
}

pub fn same3(a: [f64; 3], b: [f64; 3]) -> bool {
    close3(a, b, 0.0)
}
