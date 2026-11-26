//! JS の Math と同じ結果を返す数値関数。

/// V8 の Math.hypot と同じ手順（最大値で正規化し、Kahan 加算）で計算する。
pub(crate) fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    let mut abs = [0.0_f64; 3];
    let mut one_is_nan = false;
    let mut max = 0.0_f64;
    for (slot, value) in abs.iter_mut().zip([x, y, z]) {
        if value.is_nan() {
            one_is_nan = true;
        } else {
            let a = value.abs();
            *slot = a;
            if a > max {
                max = a;
            }
        }
    }
    // 無限大は NaN より優先される（ECMAScript の仕様どおり）
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if one_is_nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for a in abs {
        let n = a / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// Math.max と同じく、NaN を伝播し、+0 と -0 では +0 を返す。
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else if b > a {
        b
    } else if a == 0.0 && (a.is_sign_positive() || b.is_sign_positive()) {
        0.0
    } else {
        a
    }
}

/// Math.min と同じく、NaN を伝播し、+0 と -0 では -0 を返す。
pub(crate) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else if b < a {
        b
    } else if a == 0.0 && (a.is_sign_negative() || b.is_sign_negative()) {
        -0.0
    } else {
        a
    }
}
