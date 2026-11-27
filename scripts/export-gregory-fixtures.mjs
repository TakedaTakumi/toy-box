// gregory.js と mesh.js の出力を、Rust 移植（crates/gregory-core）の期待値として JSON に書き出す。乱数は使わない
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { buildModel, bezier3, add, sub, mul, dot, cross, len, norm } from '../public/tools/gregory-mesh-editor/gregory.js';
import * as M from '../public/tools/gregory-mesh-editor/mesh.js';

const OUT = new URL('../crates/gregory-core/tests/fixtures/', import.meta.url);
const BUILD_DIR = new URL('build-model/', OUT);
const MESH_DIR = new URL('mesh/', OUT);

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

// ---- mesh.js の編集操作 ----

// 期待値の入力は Rust で読める形（辺は [a, b] の組）で持ち、JS に渡すときに変換する
const keysOf = (pairs) => pairs.map(([a, b]) => M.ekey(a, b));
const toJsModel = (m) => ({ V: m.v.map((p) => p.slice()), F: m.f.map((f) => f.slice()), creases: new Set(keysOf(m.creases)) });
const toJsItems = (mode, items) => (mode === 'e' ? keysOf(items) : items.slice());
const fromItems = (mode, set) => (mode === 'e' ? pairsOf(set) : [...set]);
const toMidOf = (list) => list && new Map(list.map(({ edge, p }) => [M.ekey(edge[0], edge[1]), p]));

const MESH_OPS = {
  presets: {
    run: (i) => {
      const p = M.PRESETS[i.key];
      return { name: p.name, model: p.make() };
    },
    serialize: (r) => ({ name: r.name, model: fromModel(r.model) }),
  },
  edge_map: {
    run: (i) => M.edgeMap(i.f),
    serialize: (r) => [...r.values()].map(({ a, b, faces }) => ({ a, b, faces })),
  },
  face_normal: { run: (i) => M.faceNormal(i.v, i.f), serialize: (r) => r },
  compact: {
    run: (i) => M.compact(toJsModel(i.model)),
    serialize: (r, i) => ({ model: fromModel(r.model), remap: i.model.v.map((_, k) => (r.remap.has(k) ? r.remap.get(k) : null)) }),
  },
  delete_faces: { run: (i) => M.deleteFaces(toJsModel(i.model), i.face_idxs), serialize: fromModel },
  extrude_faces: {
    run: (i) => M.extrudeFaces(toJsModel(i.model), i.face_idxs, i.dist),
    serialize: (r) => ({ model: fromModel(r.model), faces: r.faces }),
  },
  loop_cut: {
    run: (i) => M.loopCut(toJsModel(i.model), keysOf(i.edge_keys), toMidOf(i.mid_of)),
    serialize: (r) => ({ model: fromModel(r.model), edges: pairsOf(r.edges) }),
  },
  toggle_creases: { run: (i) => M.toggleCreases(toJsModel(i.model), keysOf(i.edge_keys)), serialize: fromModel },
  selection_vertices: {
    run: (i) => M.selectionVertices(toJsModel(i.model), i.mode, toJsItems(i.mode, i.items)),
    serialize: (r) => [...r],
  },
  convert_selection: {
    run: (i) => M.convertSelection(toJsModel(i.model), i.from, toJsItems(i.from, i.items), i.to),
    serialize: (r, i) => fromItems(i.to, r),
  },
  mirror_map: { run: (i) => M.mirrorMap(i.v, i.eps), serialize: (r) => r },
  validate_model: { run: (i) => M.validateModel(toJsModel(i.model)), serialize: (r) => r },
  // 操作を順に適用する。各段の出力を、次の段の入力のモデルにする
  sequence: {
    run: (i) => {
      let model = i.model;
      const raws = [];
      const outs = [];
      for (const { op, ...args } of i.steps) {
        const stepInput = { ...args, model };
        const raw = MESH_OPS[op].run(stepInput);
        const out = MESH_OPS[op].serialize(raw, stepInput);
        raws.push(raw);
        outs.push(out);
        model = out.model ?? out;
      }
      return { raws, outs };
    },
    serialize: (r) => r.outs,
  },
};

