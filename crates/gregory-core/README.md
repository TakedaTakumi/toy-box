# gregory-core

グレゴリーメッシュエディタの、グレゴリーパッチの数学とケージの編集操作を Rust に移したライブラリ。JS 試作の `public/tools/gregory-mesh-editor/gregory.js` と `mesh.js` を、同じデータ構造と同じ浮動小数点の演算順序で移している（忠実な移植）。同じ入力に対して、JS 版と同じ値（許容誤差内）を返すことを目標にする。

- 依存クレート：なし（テストだけ `proptest`・`serde`・`serde_json` を使う）
- `unsafe` は使わない（ワークスペースの lint で禁止）
- 入力がどのような値でも panic しない。JS 版が例外を投げる入力では `Err` を返す

## モジュール構成

| モジュール | 内容 | JS の対応箇所 |
| --- | --- | --- |
| `vec3` | ベクトル演算 | `add`・`sub`・`mul`・`dot`・`cross`・`len`・`norm` |
| `js_math` | JS の `Math.hypot`・`Math.max`・`Math.min` と同じ結果を返す関数 | （JS の組み込み） |
| `bezier` | 3次ベジェ曲線とその微分 | `bern3`・`bezier3`・`bezier3d` |
| `cage` | 頂点と面の参照、面の角の幾何 | `cornerWeighted`・`flat` |
| `topology` | 辺の名前・面の角・辺の表（`EdgeMap` は `mesh` と共有し、公開する） | `ekey`・`edgeMap` |
| `normals`・`union_find` | 頂点の法線と、折れ目で区切った面の群ごとの角の法線 | `nrmV`・`corner` の計算 |
| `curves` | 辺の3次ベジェ曲線と両端の接線 | `tangent`・`curve` |
| `edge_data` | 辺をまたぐ接線場と、内部制御点の元になる係数 | `edgeData` |
| `patch` | 四角形・三角形のグレゴリーパッチ | `quadPatch`・`triPatch`・`blend`・`normalAt` |
| `continuity` | 共有辺の G0・G1 の検証 | `sampleEdge` と検証のループ |
| `tessellate` | 描画用の三角形分割 | `tessellate` |
| `model` | 全体の組み立て | `buildModel` |
| `mesh` | ケージの編集操作（下の表） | `mesh.js` |

### `mesh` の構成

| モジュール | 内容 | JS の対応箇所 |
| --- | --- | --- |
| `mesh::model` | 編集対象のケージ `CageModel` | `{ V, F, creases }`・`cloneModel` |
| `mesh::edge_map` | 辺の名前と辺の表（`topology` の `EdgeMap` を使う） | `ekey`・`edgeMap` |
| `mesh::face_normal` | 面の法線 | `faceNormal` |
| `mesh::compact` | 使われていない頂点の除去と番号の詰め直し | `compact` |
| `mesh::delete` | 面の削除 | `deleteFaces` |
| `mesh::extrude` | 面の押し出し | `extrudeFaces` |
| `mesh::loop_cut` | ループ分割 | `loopCut` |
| `mesh::crease` | 折れ目の切り替え | `toggleCreases` |
| `mesh::selection` | 選択の頂点と、モードの変換 | `selectionVertices`・`convertSelection` |
| `mesh::mirror` | ミラーの対応付け | `mirrorMap` |
| `mesh::validate` | 位相と座標の検査 | `validateModel` |
| `mesh::presets` | 最初に置く形 | `PRESETS` |
| `mesh::ordered` | 挿入順を保つ集合と表（JS の `Set`・`Map` の反復順の再現） | （JS の組み込み） |
| `mesh::error` | 編集操作の失敗 | （例外） |

## 公開 API の対応表

JS の公開関数（`gregory.js` の `export`）と 1 対 1 に対応する。`Vec3` は `[f64; 3]`。

