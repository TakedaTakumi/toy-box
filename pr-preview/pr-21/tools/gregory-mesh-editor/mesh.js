import { add, mul, cross, norm } from './gregory.js';

export const ekey = (a, b) => (a < b ? a + '-' + b : b + '-' + a);

export function cloneModel(m) {
  return { V: m.V.map((v) => v.slice()), F: m.F.map((f) => f.slice()), creases: new Set(m.creases) };
}

export function faceNormal(V, f) {
  let n = [0, 0, 0];
  for (let i = 0; i < f.length; i++) n = add(n, cross(V[f[i]], V[f[(i + 1) % f.length]]));
  return norm(n);
}

export function edgeMap(F) {
  const map = new Map();
  F.forEach((f, fi) => {
    for (let i = 0; i < f.length; i++) {
      const a = f[i], b = f[(i + 1) % f.length], k = ekey(a, b);
      if (!map.has(k)) map.set(k, { a: Math.min(a, b), b: Math.max(a, b), faces: [] });
      map.get(k).faces.push(fi);
    }
  });
  return map;
}

// 使われていない頂点を取り除き、折れ目の番号も振り直す
export function compact(model) {
  const used = new Set();
  model.F.forEach((f) => f.forEach((v) => used.add(v)));
  const remap = new Map();
  const V = [];
  model.V.forEach((p, i) => { if (used.has(i)) { remap.set(i, V.length); V.push(p.slice()); } });
  const F = model.F.map((f) => f.map((v) => remap.get(v)));
  const edges = edgeMap(F);
  const creases = new Set();
  for (const k of model.creases) {
    const [a, b] = k.split('-').map(Number);
    if (remap.has(a) && remap.has(b) && edges.has(ekey(remap.get(a), remap.get(b)))) creases.add(ekey(remap.get(a), remap.get(b)));
  }
  return { model: { V, F, creases }, remap };
}

export function deleteFaces(model, faceIdxs) {
  const del = new Set(faceIdxs);
  const F = model.F.filter((_, i) => !del.has(i));
  return compact({ V: model.V, F, creases: model.creases }).model;
}

const MIN_EXTRUDE = 0.02;
const MAX_EXTRUDE = 100;

export function extrudeFaces(model, faceIdxs, rawDist) {
  // 頂点が重なると面が潰れて曲面が作れず、巨大だと座標が溢れるため、押し出し量を範囲に収める
  const sign = typeof rawDist === 'number' && rawDist < 0 ? -1 : 1;
  const mag = typeof rawDist === 'number' && Number.isFinite(rawDist) ? Math.min(MAX_EXTRUDE, Math.max(MIN_EXTRUDE, Math.abs(rawDist))) : MIN_EXTRUDE;
  const dist = sign * mag;
  const sel = new Set(faceIdxs.filter((i) => Number.isInteger(i) && i >= 0 && i < model.F.length));
  if (!sel.size) return { model: cloneModel(model), faces: [] };
  const { V, F } = model;
  // 頂点ごとに、辺でつながった選択面の扇を別々に押し出す。頂点だけを共有する面で、複製した頂点を共有しない
  const edges = edgeMap(F);
  const parent = new Map();
  for (const fi of sel) for (const v of F[fi]) parent.set(fi + ':' + v, fi + ':' + v);
  const find = (k) => (parent.get(k) === k ? k : (parent.set(k, find(parent.get(k))), parent.get(k)));
  for (const e of edges.values()) {
    const fs = e.faces.filter((fi) => sel.has(fi));
    for (let i = 1; i < fs.length; i++) for (const v of [e.a, e.b]) parent.set(find(fs[i] + ':' + v), find(fs[0] + ':' + v));
  }
  const fanKey = (fi, v) => v + '#' + find(fi + ':' + v);
  const acc = new Map();
  for (const fi of sel) {
    const n = faceNormal(V, F[fi]);
    for (const v of F[fi]) { const k = fanKey(fi, v); acc.set(k, add(acc.get(k) || [0, 0, 0], n)); }
  }
  const V2 = V.map((v) => v.slice());
  const dup = new Map();
  for (const [k, n] of acc) { dup.set(k, V2.length); V2.push(add(V[Number(k.split('#')[0])], mul(norm(n), dist))); }
  const dupOf = (fi, v) => dup.get(fanKey(fi, v));
  const F2 = F.map((f, fi) => (sel.has(fi) ? f.map((v) => dupOf(fi, v)) : f.slice()));
  const directed = new Set();
  for (const fi of sel) {
    const f = F[fi];
    for (let i = 0; i < f.length; i++) directed.add(f[i] + '>' + f[(i + 1) % f.length]);
  }
  for (const fi of sel) {
    const f = F[fi];
    for (let i = 0; i < f.length; i++) {
      const a = f[i], b = f[(i + 1) % f.length];
      if (!directed.has(b + '>' + a)) F2.push([a, b, dupOf(fi, b), dupOf(fi, a)]);
    }
  }
  const { model: out } = compact({ V: V2, F: F2, creases: model.creases });
  return { model: out, faces: [...sel] };
}

