//! プリセットのケージから曲面を組み立てる。

use gregory_core::{BuildOptions, CageModel, GregoryError, Mode, Model, Preset, build_model};

use crate::settings::clamp_subdivisions;

/// 曲面の組み立てに効く設定。これが変わったときだけ `build_model` をやり直す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuildKey {
    pub preset: Preset,
    pub subdivisions: usize,
}

/// 表示するケージと、そこから組み立てた曲面。
#[derive(Clone, Debug)]
pub struct Scene {
    pub cage: CageModel,
    pub model: Model,
}

/// ハンドル長（JS 試作の初期値）。この段階では変えられない。
const TAU: f64 = 1.0;

pub fn build_scene(key: &BuildKey) -> Result<Scene, GregoryError> {
    let cage = key.preset.make();
    let opts = BuildOptions {
        tau: TAU,
        mode: Mode::G1,
        n: clamp_subdivisions(key.subdivisions),
        creases: cage.creases.clone(),
    };
    let model = build_model(&cage.v, &cage.f, &opts)?;
    Ok(Scene { cage, model })
}
