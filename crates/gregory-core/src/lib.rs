//! グレゴリーパッチの数学と、ケージの編集操作。JS 試作（public/tools/gregory-mesh-editor/gregory.js・mesh.js）を、同じデータ構造と演算順序で移したもの。

mod bezier;
mod cage;
mod continuity;
mod curves;
mod edge_data;
mod error;
mod js_math;
mod mesh;
mod model;
mod normals;
mod patch;
mod tessellate;
mod topology;
mod union_find;
mod vec3;

pub use bezier::bezier3;
pub use continuity::{ContinuityCheck, Sample};
pub use error::GregoryError;
pub use mesh::{
    CageModel, Compacted, DEFAULT_MIRROR_EPS, Extruded, LoopCut, MeshError, ModelProblem, PRESETS,
    Preset, SelectMode, Selection, clone_model, compact, convert_selection, delete_faces, edge_map,
    ekey, extrude_faces, face_normal, loop_cut, mirror_map, selection_vertices, toggle_creases,
    validate_model,
};
pub use model::{BuildOptions, FaceCount, Mode, Model, build_model};
pub use patch::{EdgeData, Patch, PatchKind, QuadPatch, TriPatch};
pub use tessellate::TriMesh;
pub use topology::{Edge, EdgeEntry, EdgeKey, EdgeMap, FaceCorner};
pub use vec3::{Vec3, add, cross, dot, len, mul, norm, sub};