// 選んだ辺と平行な辺を四角形の列にそって集め、すべてを中点で分割する
export function loopCut(model, edgeKeys, midOf) {
  const { V, F } = model;
  const edges = edgeMap(F);
  const ring = new Set();
  const stack = edgeKeys.filter((k) => edges.has(k));
  while (stack.length) {
    const k = stack.pop();
    if (ring.has(k)) continue;
    ring.add(k);
    for (const fi of edges.get(k).faces) {
      const f = F[fi];
      if (f.length !== 4) continue;
      for (let i = 0; i < 4; i++) {
        if (ekey(f[i], f[(i + 1) % 4]) === k) stack.push(ekey(f[(i + 2) % 4], f[(i + 3) % 4]));
      }
    }
  }
  if (!ring.size) return { model: cloneModel(model), edges: [] };

  const V2 = V.map((v) => v.slice());
  const mid = new Map();
  for (const k of ring) {
    const { a, b } = edges.get(k);
    mid.set(k, V2.length);
    V2.push(midOf && midOf.get(k) ? midOf.get(k).slice() : mul(add(V[a], V[b]), 0.5));
  }
  const newEdges = [];
  const link = (a, b) => newEdges.push(ekey(a, b));
  const F2 = [];
  for (const f of F) {
    const n = f.length;
    const marked = [];
    for (let i = 0; i < n; i++) if (ring.has(ekey(f[i], f[(i + 1) % n]))) marked.push(i);
    if (!marked.length) { F2.push(f.slice()); continue; }
    const m = (i) => mid.get(ekey(f[i % n], f[(i + 1) % n]));
    if (n === 4 && marked.length === 2) {
      const s = marked[0] === 0 && marked[1] === 2 ? 0 : marked[0] === 1 && marked[1] === 3 ? 1 : marked[0];
      const v = (i) => f[(s + i) % 4];
      const mA = m(s), mB = m(s + 2);
      F2.push([v(0), mA, mB, v(3)], [mA, v(1), v(2), mB]);
      link(mA, mB);
    } else if (n === 4) {
      const mm = [m(0), m(1), m(2), m(3)];
      const c = V2.length;
      V2.push(mul(add(add(V2[mm[0]], V2[mm[1]]), add(V2[mm[2]], V2[mm[3]])), 0.25));
      F2.push([f[0], mm[0], c, mm[3]], [mm[0], f[1], mm[1], c], [c, mm[1], f[2], mm[2]], [mm[3], c, mm[2], f[3]]);
      mm.forEach((x) => link(x, c));
    } else if (marked.length === 1) {
      const s = marked[0];
      const v = (i) => f[(s + i) % 3];
      const mA = m(s);
      F2.push([v(0), mA, v(2)], [mA, v(1), v(2)]);
      link(mA, v(2));
    } else if (marked.length === 2) {
      const s = marked.includes(0) && marked.includes(1) ? 0 : marked.includes(1) && marked.includes(2) ? 1 : 2;
      const v = (i) => f[(s + i) % 3];
      const mA = m(s), mB = m(s + 1);
      F2.push([mA, v(1), mB], [v(0), mA, mB, v(2)]);
      link(mA, mB);
    } else {
      const mm = [m(0), m(1), m(2)];
      F2.push([f[0], mm[0], mm[2]], [mm[0], f[1], mm[1]], [mm[2], mm[1], f[2]], [mm[0], mm[1], mm[2]]);
      link(mm[0], mm[1]); link(mm[1], mm[2]); link(mm[2], mm[0]);
    }
  }
  const creases = new Set(model.creases);
  for (const k of ring) {
    if (!creases.has(k)) continue;
    const { a, b } = edges.get(k);
    creases.delete(k);
    creases.add(ekey(a, mid.get(k)));
    creases.add(ekey(mid.get(k), b));
  }
  return { model: { V: V2, F: F2, creases }, edges: [...new Set(newEdges)] };
}

export function toggleCreases(model, edgeKeys) {
  const edges = edgeMap(model.F);
  const keys = edgeKeys.filter((k) => edges.has(k));
  const out = cloneModel(model);
  const allOn = keys.length > 0 && keys.every((k) => out.creases.has(k));
  for (const k of keys) { if (allOn) out.creases.delete(k); else out.creases.add(k); }
  return out;
}