| JS | Rust | 入力 | 出力 |
| --- | --- | --- | --- |
| `add(a, b)` | `add(a, b)` | `Vec3, Vec3` | `Vec3` |
| `sub(a, b)` | `sub(a, b)` | `Vec3, Vec3` | `Vec3` |
| `mul(a, s)` | `mul(a, s)` | `Vec3, f64` | `Vec3` |
| `dot(a, b)` | `dot(a, b)` | `Vec3, Vec3` | `f64` |
| `cross(a, b)` | `cross(a, b)` | `Vec3, Vec3` | `Vec3` |
| `len(a)` | `len(a)` | `Vec3` | `f64` |
| `norm(a)` | `norm(a)` | `Vec3` | `Vec3` |
| `bezier3(E, t)` | `bezier3(&e, t)` | `&[Vec3; 4], f64` | `Vec3` |
| `buildModel(V, F, opts)` | `build_model(&v, &f, &opts)` | `&[Vec3], &[Vec<usize>], &BuildOptions` | `Result<Model, GregoryError>` |

### データ型の対応

| JS | Rust |
| --- | --- |
| 頂点 `V`（`[x, y, z]` の配列） | `&[Vec3]` |
| 面 `F`（頂点番号の配列の配列） | `&[Vec<usize>]` |
| `opts.tau` | `BuildOptions::tau: f64` |
| `opts.mode`（`'g1'` かそれ以外） | `BuildOptions::mode: Mode`（`Mode::G1`・`Mode::Linear`）。文字列からは `Mode::from_js_name` |
| `opts.N` | `BuildOptions::n: usize` |
| `opts.creases`（`"a-b"` の `Set`。省略時は空） | `BuildOptions::creases: HashSet<EdgeKey>`。`EdgeKey::new(a, b)` は番号を小さい順に並べ替える。`Display` は JS の `ekey` と同じ `"a-b"` |
| 戻り値 `{ nrm, patches, edges, mesh, check, count }` | `Model { nrm, patches, edges, mesh, check, count }` |
| `edges[i]`：`{ a, b, faces: [{ fi, k }], E, crease }` | `Edge { a, b, faces: Vec<FaceCorner>, e: [Vec3; 4], crease }` |
| `mesh`：`{ pos, nrm, tris, triFace }` | `TriMesh { pos, nrm, tris: Vec<[usize; 3]>, tri_face }` |
| `check`：`{ g0, g1, pairs, creased, samples: [{ p, na, nb }] }` | `ContinuityCheck { g0, g1, pairs, creased, samples: Vec<Sample> }` |
| `count`：`{ q, t }` | `FaceCount { q, t }` |
| `patches[i].type`（`'q'`・`'t'`） | `Patch::kind()`（`PatchKind::Quad`・`PatchKind::Tri`） |
| `patches[i].ed`：`[{ E, c }]` | `Patch::ed()`：`&[EdgeData]`（`EdgeData { e, c }`） |
| `patches[i].q`：`[null, q1, q2]` の配列 | `Patch::q()`：`&[[Vec3; 2]]`。JS の `null` を除くため、番号が1つずれる（`q()[k][0]` が JS の `q[k][1]`） |
| `patches[i].at(a, b)` | `Patch::at(a, b)` |
| `patches[i].normal(a, b)` | `Patch::normal(a, b)` |
| `patches[i].edgePoint(k, t, h)`：`[p0, p1]` | `Patch::edge_point(k, t, h)`：`Option<[Vec3; 2]>`（パッチにない辺の番号では `None`。JS では例外） |

### mesh.js の公開 API の対応表

`mesh.js` の `export` と 1 対 1 に対応する。編集操作は JS と同じく入力のモデルを書き換えず、新しいモデルを返す（Rust では `&CageModel` を受け、新しい値を返す）。

