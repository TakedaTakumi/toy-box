// gregory.js の出力を、Rust 移植（crates/gregory-core）の期待値として JSON に書き出す。乱数は使わない
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { buildModel, bezier3, add, sub, mul, dot, cross, len, norm } from '../public/tools/gregory-mesh-editor/gregory.js';
import * as M from '../public/tools/gregory-mesh-editor/mesh.js';

const OUT = new URL('../crates/gregory-core/tests/fixtures/', import.meta.url);
const BUILD_DIR = new URL('build-model/', OUT);

// JSON は NaN と無限大を表せないため、"NaN"・"Infinity"・"-Infinity" の文字列にする
const replacer = (_, v) => (typeof v === 'number' && !Number.isFinite(v) ? String(v) : v);
const toJson = (x) => JSON.stringify(x, replacer) + '\n';

// ---- ベクトル演算と bezier3 ----

const VECTORS = [
  [0, 0, 0], [1, 0, 0], [3, 4, 12], [0.1, 0.2, 0.3], [-1.5, 2.25, -3.125],
  [1e-13, 0, 0], [1e-7, -1e-7, 1e-7], [1e300, 1e300, -1e300], [5e-324, 0, 0], [1e-200, 1e-200, 0],
  [NaN, 1, 2], [Infinity, NaN, 0], [-Infinity, 1, 1],
];
const SCALARS = [0, 1, -1, 1 / 3, 1e-6, 1e300, NaN, Infinity];
const BEZIER_CURVES = [
  [[0, 0, 0], [1, 2, 0], [3, 2, 1], [4, 0, -1]],
  [[-1, 1, 1], [-1 / 3, 1.2, 1.1], [1 / 3, 1.2, 1.1], [1, 1, 1]],
];
const BEZIER_TS = [0, 0.25, 1 / 3, 0.5, 1, -0.5, 1.5, NaN];

function vectorFixture() {
  const pairs = VECTORS.flatMap((a) => VECTORS.map((b) => [a, b]));
  const binary = (fn) => pairs.map(([a, b]) => ({ a, b, result: fn(a, b) }));
  return {
    add: binary(add),
    sub: binary(sub),
    dot: binary(dot),
    cross: binary(cross),
    mul: VECTORS.flatMap((a) => SCALARS.map((s) => ({ a, s, result: mul(a, s) }))),
    len: VECTORS.map((a) => ({ a, result: len(a) })),
    norm: VECTORS.map((a) => ({ a, result: norm(a) })),
    bezier3: BEZIER_CURVES.flatMap((e) => BEZIER_TS.map((t) => ({ e, t, result: bezier3(e, t) }))),
  };
}

// ---- buildModel ----

const pairsOf = (keys) => [...keys].map((k) => k.split('-').map(Number));
const fromModel = (m) => ({ v: m.V, f: m.F, creases: pairsOf(m.creases) });
const preset = (key) => fromModel(M.PRESETS[key].make());
const edgeKeysOf = (f) => [...M.edgeMap(f).keys()];
const jitter = (v, amp) => v.map((p, i) => p.map((x, k) => x + amp * Math.sin(1.7 * i + 2.3 * k + 0.5)));
const withVertex = (v, i, p) => v.map((q, j) => (j === i ? p : q.slice()));
const pentagon = (cx, cz, r) => Array.from({ length: 5 }, (_, i) => [cx + r * Math.cos((2 * Math.PI * i) / 5), 0, cz + r * Math.sin((2 * Math.PI * i) / 5)]);