// 選択の要素（点は頂点番号、辺は "a-b"、面は面番号）に含まれる頂点の集合。不正な要素は無視する
export function selectionVertices(model, mode, items) {
  const out = new Set();
  const nV = model.V.length;
  const isVertex = (x) => Number.isInteger(x) && x >= 0 && x < nV;
  for (const it of items) {
    if (mode === 'v') {
      if (isVertex(it)) out.add(it);
    } else if (mode === 'e') {
      const ends = typeof it === 'string' ? it.split('-').map(Number) : [];
      if (ends.length === 2 && ends.every(isVertex)) ends.forEach((x) => out.add(x));
    } else if (Number.isInteger(it) && it >= 0 && it < model.F.length) {
      model.F[it].forEach((v) => out.add(v));
    }
  }
  return out;
}

// 選択を別のモードの要素に変換する。辺は両端が選ばれたもの、面はすべての頂点が選ばれたもの
export function convertSelection(model, from, items, to) {
  const verts = selectionVertices(model, from, items);
  const out = new Set();
  if (to === 'v') verts.forEach((v) => out.add(v));
  else if (to === 'e') {
    for (const [k, e] of edgeMap(model.F)) if (verts.has(e.a) && verts.has(e.b)) out.add(k);
  } else {
    model.F.forEach((f, i) => { if (f.every((v) => verts.has(v))) out.add(i); });
  }
  return out;
}

// x=0 の面について対称な頂点の組を、位置の一致で探す。無ければ -1
export function mirrorMap(V, eps = 1e-4) {
  // 0 をはさんで丸めが対称になるよう、絶対値で丸める
  const q = (x) => Math.sign(x) * Math.round(Math.abs(x) / eps);
  const at = new Map();
  V.forEach((p, i) => at.set(q(p[0]) + ',' + q(p[1]) + ',' + q(p[2]), i));
  return V.map((p) => {
    const j = at.get(q(-p[0]) + ',' + q(p[1]) + ',' + q(p[2]));
    return j === undefined ? -1 : j;
  });
}

export function validateModel(model) {
  const problems = [];
  const { V, F } = model;
  V.forEach((p, i) => { if (p.length !== 3 || p.some((x) => !Number.isFinite(x))) problems.push(`頂点${i}の座標が不正`); });
  const directed = new Map();
  F.forEach((f, fi) => {
    if (f.length !== 3 && f.length !== 4) problems.push(`面${fi}が${f.length}辺`);
    if (new Set(f).size !== f.length) problems.push(`面${fi}に重複頂点`);
    f.forEach((v, i) => {
      if (!Number.isInteger(v) || v < 0 || v >= V.length) problems.push(`面${fi}の頂点番号が範囲外`);
      const k = v + '>' + f[(i + 1) % f.length];
      directed.set(k, (directed.get(k) || 0) + 1);
    });
  });
  for (const [k, c] of directed) if (c > 1) problems.push(`有向辺${k}が${c}回使われている`);
  for (const [k, e] of edgeMap(F)) if (e.faces.length > 2) problems.push(`辺${k}が${e.faces.length}面で共有されている`);
  const used = new Set(F.flat());
  V.forEach((_, i) => { if (!used.has(i)) problems.push(`頂点${i}が未使用`); });
  return problems;
}

const cubeV = [[-1, -1, -1], [1, -1, -1], [1, 1, -1], [-1, 1, -1], [-1, -1, 1], [1, -1, 1], [1, 1, 1], [-1, 1, 1]];
// 高さを z で書いた座標を、Y-up（x, z, -y）に直す。回転なので、右手系も面の向きも変わらない
const make = (V, F) => ({ V: V.map(([x, y, z]) => [x, z, 0 - y]), F, creases: new Set() });

export const PRESETS = {
  cube: { name: '立方体', make: () => make(cubeV.map((v) => v.slice()), [[4, 5, 6, 7], [3, 2, 1, 0], [0, 1, 5, 4], [2, 3, 7, 6], [1, 2, 6, 5], [3, 0, 4, 7]]) },
  plane: {
    name: '平面',
    make: () => {
      const V = [];
      for (let j = 0; j < 3; j++) for (let i = 0; i < 3; i++) V.push([i - 1, j - 1, 0]);
      const F = [];
      for (let j = 0; j < 2; j++) for (let i = 0; i < 2; i++) F.push([j * 3 + i, j * 3 + i + 1, (j + 1) * 3 + i + 1, (j + 1) * 3 + i]);
      return make(V, F);
    },
  },
  pyramid: { name: '四角錐', make: () => make([[-1, -1, -1], [1, -1, -1], [1, 1, -1], [-1, 1, -1], [0, 0, 1.2]], [[3, 2, 1, 0], [0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]]) },
  prism: { name: '三角柱', make: () => make([[-1.1, -0.7, -1], [1.1, -0.7, -1], [0, 1, -1], [-1.1, -0.7, 1], [1.1, -0.7, 1], [0, 1, 1]], [[3, 4, 5], [2, 1, 0], [0, 1, 4, 3], [1, 2, 5, 4], [2, 0, 3, 5]]) },
};
