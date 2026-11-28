//! 編集中のケージから曲面を組み立てる。

use gregory_core::{BuildOptions, CageModel, GregoryError, Mode, Model, build_model};

use crate::settings::clamp_subdivisions;

/// 曲面の組み立てに効く入力。これが変わったときだけ `build_model` をやり直す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuildKey {
    /// ケージの版番号（編集のたびに増える）。
    pub cage_revision: u64,
    pub subdivisions: usize,
}

/// 組み立てに使ったケージと、そこから組み立てた曲面。
#[derive(Clone, Debug)]
pub struct Scene {
    pub cage: CageModel,
    pub model: Model,
}

/// ハンドル長（JS 試作の初期値）。この段階では変えられない。
const TAU: f64 = 1.0;

/// 分割数は範囲に丸める。
pub fn build_scene(cage: &CageModel, subdivisions: usize) -> Result<Scene, GregoryError> {
    let opts = BuildOptions {
        tau: TAU,
        mode: Mode::G1,
        n: clamp_subdivisions(subdivisions),
        creases: cage.creases.clone(),
    };
    let model = build_model(&cage.v, &cage.f, &opts)?;
    Ok(Scene {
        cage: cage.clone(),
        model,
    })
}