function buildCases() {
  const cases = [];
  const push = (name, geo, opts = {}) => cases.push({ name, input: { creases: [], tau: 1, mode: 'g1', n: 4, ...geo, ...opts } });

  for (const key of Object.keys(M.PRESETS)) {
    const base = preset(key);
    const shaken = { ...base, v: jitter(base.v, 0.1) };
    push(`${key}-g1-n4`, base);
    push(`${key}-linear-tau-half-n3`, base, { mode: 'linear', tau: 0.5, n: 3 });
    push(`${key}-jitter-g1-n8`, shaken, { n: 8 });
    push(`${key}-jitter-alternate-creases-n4`, { ...shaken, creases: pairsOf(edgeKeysOf(base.f).filter((_, i) => i % 2 === 0)) });
  }

  const cube = preset('cube');
  const prism = preset('prism');
  const pyramid = preset('pyramid');
  const plane = preset('plane');
  const cubeTop = cube.f[0].map((v, i, f) => M.ekey(v, f[(i + 1) % f.length]));
  push('cube-top-creases-n4', { ...cube, creases: pairsOf(cubeTop) });
  push('prism-all-creases-n4', { ...prism, v: jitter(prism.v, 0.05), creases: pairsOf(edgeKeysOf(prism.f)) });
  push('cube-n1', cube, { n: 1 });
  push('cube-n2', cube, { n: 2 });
  push('prism-n1', prism, { n: 1 });
  push('pyramid-n16', pyramid, { n: 16 });
  push('cube-tau-zero-n4', cube, { tau: 0 });
  push('cube-tau-two-n4', cube, { tau: 2 });

  // 折れ目の両側がほぼ同じ平面で、交線の向きが不安定な場合
  const flatV = plane.v.map((p) => p.slice());
  flatV[4] = [-0.15, 0, 0];
  flatV[3] = [-1, 0.0012, 0];
  push('plane-near-coplanar-crease-n8', { ...plane, v: flatV, creases: [[4, 5], [4, 7]] }, { n: 8 });

  // 編集操作（mesh.js）で作った、四角形と三角形が混ざる形
  const cubeModel = M.PRESETS.cube.make();
  const extruded = M.extrudeFaces(cubeModel, [0], 0.5).model;
  push('cube-extruded-n4', fromModel(extruded));
  push('cube-extruded-creased-n4', fromModel(M.toggleCreases(extruded, edgeKeysOf(extruded.F).slice(0, 3))));
  push('prism-loopcut-n4', fromModel(M.loopCut(M.PRESETS.prism.make(), ['0-1']).model));
  push('pyramid-loopcut-n4', fromModel(M.loopCut(M.PRESETS.pyramid.make(), ['0-1']).model));
  const cutOnce = M.loopCut(cubeModel, ['0-1']).model;
  push('cube-loopcut-twice-n4', fromModel(M.loopCut(cutOnce, ['0-3']).model));
  push('cube-crease-loopcut-n4', fromModel(M.loopCut(M.toggleCreases(cubeModel, ['0-1']), ['0-1']).model));

  // 不正・極端だが、JS が例外を投げずに値を返す入力
  push('cube-n0', cube, { n: 0 });
  push('pyramid-tau-nan', pyramid, { tau: NaN });
  push('plane-nan-vertex', { ...plane, v: withVertex(plane.v, 4, [0, NaN, 0]) });
  push('plane-infinite-vertex', { ...plane, v: withVertex(plane.v, 4, [0, 0, Infinity]) });
  push('cube-unknown-mode', cube, { mode: 'foo' });
  push('cube-unused-vertex', { ...cube, v: [...cube.v, [5, 5, 5]] });
  push('cube-creases-not-on-edges', { ...cube, creases: [[0, 6], [2, 99]] });
  push('vertices-without-faces', { v: cube.v, f: [] });
  push('empty', { v: [], f: [] });
  push('single-triangle', { v: [[0, 0, 0], [1, 0, 0], [0, 0, -1]], f: [[0, 1, 2]] });
  push('single-quad', { v: [[0, 0, 0], [1, 0, 0], [1, 0.2, -1], [0, 0, -1]], f: [[0, 1, 2, 3]] });
  push('coincident-vertices', { v: [[0, 0, 0], [1, 0, 0], [1, 0, 0], [0, 1, 0]], f: [[0, 1, 2, 3]] });
  push('repeated-vertex-face', { v: [[0, 0, 0], [1, 0, 0], [0, 0, -1]], f: [[0, 1, 2, 2]] });
  push('non-manifold-edge', { v: [[0, 0, 0], [1, 0, 0], [0.5, 0, -1], [0.5, 0, 1], [0.5, 1, 0]], f: [[0, 1, 2], [1, 0, 3], [0, 1, 4]], creases: [[0, 1]] });
  push('single-pentagon', { v: pentagon(0, 0, 1), f: [[0, 1, 2, 3, 4]] });
  const pent = pentagon(0, 0, 1);
  const pentPair = [...pent, [3, 0, -0.5], [3, 0, 1], [2, 0, 2]];
  push('pentagon-pair-shared-first-edge', { v: pentPair, f: [[0, 1, 2, 3, 4], [1, 0, 5, 6, 7]] });

  // JS が例外を投げる入力
  push('error-vertex-out-of-range', { ...cube, f: cube.f.map((f, i) => (i === 2 ? [f[0], 8, f[2], f[3]] : f.slice())) });
  push('error-face-two-vertices', { v: [[0, 0, 0], [1, 0, 0], [0, 0, -1]], f: [[0, 1, 2], [0, 1]] });
  push('error-face-one-vertex', { v: [[0, 0, 0], [1, 0, 0], [0, 0, -1]], f: [[0, 1, 2], [0]] });
  push('error-face-empty', { v: [[0, 0, 0], [1, 0, 0], [0, 0, -1]], f: [[0, 1, 2], []] });
  push('error-pentagon-pair-shared-fourth-edge', { v: pentPair, f: [[0, 1, 2, 3, 4], [4, 3, 5, 6, 7]] });
  return cases;
}

