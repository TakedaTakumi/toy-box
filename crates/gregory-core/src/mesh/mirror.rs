//! x=0 の面についてのミラーの対応付け。

use std::collections::HashMap;

use crate::js_math::{js_round, js_sign};
use crate::vec3::Vec3;

/// JS の `mirrorMap` の `eps` の既定値。
pub const DEFAULT_MIRROR_EPS: f64 = 1e-4;

/// x=0 の面について対称な頂点の組を、`eps` 刻みに丸めた位置の一致で探す。相手がなければ `None`（JS の `-1`）。
///
/// 同じ位置に複数の頂点があれば、番号の最も大きい頂点を相手にする。
pub fn mirror_map(v: &[Vec3], eps: f64) -> Vec<Option<usize>> {
    // 0 をはさんで丸めが対称になるよう、絶対値で丸める
    let q = |x: f64| js_key(js_sign(x) * js_round(x.abs() / eps));
    let key = |x: f64, y: f64, z: f64| [q(x), q(y), q(z)];
    let mut at: HashMap<[u64; 3], usize> = HashMap::new();
    for (i, p) in v.iter().enumerate() {
        at.insert(key(p[0], p[1], p[2]), i);
    }
    v.iter()
        .map(|p| at.get(&key(-p[0], p[1], p[2])).copied())
        .collect()
}

/// JS で文字列にしたときに同じになる数（0 と -0、すべての NaN）を、同じ値にする。
fn js_key(x: f64) -> u64 {
    if x.is_nan() {
        f64::NAN.to_bits()
    } else if x == 0.0 {
        0
    } else {
        x.to_bits()
    }
}