// 面の頂点が undefined になるなど、JS が例外を投げずに壊れた値を返したか
function hasUndefined(x, seen = new Set()) {
  if (x === undefined) return true;
  if (typeof x === 'string') return x.includes('undefined');
  if (x === null || typeof x !== 'object' || seen.has(x)) return false;
  seen.add(x);
  const values = x instanceof Map ? [...x.keys(), ...x.values()] : x instanceof Set ? [...x] : Array.isArray(x) ? x : Object.values(x);
  return values.some((v) => hasUndefined(v, seen));
}

const modelOf = (v, f, creases = []) => ({ v, f, creases });
const withFace = (m, fi, face) => ({ ...m, f: m.f.map((f, i) => (i === fi ? face : f.slice())) });
const pickFaces = (m, picks) => [...new Set(picks.map((p) => p % m.f.length))];
const pickEdges = (m, picks) => {
  const keys = edgeKeysOf(m.f);
  return pairsOf(new Set(picks.map((p) => keys[p % keys.length])));
};

// 各段の引数を、その時点のモデルから決めて、具体的な値として記録する
function sequenceInput(model, pickers) {
  let cur = model;
  const steps = [];
  for (const pick of pickers) {
    const step = pick(cur);
    steps.push(step);
    const { op, ...args } = step;
    const out = MESH_OPS[op].serialize(MESH_OPS[op].run({ ...args, model: cur }));
    cur = out.model ?? out;
  }
  return { model, steps };
}
const extrudeStep = (picks, dist) => (m) => ({ op: 'extrude_faces', face_idxs: pickFaces(m, picks), dist });
const loopStep = (picks) => (m) => ({ op: 'loop_cut', edge_keys: pickEdges(m, picks), mid_of: null });
const deleteStep = (picks) => (m) => ({ op: 'delete_faces', face_idxs: pickFaces(m, picks).slice(0, m.f.length - 1) });
const creaseStep = (picks) => (m) => ({ op: 'toggle_creases', edge_keys: pickEdges(m, picks) });