| JS | Rust | 入力 | 出力 | 入力を書き換えるか |
| --- | --- | --- | --- | --- |
| `ekey(a, b)` | `ekey(a, b)` | `usize, usize` | `EdgeKey` | - |
| `cloneModel(m)` | `clone_model(&m)`（`CageModel::clone` と同じ） | `&CageModel` | `CageModel` | しない |
| `faceNormal(V, f)` | `face_normal(&v, &f)` | `&[Vec3], &[usize]` | `Result<Vec3, MeshError>` | - |
| `edgeMap(F)` | `edge_map(&f)` | `&[Vec<usize>]` | `EdgeMap` | - |
| `compact(model)` | `compact(&model)` | `&CageModel` | `Result<Compacted, MeshError>` | しない |
| `deleteFaces(model, faceIdxs)` | `delete_faces(&model, &face_idxs)` | `&CageModel, &[usize]` | `Result<CageModel, MeshError>` | しない |
| `extrudeFaces(model, faceIdxs, dist)` | `extrude_faces(&model, &face_idxs, dist)` | `&CageModel, &[usize], f64` | `Result<Extruded, MeshError>` | しない |
| `loopCut(model, edgeKeys, midOf)` | `loop_cut(&model, &edge_keys, mid_of)` | `&CageModel, &[EdgeKey], Option<&HashMap<EdgeKey, Vec3>>` | `Result<LoopCut, MeshError>` | しない |
| `toggleCreases(model, edgeKeys)` | `toggle_creases(&model, &edge_keys)` | `&CageModel, &[EdgeKey]` | `CageModel` | しない |
| `selectionVertices(model, mode, items)` | `selection_vertices(&model, &items)` | `&CageModel, &Selection` | `Vec<usize>` | - |
| `convertSelection(model, from, items, to)` | `convert_selection(&model, &items, to)` | `&CageModel, &Selection, SelectMode` | `Selection` | - |
| `mirrorMap(V, eps = 1e-4)` | `mirror_map(&v, eps)`（既定値は `DEFAULT_MIRROR_EPS`） | `&[Vec3], f64` | `Vec<Option<usize>>` | - |
| `validateModel(model)` | `validate_model(&model)` | `&CageModel` | `Vec<ModelProblem>` | - |
| `PRESETS` | `PRESETS: [Preset; 4]` | - | - | - |
| `PRESETS[key].name` | `Preset::name()` | - | `&'static str` | - |
| `PRESETS[key].make()` | `Preset::make()` | - | `CageModel` | - |
| `PRESETS[key]`（キーから引く） | `Preset::from_key(key)` | `&str` | `Option<Preset>` | - |

### mesh.js のデータ型の対応

| JS | Rust |
| --- | --- |
| モデル `{ V, F, creases }` | `CageModel { v: Vec<Vec3>, f: Vec<Vec<usize>>, creases: HashSet<EdgeKey> }`。`creases` は `BuildOptions::creases` と同じ型で、そのまま `build_model` に渡せる。JS の `Set` と違い反復順を持たない |
| 辺の名前 `"a-b"` | `EdgeKey`（段階1と同じ。常に正規形） |
| `edgeMap` の値 `{ a, b, faces: [fi] }` | `EdgeEntry { a, b, faces: Vec<FaceCorner> }`。JS の `fi` は `FaceCorner::fi`。`EdgeMap::entries()` が JS の `Map` の反復順 |
| `compact` の戻り値 `{ model, remap }`（`remap` は `Map`） | `Compacted { model, remap: Vec<Option<usize>> }`。`remap[i]` が元の頂点 `i` の新しい番号。JS の `Map` に無い番号は `None` |
| `extrudeFaces` の戻り値 `{ model, faces }` | `Extruded { model, faces: Vec<usize> }` |
| `loopCut` の戻り値 `{ model, edges }` | `LoopCut { model, edges: Vec<EdgeKey> }` |
| `loopCut` の `midOf`（`Map<"a-b", [x, y, z]>`、省略可） | `Option<&HashMap<EdgeKey, Vec3>>` |
| 選択のモード `'v'`・`'e'`・`'f'` | `SelectMode::{Vertex, Edge, Face}`。文字列からは `SelectMode::from_js_name` |
| 選択の `mode` と `items` の組 | `Selection::{Vertices(Vec<usize>), Edges(Vec<EdgeKey>), Faces(Vec<usize>)}`（モードと要素の型を1つにした） |
| `selectionVertices` の戻り値（`Set<number>`） | `Vec<usize>`（重複なし、JS の `Set` の反復順） |
| `convertSelection` の戻り値（`Set`） | `Selection`（`to` のモード。要素は JS の `Set` の反復順） |
| `mirrorMap` の戻り値（相手がなければ `-1`） | `Vec<Option<usize>>`（`-1` は `None`） |
| `validateModel` の戻り値（問題の文字列の配列） | `Vec<ModelProblem>`。`Display` が JS の文字列と同じ |
| 例外・`undefined` の混ざった結果 | `Err(MeshError)` |

