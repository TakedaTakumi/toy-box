//! ケージの編集操作。JS 試作（public/tools/gregory-mesh-editor/mesh.js）を、同じデータ構造と演算順序で移したもの。
//!
//! 編集操作は、JS と同じく入力のモデルを書き換えず、新しいモデルを返す。

mod compact;
mod crease;
mod delete;
mod edge_map;
mod error;
mod extrude;
mod face_normal;
mod loop_cut;
mod mirror;
mod model;
mod ordered;
mod presets;
mod selection;
mod validate;

pub use compact::{Compacted, compact};
pub use crease::toggle_creases;
pub use delete::delete_faces;
pub use edge_map::{edge_map, ekey};
pub use error::MeshError;
pub use extrude::{Extruded, extrude_faces};
pub use face_normal::face_normal;
pub use loop_cut::{LoopCut, loop_cut};
pub use mirror::{DEFAULT_MIRROR_EPS, mirror_map};
pub use model::{CageModel, clone_model};
pub use presets::{PRESETS, Preset};
pub use selection::{SelectMode, Selection, convert_selection, selection_vertices};
pub use validate::{ModelProblem, validate_model};
