//! 最初に置くケージの形。

use super::model::CageModel;
use crate::vec3::Vec3;

/// JS の `PRESETS` のキー。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    Cube,
    Plane,
    Pyramid,
    Prism,
}

/// JS の `Object.keys(PRESETS)` と同じ順。
pub const PRESETS: [Preset; 4] = [Preset::Cube, Preset::Plane, Preset::Pyramid, Preset::Prism];

impl Preset {
    /// JS の `PRESETS` のキーの文字列。
    pub fn key(self) -> &'static str {
        match self {
            Self::Cube => "cube",
            Self::Plane => "plane",
            Self::Pyramid => "pyramid",
            Self::Prism => "prism",
        }
    }

    /// 未知のキーは `None`（JS では `PRESETS[key]` が `undefined`）。
    pub fn from_key(key: &str) -> Option<Self> {
        PRESETS.into_iter().find(|p| p.key() == key)
    }

    /// 画面に出す名前（JS の `name`）。
    pub fn name(self) -> &'static str {
        match self {
            Self::Cube => "立方体",
            Self::Plane => "平面",
            Self::Pyramid => "四角錐",
            Self::Prism => "三角柱",
        }
    }

    pub fn make(self) -> CageModel {
        match self {
            Self::Cube => make(
                vec![
                    [-1.0, -1.0, -1.0],
                    [1.0, -1.0, -1.0],
                    [1.0, 1.0, -1.0],
                    [-1.0, 1.0, -1.0],
                    [-1.0, -1.0, 1.0],
                    [1.0, -1.0, 1.0],
                    [1.0, 1.0, 1.0],
                    [-1.0, 1.0, 1.0],
                ],
                vec![
                    vec![4, 5, 6, 7],
                    vec![3, 2, 1, 0],
                    vec![0, 1, 5, 4],
                    vec![2, 3, 7, 6],
                    vec![1, 2, 6, 5],
                    vec![3, 0, 4, 7],
                ],
            ),
            Self::Plane => {
                let mut v = Vec::new();
                for j in 0..3 {
                    for i in 0..3 {
                        v.push([f64::from(i) - 1.0, f64::from(j) - 1.0, 0.0]);
                    }
                }
                let mut f = Vec::new();
                for j in 0..2 {
                    for i in 0..2 {
                        f.push(vec![
                            j * 3 + i,
                            j * 3 + i + 1,
                            (j + 1) * 3 + i + 1,
                            (j + 1) * 3 + i,
                        ]);
                    }
                }
                make(v, f)
            }
            Self::Pyramid => make(
                vec![
                    [-1.0, -1.0, -1.0],
                    [1.0, -1.0, -1.0],
                    [1.0, 1.0, -1.0],
                    [-1.0, 1.0, -1.0],
                    [0.0, 0.0, 1.2],
                ],
                vec![
                    vec![3, 2, 1, 0],
                    vec![0, 1, 4],
                    vec![1, 2, 4],
                    vec![2, 3, 4],
                    vec![3, 0, 4],
                ],
            ),
            Self::Prism => make(
                vec![
                    [-1.1, -0.7, -1.0],
                    [1.1, -0.7, -1.0],
                    [0.0, 1.0, -1.0],
                    [-1.1, -0.7, 1.0],
                    [1.1, -0.7, 1.0],
                    [0.0, 1.0, 1.0],
                ],
                vec![
                    vec![3, 4, 5],
                    vec![2, 1, 0],
                    vec![0, 1, 4, 3],
                    vec![1, 2, 5, 4],
                    vec![2, 0, 3, 5],
                ],
            ),
        }
    }
}

/// 高さを z で書いた座標を、Y-up（x, z, -y）に直す。回転なので、右手系も面の向きも変わらない。
fn make(v: Vec<Vec3>, f: Vec<Vec<usize>>) -> CageModel {
    CageModel {
        v: v.into_iter().map(|[x, y, z]| [x, z, 0.0 - y]).collect(),
        f,
        creases: Default::default(),
    }
}