## 不正な入力

JS 版の振る舞いに合わせている。型で表せない入力（負の番号など）は、Rust では渡せない。

### gregory.js（`build_model` など）

| 入力 | JS 版 | Rust 版 |
| --- | --- | --- |
| 頂点の座標に NaN・無限大 | 例外なし。NaN・無限大が結果に伝わる | 同じ（`Ok`。値が伝わる） |
| `len` に無限大と NaN が混ざる | `Math.hypot` の仕様で無限大 | 同じ（V8 の `Math.hypot` と同じ手順） |
| `norm` に長さ `1e-12` 未満のベクトル | `[0, 0, 0]` | 同じ |
| `bezier3` の `t` が 0〜1 の範囲外・NaN | 例外なし。外挿・NaN | 同じ |
| 面が存在しない頂点の番号を参照（範囲外） | `TypeError` | `Err(GregoryError::VertexOutOfRange)` |
| 面の頂点番号が負・整数でない | `TypeError` | 型（`usize`）で排除 |
| 頂点が3つ未満の面（空の面を含む） | `TypeError` | `Err(GregoryError::TooFewCorners)` |
| 5辺以上の面 | 先頭の3辺で三角形パッチを作る。4辺目以降がちょうど2面で共有されていると `TypeError`、そうでなければ例外なし | 同じ（共有されていると `Err(GregoryError::EdgeOutsidePatch)`）。`count` には数えない |
| 面の中で頂点番号が重複（`[0, 1, 2, 2]` など） | 例外なし。角の法線は面の中で最初に現れる角のものを使う。両端が同じ辺の曲線は、2回目以降は逆向きのものが返る | 同じ |
| 3面以上で共有される辺 | 例外なし。折れ目の指定は無視され、G0・G1 の検証から外れる | 同じ |
| 面が空（`F = []`） | 例外なし。頂点の法線は `[0, 0, 0]`、パッチ・辺・メッシュは空 | 同じ |
| 頂点も面も空 | 例外なし。すべて空 | 同じ |
| どの面にも使われない頂点 | 例外なし。その頂点の法線は `[0, 0, 0]` | 同じ |
| `tau` が NaN・無限大・負 | 例外なし。値が伝わる | 同じ |
| `mode` が `'g1'` 以外の文字列 | 線形補間の接線場 | `Mode::from_js_name` が `Mode::Linear` を返す |
| `N` が 0 | 例外なし。各パッチに `at(NaN, NaN)` の点が1つ、三角形なし | 同じ（`0/0` の NaN で評価する） |
| `N` が負・NaN | 例外なし。点も三角形もない | 型（`usize`）で排除 |
| `N` が整数でない | 例外なし。整数でない番号の三角形ができる | 型（`usize`）で排除 |
| `N` が巨大 | メモリ不足か、終わらない | 同じ（メモリの確保に失敗して異常終了する。panic ではない） |
| 折れ目に、存在しない辺の名前 | 無視する | 同じ（`EdgeKey` で辺の表にないものは無視） |
| 折れ目の名前が `"5-4"`・`"a-b"` など正規形でない | 一致しないので無視する | 型で排除（`EdgeKey::new` は常に正規形） |
| `edgePoint` に範囲外の辺の番号 | `TypeError` | `None` |

### mesh.js（編集操作）

「型で排除」と書いた入力は、JS では無視される（または表の結果になる）ことを、期待値のスクリプトが確かめている。

