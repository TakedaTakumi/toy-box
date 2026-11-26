# gregory-core

グレゴリーメッシュエディタの、グレゴリーパッチの数学を Rust に移したライブラリ。JS 試作の `public/tools/gregory-mesh-editor/gregory.js` を、同じデータ構造と同じ浮動小数点の演算順序で移している（忠実な移植）。同じ入力に対して、JS 版と同じ値（許容誤差内）を返すことを目標にする。

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
| `topology` | 辺の名前・面の角・辺の表 | `ekey`・`edgeMap` |
| `normals`・`union_find` | 頂点の法線と、折れ目で区切った面の群ごとの角の法線 | `nrmV`・`corner` の計算 |
| `curves` | 辺の3次ベジェ曲線と両端の接線 | `tangent`・`curve` |
| `edge_data` | 辺をまたぐ接線場と、内部制御点の元になる係数 | `edgeData` |
| `patch` | 四角形・三角形のグレゴリーパッチ | `quadPatch`・`triPatch`・`blend`・`normalAt` |
| `continuity` | 共有辺の G0・G1 の検証 | `sampleEdge` と検証のループ |
| `tessellate` | 描画用の三角形分割 | `tessellate` |
| `model` | 全体の組み立て | `buildModel` |

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

## 不正な入力

JS 版の振る舞いに合わせている。型で表せない入力（負の番号など）は、Rust では渡せない。

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