function meshCases() {
  const cases = [];
  const push = (name, op, input) => cases.push({ name, op, input });
  const keys = Object.keys(M.PRESETS);
  const cube = preset('cube');
  const plane = preset('plane');
  const pyramid = preset('pyramid');
  const prism = preset('prism');
  const pent = modelOf(pentagon(0, 0, 1), [[0, 1, 2, 3, 4]]);
  const tri = modelOf([[0, 0, 0], [1, 0, 0], [0, 0, -1]], [[0, 1, 2]]);
  // 面 2 が、存在しない頂点 8 を参照する立方体
  const cubeBad = withFace(cube, 2, [0, 8, 5, 4]);
  const cubeFarBad = withFace(cube, 2, [0, 99, 5, 4]);

  // PRESETS
  for (const key of keys) push(`presets-${key}`, 'presets', { key });
  push('error-presets-unknown-key', 'presets', { key: 'foo' });

  // edgeMap
  for (const key of keys) push(`edge-map-${key}`, 'edge_map', { f: preset(key).f });
  push('edge-map-empty', 'edge_map', { f: [] });
  push('edge-map-repeated-vertex', 'edge_map', { f: [[0, 1, 0, 2]] });
  push('edge-map-degenerate-faces', 'edge_map', { f: [[], [3], [1, 2], [5, 9, 2], [2, 1]] });

  // faceNormal
  push('face-normal-cube-top', 'face_normal', { v: cube.v, f: cube.f[0] });
  push('face-normal-pyramid-side', 'face_normal', { v: pyramid.v, f: pyramid.f[1] });
  push('face-normal-prism-cap', 'face_normal', { v: prism.v, f: prism.f[0] });
  push('face-normal-pentagon', 'face_normal', { v: pent.v, f: pent.f[0] });
  push('face-normal-nonplanar-quad', 'face_normal', { v: [[0, 0, 0], [1, 0.3, 0], [1, -0.2, -1], [0, 0.1, -1]], f: [0, 1, 2, 3] });
  push('face-normal-empty-face', 'face_normal', { v: cube.v, f: [] });
  push('face-normal-two-gon', 'face_normal', { v: cube.v, f: [0, 1] });
  push('face-normal-collinear', 'face_normal', { v: [[0, 0, 0], [1, 0, 0], [2, 0, 0]], f: [0, 1, 2] });
  push('face-normal-tiny', 'face_normal', { v: [[0, 0, 0], [1e-7, 0, 0], [0, 0, -1e-7]], f: [0, 1, 2] });
  push('face-normal-nan-vertex', 'face_normal', { v: [[0, 0, 0], [NaN, 0, 0], [0, 0, -1]], f: [0, 1, 2] });
  push('error-face-normal-out-of-range', 'face_normal', { v: tri.v, f: [0, 1, 3] });

  // compact
  for (const key of keys) push(`compact-${key}`, 'compact', { model: preset(key) });
  push('compact-unused-vertices-and-creases', 'compact', {
    model: modelOf([[9, 9, 9], [0, 0, 0], [8, 8, 8], [1, 0, 0], [0, 0, -1], [1, 0, -1]], [[1, 3, 5, 4]], [[1, 3], [3, 5], [0, 1], [1, 5], [4, 99]]),
  });
  push('compact-empty', 'compact', { model: modelOf([], []) });
  push('compact-no-faces', 'compact', { model: modelOf(cube.v, [], [[0, 1]]) });
  push('undefined-compact-out-of-range', 'compact', { model: cubeBad });

  // deleteFaces
  for (const key of keys) {
    push(`delete-faces-${key}-first`, 'delete_faces', { model: preset(key), face_idxs: [0] });
    push(`delete-faces-${key}-all`, 'delete_faces', { model: preset(key), face_idxs: preset(key).f.map((_, i) => i) });
  }
  const cubeCreased = { ...cube, creases: pairsOf(edgeKeysOf(cube.f)) };
  push('delete-faces-cube-creased-two', 'delete_faces', { model: cubeCreased, face_idxs: [2, 0] });
  push('delete-faces-duplicates-and-out-of-range', 'delete_faces', { model: cube, face_idxs: [1, 1, 99] });
  push('delete-faces-empty-selection', 'delete_faces', { model: { ...cube, v: [...cube.v, [5, 5, 5]] }, face_idxs: [] });
  push('delete-faces-removes-bad-face', 'delete_faces', { model: cubeBad, face_idxs: [2] });
  push('undefined-delete-faces-keeps-bad-face', 'delete_faces', { model: cubeBad, face_idxs: [0] });

  // extrudeFaces
  for (const key of keys) {
    const m = preset(key);
    m.f.forEach((_, fi) => push(`extrude-${key}-face${fi}`, 'extrude_faces', { model: m, face_idxs: [fi], dist: 0.5 }));
    push(`extrude-${key}-all`, 'extrude_faces', { model: m, face_idxs: m.f.map((_, i) => i), dist: 0.3 });
  }
  push('extrude-plane-vertex-sharing-faces', 'extrude_faces', { model: plane, face_idxs: [0, 3], dist: 0.4 });
  push('extrude-plane-adjacent-faces', 'extrude_faces', { model: plane, face_idxs: [0, 1], dist: 0.4 });
  push('extrude-cube-adjacent-faces', 'extrude_faces', { model: cube, face_idxs: [0, 2], dist: 0.5 });
  push('extrude-cube-adjacent-faces-reversed', 'extrude_faces', { model: cube, face_idxs: [2, 0], dist: 0.5 });
  push('extrude-cube-negative', 'extrude_faces', { model: cube, face_idxs: [0], dist: -0.3 });
  push('extrude-cube-duplicates', 'extrude_faces', { model: cube, face_idxs: [0, 0, 0], dist: 0.5 });
  push('extrude-cube-out-of-range-only', 'extrude_faces', { model: cube, face_idxs: [6, 99], dist: 0.5 });
  push('extrude-cube-empty-selection', 'extrude_faces', { model: cube, face_idxs: [], dist: 0.5 });
  push('extrude-cube-creased-top', 'extrude_faces', { model: { ...cube, creases: pairsOf(cube.f[0].map((v, i, f) => M.ekey(v, f[(i + 1) % f.length]))) }, face_idxs: [0], dist: 0.5 });
  const dists = { nan: NaN, infinity: Infinity, 'negative-infinity': -Infinity, zero: 0, tiny: 1e-9, 'negative-tiny': -1e-9, min: 0.02, max: 100, 'above-max': 1000, 'below-negative-max': -1000, huge: 1e308 };
  for (const [label, dist] of Object.entries(dists)) push(`extrude-cube-dist-${label}`, 'extrude_faces', { model: cube, face_idxs: [0], dist });
  push('extrude-repeated-vertex-face', 'extrude_faces', { model: modelOf(tri.v, [[0, 1, 2, 2]]), face_idxs: [0], dist: 0.5 });
  push('extrude-two-gon', 'extrude_faces', { model: modelOf(tri.v, [[0, 1, 2], [0, 1]]), face_idxs: [1], dist: 0.5 });
  push('extrude-empty-face', 'extrude_faces', { model: modelOf(tri.v, [[0, 1, 2], []]), face_idxs: [1, 0], dist: 0.5 });
  push('extrude-pentagon', 'extrude_faces', { model: pent, face_idxs: [0], dist: 0.5 });
  push('extrude-nan-vertex', 'extrude_faces', { model: { ...plane, v: withVertex(plane.v, 4, [0, NaN, 0]) }, face_idxs: [0, 1], dist: 0.5 });
  push('extrude-unused-vertex', 'extrude_faces', { model: { ...cube, v: [...cube.v, [5, 5, 5]] }, face_idxs: [0], dist: 0.5 });
  push('error-extrude-selected-out-of-range', 'extrude_faces', { model: cubeBad, face_idxs: [2], dist: 0.5 });
  // 押し出しで増えた頂点の番号と重なる範囲外の番号は、その頂点を指すことになり、例外にならない
  push('extrude-unselected-out-of-range-aliases-new-vertex', 'extrude_faces', { model: cubeBad, face_idxs: [0], dist: 0.5 });
  push('undefined-extrude-unselected-out-of-range', 'extrude_faces', { model: cubeFarBad, face_idxs: [0], dist: 0.5 });

  // loopCut
  for (const key of keys) {
    const m = preset(key);
    pairsOf(edgeKeysOf(m.f)).forEach((e, i) => push(`loop-cut-${key}-edge${i}`, 'loop_cut', { model: m, edge_keys: [e], mid_of: null }));
  }
  push('loop-cut-cube-cross', 'loop_cut', { model: cube, edge_keys: [[0, 1], [0, 3]], mid_of: null });
  push('loop-cut-cube-cross-reversed', 'loop_cut', { model: cube, edge_keys: [[0, 3], [0, 1]], mid_of: null });
  push('loop-cut-cube-duplicate-keys', 'loop_cut', { model: cube, edge_keys: [[0, 1], [0, 1], [2, 3]], mid_of: null });
  push('loop-cut-cube-missing-edge-only', 'loop_cut', { model: cube, edge_keys: [[0, 6], [2, 99]], mid_of: null });
  push('loop-cut-cube-empty', 'loop_cut', { model: cube, edge_keys: [], mid_of: null });
  push('loop-cut-cube-all-creased', 'loop_cut', { model: cubeCreased, edge_keys: [[0, 1]], mid_of: null });
  push('loop-cut-cube-crease-off-ring', 'loop_cut', { model: { ...cube, creases: [[0, 3], [0, 6]] }, edge_keys: [[0, 1]], mid_of: null });
  const cubeJs = M.PRESETS.cube.make();
  const built = buildModel(cubeJs.V, cubeJs.F, { tau: 1, mode: 'g1', N: 4, creases: cubeJs.creases });
  const mids = built.edges.map((e) => ({ edge: [e.a, e.b], p: bezier3(e.E, 0.5) }));
  push('loop-cut-cube-mid-of-curves', 'loop_cut', { model: cube, edge_keys: [[0, 1]], mid_of: mids });
  push('loop-cut-cube-mid-of-partial', 'loop_cut', { model: cube, edge_keys: [[0, 1]], mid_of: [{ edge: [2, 3], p: [0.5, 2, -0.5] }, { edge: [0, 6], p: [9, 9, 9] }] });
  push('loop-cut-pyramid-triangle-two-edges', 'loop_cut', { model: pyramid, edge_keys: [[0, 4], [1, 4]], mid_of: null });
  push('loop-cut-pyramid-triangle-three-edges', 'loop_cut', { model: pyramid, edge_keys: [[0, 1], [1, 4], [0, 4]], mid_of: null });
  push('loop-cut-triangle-edges-0-2', 'loop_cut', { model: tri, edge_keys: [[2, 0], [0, 1]], mid_of: null });
  push('loop-cut-pentagon-one-edge', 'loop_cut', { model: pent, edge_keys: [[0, 1]], mid_of: null });
  push('loop-cut-pentagon-adjacent-edges', 'loop_cut', { model: pent, edge_keys: [[0, 1], [1, 2]], mid_of: null });
  push('loop-cut-nan-vertex', 'loop_cut', { model: { ...plane, v: withVertex(plane.v, 4, [0, NaN, 0]) }, edge_keys: [[3, 4]], mid_of: null });
  push('loop-cut-bad-face-off-ring', 'loop_cut', { model: cubeBad, edge_keys: [[1, 2]], mid_of: null });
  push('loop-cut-bad-edge-with-mid', 'loop_cut', { model: cubeBad, edge_keys: [[0, 8]], mid_of: [{ edge: [0, 8], p: [0, 0, 0] }] });
  push('undefined-loop-cut-pentagon-edges-0-3', 'loop_cut', { model: pent, edge_keys: [[0, 1], [3, 4]], mid_of: null });
  push('undefined-loop-cut-two-gon', 'loop_cut', { model: modelOf(tri.v, [[0, 1, 2], [0, 1]]), edge_keys: [[0, 1]], mid_of: null });
  push('error-loop-cut-out-of-range', 'loop_cut', { model: cubeBad, edge_keys: [[0, 8]], mid_of: null });

  // toggleCreases
  push('toggle-creases-cube-on', 'toggle_creases', { model: cube, edge_keys: [[0, 1], [1, 2]] });
  push('toggle-creases-cube-off', 'toggle_creases', { model: { ...cube, creases: [[0, 1], [1, 2], [2, 3]] }, edge_keys: [[0, 1], [1, 2]] });
  push('toggle-creases-cube-mixed', 'toggle_creases', { model: { ...cube, creases: [[0, 1]] }, edge_keys: [[0, 1], [1, 2]] });
  push('toggle-creases-cube-duplicates-off', 'toggle_creases', { model: { ...cube, creases: [[0, 1]] }, edge_keys: [[0, 1], [0, 1]] });
  push('toggle-creases-cube-missing-edge-only', 'toggle_creases', { model: { ...cube, creases: [[0, 1]] }, edge_keys: [[0, 6]] });
  push('toggle-creases-cube-missing-and-on', 'toggle_creases', { model: { ...cube, creases: [[0, 1], [0, 6]] }, edge_keys: [[0, 6], [0, 1]] });
  push('toggle-creases-cube-empty', 'toggle_creases', { model: { ...cube, creases: [[0, 1]] }, edge_keys: [] });

  // selectionVertices・convertSelection
  const items = { v: [6, 0, 1, 5, 99, 6], e: [[4, 5], [0, 1], [1, 5], [0, 6], [2, 99]], f: [2, 0, 99, 2] };
  for (const mode of ['v', 'e', 'f']) {
    push(`selection-vertices-cube-${mode}`, 'selection_vertices', { model: cube, mode, items: items[mode] });
    push(`selection-vertices-cube-${mode}-empty`, 'selection_vertices', { model: cube, mode, items: [] });
    for (const to of ['v', 'e', 'f']) {
      push(`convert-selection-cube-${mode}-to-${to}`, 'convert_selection', { model: cube, from: mode, items: items[mode], to });
      push(`convert-selection-prism-${mode}-to-${to}`, 'convert_selection', { model: prism, from: mode, items: mode === 'e' ? [[0, 1], [1, 4]] : [0, 1, 4], to });
    }
  }
  push('selection-vertices-bad-face', 'selection_vertices', { model: cubeBad, mode: 'f', items: [2] });
  push('convert-selection-empty-face', 'convert_selection', { model: modelOf(tri.v, [[0, 1, 2], []]), from: 'v', items: [0], to: 'f' });

  // mirrorMap
  for (const key of keys) push(`mirror-map-${key}`, 'mirror_map', { v: preset(key).v, eps: 1e-4 });
  push('mirror-map-extruded-cube', 'mirror_map', { v: fromModel(M.extrudeFaces(M.PRESETS.cube.make(), [4], 0.5).model).v, eps: 1e-4 });
  push('mirror-map-empty', 'mirror_map', { v: [], eps: 1e-4 });
  push('mirror-map-duplicates', 'mirror_map', { v: [[1, 0, 0], [1, 0, 0], [-1, 0, 0], [0, 2, 0], [0, 2, 0]], eps: 1e-4 });
  push('mirror-map-rounding-boundary', 'mirror_map', { v: [[0.00005, 0, 0], [-0.00005, 0, 0], [0.00015, 1, 0], [-0.000149, 1, 0], [0.00004, 2, 0], [0.00025, 3, 0], [-0.00025, 3, 0]], eps: 1e-4 });
  push('mirror-map-non-finite', 'mirror_map', { v: [[NaN, 0, 0], [NaN, 0, 0], [Infinity, 1, 1], [-Infinity, 1, 1], [0, NaN, 0], [1, NaN, 0], [-1, NaN, 0]], eps: 1e-4 });
  push('mirror-map-overflow', 'mirror_map', { v: [[1e305, 0, 0], [-1.5e305, 0, 0], [1e300, 0, 0], [-1e300, 0, 0]], eps: 1e-4 });
  push('mirror-map-eps-zero', 'mirror_map', { v: [[0, 0, 0], [1, 0, 0], [-2, 0, 0], [0, 1, 0]], eps: 0 });
  push('mirror-map-eps-nan', 'mirror_map', { v: [[0, 0, 0], [1, 0, 0], [-1, 0, 0]], eps: NaN });
  push('mirror-map-eps-infinity', 'mirror_map', { v: [[0, 0, 0], [1, 0, 0], [-1, 5, 0]], eps: Infinity });
  push('mirror-map-eps-negative', 'mirror_map', { v: [[2.5, 0, 0], [-2.5, 0, 0], [2.4, 0, 0], [-2.6, 0, 0], [0.4, 1, 0], [-0.4, 1, 0]], eps: -1 });

  // validateModel
  for (const key of keys) push(`validate-model-${key}`, 'validate_model', { model: preset(key) });
  push('validate-model-empty', 'validate_model', { model: modelOf([], []) });
  push('validate-model-many-problems', 'validate_model', {
    model: modelOf([[0, 0, 0], [1, NaN, 0], [Infinity, 0, 0], [0, 0, 1], [5, 5, 5], [1, 1, 1]], [[0, 1, 2], [0, 1, 2], [0, 1], [0, 1, 2, 3, 0], [0, 9, 2, 10], []]),
  });
  push('validate-model-non-manifold', 'validate_model', { model: modelOf([[0, 0, 0], [1, 0, 0], [0.5, 0, -1], [0.5, 0, 1], [0.5, 1, 0]], [[0, 1, 2], [1, 0, 3], [0, 1, 4]]) });
  push('validate-model-flipped-face', 'validate_model', { model: withFace(cube, 0, [7, 6, 5, 4]) });
  push('validate-model-unused-vertex', 'validate_model', { model: { ...cube, v: [...cube.v, [5, 5, 5]] } });

  // 操作の連続適用
  push('sequence-cube', 'sequence', sequenceInput(cube, [extrudeStep([0], 0.5), loopStep([0]), creaseStep([1, 5]), extrudeStep([2, 3], -0.3), deleteStep([4]), loopStep([7])]));
  push('sequence-plane', 'sequence', sequenceInput(plane, [extrudeStep([0, 3], 0.4), creaseStep([0, 1, 2]), loopStep([3]), deleteStep([1]), extrudeStep([0, 1, 2, 3], 0.2)]));
  push('sequence-pyramid', 'sequence', sequenceInput(pyramid, [loopStep([0]), extrudeStep([1], 0.3), creaseStep([2]), creaseStep([2]), loopStep([5, 9])]));
  push('sequence-prism', 'sequence', sequenceInput(prism, [creaseStep([0, 1, 2, 3, 4, 5, 6, 7, 8]), loopStep([0]), extrudeStep([0, 2], 1), deleteStep([0, 1]), loopStep([3])]));
  push('sequence-cube-repeated-loops', 'sequence', sequenceInput(cube, [loopStep([0]), loopStep([5]), loopStep([11, 2]), extrudeStep([3, 4, 5], 0.25), creaseStep([0, 3, 6, 9])]));
  return cases;
}