| 入力 | JS 版 | Rust 版 |
| --- | --- | --- |
| 押し出し量が NaN・+無限大 | 0.02 で押し出す | 同じ |
| 押し出し量が -無限大 | -0.02 で押し出す（大きさは 100 でなく 0.02） | 同じ |
| 押し出し量が 0・-0・絶対値 0.02 未満 | 符号を保って ±0.02（0 と -0 は +0.02） | 同じ |
| 押し出し量の絶対値が 100 超 | 符号を保って ±100 | 同じ |
| 押し出し量が数でない（`undefined`・`null`・文字列・真偽値など） | 0.02 で押し出す | 型で排除（NaN を渡すと同じ結果） |
| 押し出す面の番号が範囲外 | 無視する | 同じ |
| 押し出す面の番号が負・整数でない・NaN・数でない | 無視する | 型で排除 |
| 押し出す面の番号の重複 | 最初の1回だけ数える（`faces` も重複なし、初出の順） | 同じ |
| 押し出す面が1つもない（空・すべて無効） | 入力の複製を返し、`faces` は空 | 同じ |
| 頂点だけを共有する選択面 | 頂点を面ごとに別々に複製する | 同じ |
| 押し出しで、面が存在しない頂点を参照 | 選択した面なら TypeError。選択しない面なら、番号が押し出しで増えた頂点に重なれば、その頂点を指す（例外なし）。重ならなければ、面に `undefined` が混ざる | 同じ番号の重なり方をする。例外か `undefined` になる入力は `Err(MeshError::VertexOutOfRange)` |
| 頂点が 3 つ未満の面・空の面・頂点の重複する面を押し出す | 例外なし（法線は `[0, 0, 0]` になりうる） | 同じ |
| 削除する面の番号が範囲外・重複 | 無視する | 同じ |
| 削除する面の番号が負・整数でない・NaN・数でない | 無視する | 型で排除 |
| すべての面を削除 | 頂点も面も空 | 同じ |
| 削除後に残る面が存在しない頂点を参照（`compact` も同じ） | 面に `undefined` が混ざる（例外なし） | `Err(MeshError::VertexOutOfRange)` |
| ループ分割・折れ目の辺が存在しない | 無視する（すべて無視されたら入力の複製） | 同じ |
| ループ分割・折れ目の辺の名前が正規形でない（`"1-0"`）・壊れている（`"a-b"`）・文字列でない | 存在しない辺として無視する | 型で排除（`EdgeKey::new(1, 0)` は `"0-1"` を指す） |
| ループ分割・折れ目の辺の重複 | ループ分割は1回だけ分割する。折れ目は、すべてが折れ目なら外し、そうでなければ付ける（重複は結果を変えない） | 同じ |
| ループ分割の辺の端点が存在しない頂点 | `midOf` にその辺がなければ TypeError。あれば例外なし（面は範囲外の番号のまま） | `midOf` がなければ `Err(MeshError::VertexOutOfRange)`、あれば同じ |
| ループ分割で、分割する辺を持つ 5 辺以上の面 | 角の番号を 3 で割った余りで選んだ3角の、三角形として分割する（残りの角は消える）。中点の定まらない辺を使うと、面に `undefined` が混ざる | 同じ。`undefined` になる入力は `Err(MeshError::UndefinedCorner)` |
| ループ分割で、分割する辺を持つ、頂点が 1〜2 個の面 | 面に `undefined` が混ざる | `Err(MeshError::UndefinedCorner)` |
| `midOf` に、分割しない辺の位置 | 無視する | 同じ |
| 座標に NaN・無限大 | 例外なし。値が伝わる | 同じ |
| 選択のモードが `'v'`・`'e'` 以外の文字列 | 面のモード | `SelectMode::from_js_name` が `Face` を返す |
| 選択の要素の型がモードと違う（辺のモードに数など） | 無視する | 型で排除（`Selection` の各モードは要素の型が決まっている） |
| 選択の点・面の番号が範囲外 | 無視する | 同じ |
| 選択の点・面の番号が負・整数でない・NaN | 無視する | 型で排除 |
| 選択の辺の端点が範囲外 | 無視する | 同じ |
| 選択の辺が、両端は範囲内だが存在しない辺 | 両端の頂点を入れる | 同じ |
| 選択の辺の名前が `"1-"`・`"-1"` | `Number('')` が 0 になるため、頂点 0 と 1 として扱う | 型で排除 |
| 選択した面が存在しない頂点を参照 | 範囲外の番号をそのまま頂点として返す | 同じ |
| 面への変換で、頂点のない面 | 常に選ばれる | 同じ |
| ミラーの `eps` が 0・NaN・無限大・負 | 例外なし。丸めの結果どおり（負では JS の `Math.round` の丸め方） | 同じ（`Math.round`・`Math.sign` と同じ関数で丸める） |
| ミラーで、同じ位置に複数の頂点 | 番号の最も大きい頂点を相手にする | 同じ |
| ミラーで、座標に NaN | NaN どうしは同じ位置とみなす | 同じ |
| 検査（`validateModel`）で、座標が 3 つでない | 座標が不正 | 型で排除（`Vec3`） |
| 検査で、面の頂点番号が負・整数でない | 範囲外 | 型で排除 |
| `PRESETS` に未知のキー | `PRESETS[key]` が `undefined`（`make` で TypeError） | `Preset::from_key` が `None` |

