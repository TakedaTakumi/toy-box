//! 表示の設定。ユーザー操作で変わる値は、ここで範囲に丸める。

use std::ops::RangeInclusive;

/// 分割数の範囲（JS 試作のスライダーと同じ）。
pub const SUBDIVISION_RANGE: RangeInclusive<usize> = 4..=16;
pub const DEFAULT_SUBDIVISIONS: usize = 8;

/// 分割数を範囲に丸める。
pub fn clamp_subdivisions(n: usize) -> usize {
    n.clamp(*SUBDIVISION_RANGE.start(), *SUBDIVISION_RANGE.end())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shading {
    /// 頂点の法線をピクセルごとに補間する。
    #[default]
    Smooth,
    /// 細かい三角形ごとに面の向きで明るさを決める。
    Flat,
}

impl Shading {
    pub const ALL: [Self; 2] = [Self::Smooth, Self::Flat];

    pub fn label(self) -> &'static str {
        match self {
            Self::Smooth => "スムース",
            Self::Flat => "フラット",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Smooth => Self::Flat,
            Self::Flat => Self::Smooth,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySettings {
    pub shading: Shading,
    subdivisions: usize,
    pub show_surface: bool,
    pub show_cage: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            shading: Shading::Smooth,
            subdivisions: DEFAULT_SUBDIVISIONS,
            show_surface: true,
            show_cage: true,
        }
    }
}

impl DisplaySettings {
    pub fn subdivisions(&self) -> usize {
        self.subdivisions
    }

    pub fn set_subdivisions(&mut self, n: usize) {
        self.subdivisions = clamp_subdivisions(n);
    }
}