function runMeshCase({ name, op, input }) {
  const { run, serialize } = MESH_OPS[op];
  let raw;
  try {
    raw = run(input);
  } catch (e) {
    return { name, op, input, error: String(e) };
  }
  if (hasUndefined(raw)) return { name, op, input, undefined_output: true };
  return { name, op, input, output: serialize(raw, input) };
}

// Rust では型で渡せない入力（数でない押し出し量、整数でない番号、壊れた辺の名前、未知のモードなど）が、
// 型で表せる入力と同じ結果になることを確かめる
const JUNK_INDICES = [-1, 1.5, NaN, 1e9, '0', null, undefined];
const JUNK_KEYS = ['a-b', '1-0', '0-1-2', '', 5, null, undefined];
const JUNK_ITEMS = { v: [-1, 1.5, NaN, 1e9, 'x', '0', null, undefined, true], e: [0, 1, 1.5, null, undefined, true, 'a-b', '1-2-3', '99-100', 'x'], f: [-1, 1.5, NaN, 1e9, 'x', '0', null, undefined, true] };

function checkJsOnlyInputs({ name, op, input }) {
  const outputOf = (run) => {
    try {
      const raw = run();
      return hasUndefined(raw) ? 'undefined' : JSON.stringify(MESH_OPS[op].serialize(raw, input), replacer);
    } catch (e) {
      return `error: ${String(e)}`;
    }
  };
  const want = outputOf(() => MESH_OPS[op].run(input));
  const variants = [];
  if (op === 'extrude_faces') {
    const m = () => toJsModel(input.model);
    variants.push(() => M.extrudeFaces(m(), [...input.face_idxs, ...JUNK_INDICES], input.dist));
    if (Number.isNaN(input.dist)) for (const d of [undefined, null, 'abc', '-1', true, false, {}]) variants.push(() => M.extrudeFaces(m(), input.face_idxs, d));
  } else if (op === 'delete_faces') {
    variants.push(() => M.deleteFaces(toJsModel(input.model), [...input.face_idxs, ...JUNK_INDICES]));
  } else if (op === 'loop_cut') {
    variants.push(() => M.loopCut(toJsModel(input.model), [...JUNK_KEYS, ...keysOf(input.edge_keys)], toMidOf(input.mid_of)));
  } else if (op === 'toggle_creases') {
    variants.push(() => M.toggleCreases(toJsModel(input.model), [...keysOf(input.edge_keys), ...JUNK_KEYS]));
  } else if (op === 'selection_vertices') {
    variants.push(() => M.selectionVertices(toJsModel(input.model), input.mode, [...toJsItems(input.mode, input.items), ...JUNK_ITEMS[input.mode]]));
  } else if (op === 'convert_selection') {
    const items = toJsItems(input.from, input.items);
    if (input.from === 'f') variants.push(() => M.convertSelection(toJsModel(input.model), 'x', items, input.to));
    if (input.to === 'f') variants.push(() => M.convertSelection(toJsModel(input.model), input.from, items, 'x'));
  } else if (op === 'mirror_map' && input.eps === 1e-4) {
    variants.push(() => M.mirrorMap(input.v));
  }
  for (const run of variants) {
    const got = outputOf(run);
    if (got !== want) throw new Error(`${name}: JS だけで渡せる入力の結果が、型で表せる入力の結果と違う`);
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

await rm(MESH_DIR, { recursive: true, force: true });
await mkdir(MESH_DIR, { recursive: true });
const meshList = meshCases();
if (new Set(meshList.map((c) => c.name)).size !== meshList.length) throw new Error('mesh の期待値の名前が重複している');
for (const c of meshList) {
  const result = runMeshCase(c);
  const got = 'error' in result ? 'error' : 'undefined_output' in result ? 'undefined' : 'output';
  const want = c.name.startsWith('error-') ? 'error' : c.name.startsWith('undefined-') ? 'undefined' : 'output';
  if (got !== want) throw new Error(`${c.name}: 結果の種類が想定（${want}）と違う（${got}${result.error ? `: ${result.error}` : ''}）`);
  checkJsOnlyInputs(c);
  await writeFile(new URL(`${c.name}.json`, MESH_DIR), toJson(result));
}
console.log(`vector.json と build-model/ の ${cases.length} 件、mesh/ の ${meshList.length} 件を書き出した`);