## 期待値（tests/fixtures/）

`npm run export:fixtures`（`scripts/export-gregory-fixtures.mjs`）が、JS 版の出力を JSON に書き出す。乱数は使わない。

- 非有限の数は、JSON で表せないため、文字列の `"NaN"`・`"Infinity"`・`"-Infinity"` にする。それ以外の数は、JSON の数値（JS の最短表記）。`-0` は `0` になる
- 数値をビット単位で読むため、`serde_json` の `float_roundtrip` を有効にしている

### `vector.json`

```json
{
  "add":     [{ "a": [x, y, z], "b": [x, y, z], "result": [x, y, z] }],
  "sub":     [{ "a": [...], "b": [...], "result": [...] }],
  "dot":     [{ "a": [...], "b": [...], "result": 0.0 }],
  "cross":   [{ "a": [...], "b": [...], "result": [...] }],
  "mul":     [{ "a": [...], "s": 0.0, "result": [...] }],
  "len":     [{ "a": [...], "result": 0.0 }],
  "norm":    [{ "a": [...], "result": [...] }],
  "bezier3": [{ "e": [[...], [...], [...], [...]], "t": 0.0, "result": [...] }]
}
```

### `build-model/<名前>.json`

1ファイル1ケース。JS が例外を投げたケースは `output` の代わりに `error`（JS の例外の文字列）を持つ。名前が `error-` で始まるケースだけが `error` を持つ。

```json
{
  "name": "cube-g1-n4",
  "input": {
    "v": [[x, y, z]],
    "f": [[0, 1, 2, 3]],
    "tau": 1,
    "mode": "g1",
    "n": 4,
    "creases": [[a, b]]
  },
  "output": {
    "nrm": [[x, y, z]],
    "edges": [{ "a": 0, "b": 1, "faces": [{ "fi": 0, "k": 0 }], "e": [[...], [...], [...], [...]], "crease": false }],
    "mesh": { "pos": [[...]], "nrm": [[...]], "tris": [[0, 1, 5]], "tri_face": [0] },
    "check": { "g0": 0, "g1": 0, "pairs": 12, "creased": 0, "samples": [{ "p": [...], "na": [...], "nb": [...] }] },
    "count": { "q": 6, "t": 0 },
    "patches": [{
      "kind": "quad",
      "ed": [{ "e": [[...], [...], [...], [...]], "c": [[...], [...], [...], [...]] }],
      "q": [[[...], [...]]],
      "evals": [{ "a": 0.5, "b": 0.5, "at": [...], "normal": [...] }],
      "edge_points": [{ "k": 0, "t": 0.3, "h": 1e-6, "p": [[...], [...]] }]
    }]
  }
}
```