const QUAD_PARAMS = [[0, 0], [1, 0], [1, 1], [0, 1], [0.5, 0.5], [0.25, 0.75], [0.1, 0.3], [1 / 3, 2 / 3], [-0.1, 1.2]];
const TRI_PARAMS = [[0, 0], [1, 0], [0, 1], [1 / 3, 1 / 3], [0.25, 0.5], [0.1, 0.1], [0.5, 0.5], [0.6, 0.6]];
const EDGE_TS = [0, 0.3, 1];
const EDGE_HS = [1e-6, 0.05];

function serializePatch(p) {
  const quad = p.type === 'q';
  const edgeCount = quad ? 4 : 3;
  const edgePoints = [];
  for (let k = 0; k < edgeCount; k++) {
    for (const t of EDGE_TS) for (const h of EDGE_HS) edgePoints.push({ k, t, h, p: p.edgePoint(k, t, h) });
  }
  return {
    kind: quad ? 'quad' : 'tri',
    ed: p.ed.map((d) => ({ e: d.E, c: d.c })),
    q: p.q.map((x) => [x[1], x[2]]),
    evals: (quad ? QUAD_PARAMS : TRI_PARAMS).map(([a, b]) => ({ a, b, at: p.at(a, b), normal: p.normal(a, b) })),
    edge_points: edgePoints,
  };
}

function serializeModel(r) {
  return {
    nrm: r.nrm,
    edges: r.edges.map((e) => ({ a: e.a, b: e.b, faces: e.faces.map(({ fi, k }) => ({ fi, k })), e: e.E, crease: e.crease })),
    mesh: { pos: r.mesh.pos, nrm: r.mesh.nrm, tris: r.mesh.tris, tri_face: r.mesh.triFace },
    check: {
      g0: r.check.g0,
      g1: r.check.g1,
      pairs: r.check.pairs,
      creased: r.check.creased,
      samples: r.check.samples.map(({ p, na, nb }) => ({ p, na, nb })),
    },
    count: r.count,
    patches: r.patches.map(serializePatch),
  };
}

function runCase({ name, input }) {
  const opts = { tau: input.tau, mode: input.mode, N: input.n, creases: new Set(input.creases.map(([a, b]) => M.ekey(a, b))) };
  try {
    return { name, input, output: serializeModel(buildModel(input.v, input.f, opts)) };
  } catch (e) {
    return { name, input, error: String(e) };
  }
}

await rm(BUILD_DIR, { recursive: true, force: true });
await mkdir(BUILD_DIR, { recursive: true });
await writeFile(new URL('vector.json', OUT), toJson(vectorFixture()));
const cases = buildCases();
for (const c of cases) {
  const result = runCase(c);
  if (c.name.startsWith('error-') !== ('error' in result)) throw new Error(`${c.name}: 例外の有無が想定と違う（${result.error ?? '例外なし'}）`);
  await writeFile(new URL(`${c.name}.json`, BUILD_DIR), toJson(result));
}
console.log(`vector.json と build-model/ の ${cases.length} 件を書き出した`);
