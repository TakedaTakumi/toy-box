//! 3次ベジェ曲線。

use crate::vec3::{Vec3, add, mul, sub};

pub(crate) fn bern3(u: f64) -> [f64; 4] {
    let s = 1.0 - u;
    [s * s * s, 3.0 * u * s * s, 3.0 * u * u * s, u * u * u]
}

pub fn bezier3(e: &[Vec3; 4], t: f64) -> Vec3 {
    let b = bern3(t);
    std::array::from_fn(|k| e[0][k] * b[0] + e[1][k] * b[1] + e[2][k] * b[2] + e[3][k] * b[3])
}

pub(crate) fn bezier3d(e: &[Vec3; 4], t: f64) -> Vec3 {
    let s = 1.0 - t;
    let a = mul(sub(e[1], e[0]), 3.0 * s * s);
    let b = mul(sub(e[2], e[1]), 6.0 * s * t);
    let c = mul(sub(e[3], e[2]), 3.0 * t * t);
    add(add(a, b), c)
}