- `input.mode` は JS に渡した文字列のまま（`"g1"`・`"linear"`・`"foo"`）。Rust では `Mode::from_js_name` で変換する
- `input.creases` は辺の両端の番号の組（小さい順）
- `patches[i].kind` は `"quad"` か `"tri"`。`q` は JS の `[null, q1, q2]` から `null` を除いた `[q1, q2]`
- `evals` は `Patch::at`・`Patch::normal` を、`edge_points` は `Patch::edge_point` を、決まった引数で評価した値

### `mesh/<名前>.json`

1ファイル1ケース。`op` が操作の名前（Rust の関数名）。結果は、次のどれか1つを持つ。

- `output`：JS の戻り値
- `error`：JS の例外の文字列。名前が `error-` で始まるケースだけが持つ
- `undefined_output: true`：JS が例外を投げず、面や辺に `undefined` の混ざった値を返した。名前が `undefined-` で始まるケースだけが持つ。Rust では `Err` になる

スクリプトは、名前の接頭辞と結果の種類が一致することに加え、Rust の型で渡せない入力（数でない押し出し量、整数でない番号、壊れた辺の名前、型の違う選択の要素、未知のモード、`eps` の省略）が、型で表せる入力と同じ結果になることを確かめる。

```json
{ "name": "extrude-cube-face0", "op": "extrude_faces", "input": { ... }, "output": { ... } }
```

モデルは `{ "v": [[x, y, z]], "f": [[0, 1, 2, 3]], "creases": [[a, b]] }`（`creases` は JS の `Set` の順。比べるときは集合として比べる）。辺は `[a, b]`（小さい順）。

| `op` | `input` | `output` |
| --- | --- | --- |
| `presets` | `{ "key": "cube" }` | `{ "name": "立方体", "model": モデル }` |
| `edge_map` | `{ "f": [[...]] }` | `[{ "a": 0, "b": 1, "faces": [0, 2] }]`（`faces` は面番号。JS の `Map` の順） |
| `face_normal` | `{ "v": [[...]], "f": [0, 1, 2] }` | `[x, y, z]` |
| `compact` | `{ "model": モデル }` | `{ "model": モデル, "remap": [0, null, 1] }`（`remap[i]` は元の頂点 `i` の新しい番号か `null`） |
| `delete_faces` | `{ "model": モデル, "face_idxs": [0] }` | モデル |
| `extrude_faces` | `{ "model": モデル, "face_idxs": [0], "dist": 0.5 }` | `{ "model": モデル, "faces": [0] }` |
| `loop_cut` | `{ "model": モデル, "edge_keys": [[0, 1]], "mid_of": null }`（`mid_of` は `null` か `[{ "edge": [a, b], "p": [x, y, z] }]`） | `{ "model": モデル, "edges": [[a, b]] }`（`edges` は順序つき） |
| `toggle_creases` | `{ "model": モデル, "edge_keys": [[0, 1]] }` | モデル |
| `selection_vertices` | `{ "model": モデル, "mode": "v", "items": [...] }`（`items` は点・面なら番号、辺なら `[a, b]`） | `[0, 1]`（順序つき） |
| `convert_selection` | `{ "model": モデル, "from": "v", "items": [...], "to": "e" }` | `to` のモードの要素（順序つき） |
| `mirror_map` | `{ "v": [[...]], "eps": 1e-4 }` | `[1, 0, -1]`（`-1` は相手なし） |
| `validate_model` | `{ "model": モデル }` | `["面0が5辺"]`（`ModelProblem` の `Display`） |
| `sequence` | `{ "model": モデル, "steps": [{ "op": "extrude_faces", "face_idxs": [0], "dist": 0.5 }] }` | 各段の `output` の配列。各段の入力のモデルは、前の段の出力のモデル（最初の段は `input.model`） |

- `dist`・`eps`・座標の非有限の数は、`"NaN"`・`"Infinity"`・`"-Infinity"` の文字列
