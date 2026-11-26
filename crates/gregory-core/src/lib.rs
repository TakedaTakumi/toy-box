//! グレゴリーパッチの数学。JS 試作（public/tools/gregory-mesh-editor/gregory.js）を、同じデータ構造と演算順序で移したもの。

mod bezier;
mod cage;
mod continuity;
mod curves;
mod edge_data;
mod error;
mod js_math;
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
pub use model::{BuildOptions, FaceCount, Mode, Model, build_model};
pub use patch::{EdgeData, Patch, PatchKind, QuadPatch, TriPatch};
pub use tessellate::TriMesh;
pub use topology::{Edge, EdgeKey, FaceCorner};
pub use vec3::{Vec3, add, cross, dot, len, mul, norm, sub};
