//! 共有辺をまたぐ位置（G0）と法線（G1）の連続性の検証。

use crate::bezier::bezier3d;
use crate::error::GregoryError;
use crate::js_math::{js_max, js_min};
use crate::patch::Patch;
use crate::topology::{Edge, FaceCorner};
use crate::vec3::{Vec3, cross, dot, len, mul, norm, sub};

/// 滑らかな共有辺の上で、両側のパッチの法線を比べた点。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub p: Vec3,
    pub na: Vec3,
    pub nb: Vec3,
}

/// `g0` は位置の差の最大値、`g1` は法線の角度差の最大値（度）。`pairs` は滑らかな共有辺、`creased` は折れ目の共有辺の数。
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuityCheck {
    pub g0: f64,
    pub g1: f64,
    pub pairs: usize,
    pub creased: usize,
    pub samples: Vec<Sample>,
}

struct EdgeSample {
    p: Vec3,
    n: Vec3,
}

fn sample_edge(patches: &[Patch], at: FaceCorner, t: f64) -> Result<EdgeSample, GregoryError> {
    let patch = &patches[at.fi];
    let h = 1e-6;
    let [p0, p1] = patch
        .edge_point(at.k, t, h)
        .ok_or(GregoryError::EdgeOutsidePatch { face: at.fi, edge: at.k })?;
    let tangent = bezier3d(&patch.ed()[at.k].e, t);
    Ok(EdgeSample { p: p0, n: norm(cross(tangent, mul(sub(p1, p0), 1.0 / h))) })
}

pub(crate) fn check_continuity(
    f: &[Vec<usize>],
    patches: &[Patch],
    edges: &[Edge],
) -> Result<ContinuityCheck, GregoryError> {
    let mut check = ContinuityCheck { g0: 0.0, g1: 0.0, pairs: 0, creased: 0, samples: Vec::new() };
    for e in edges {
        let &[fa, fb] = e.faces.as_slice() else {
            continue;
        };
        if e.crease {
            check.creased += 1;
        } else {
            check.pairs += 1;
        }
        for i in 1..=9_i32 {
            let s = f64::from(i) / 10.0;
            let param = |c: FaceCorner| if f[c.fi][c.k] == e.a { s } else { 1.0 - s };
            let a = sample_edge(patches, fa, param(fa))?;
            let b = sample_edge(patches, fb, param(fb))?;
            check.g0 = js_max(check.g0, len(sub(a.p, b.p)));
            if e.crease {
                continue;
            }
            let ang = js_max(-1.0, js_min(1.0, dot(a.n, b.n))).acos() * (180.0 / std::f64::consts::PI);
            check.g1 = js_max(check.g1, ang);
            if i == 3 || i == 5 || i == 7 {
                check.samples.push(Sample { p: a.p, na: a.n, nb: b.n });
            }
        }
    }
    Ok(check)
}
