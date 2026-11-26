import { test } from 'node:test';
import assert from 'node:assert/strict';
import fc from 'fast-check';
import * as M from '../public/tools/gregory-mesh-editor/mesh.js';
import { buildModel } from '../public/tools/gregory-mesh-editor/gregory.js';

const presetKey = fc.constantFrom(...Object.keys(M.PRESETS));
const finiteNum = fc.double({ min: -1000, max: 1000, noNaN: true });
// NaN・無限大・数でない値も含めた、不正な押し出し量
const anyDist = fc.oneof(fc.double(), fc.constant(undefined), fc.constant(null), fc.string(), fc.boolean());

const op = fc.record({
  kind: fc.constantFrom('extrude', 'loop', 'delete', 'crease'),
  pick: fc.array(fc.nat(1000), { minLength: 1, maxLength: 3 }),
  dist: fc.double({ min: -0.6, max: 0.8, noNaN: true }),
});

function apply(model, o, kind = o.kind) {
  const nF = model.F.length;
  const faces = [...new Set(o.pick.map((p) => p % nF))];
  if (kind === 'extrude') return M.extrudeFaces(model, faces, o.dist).model;
  if (kind === 'delete') return nF <= 1 ? model : M.deleteFaces(model, faces.slice(0, nF - 1));
  const keys = [...M.edgeMap(model.F).keys()];
  const picked = [...new Set(o.pick.map((p) => keys[p % keys.length]))];
  return kind === 'loop' ? M.loopCut(model, picked).model : M.toggleCreases(model, picked);
}
const build = (m, N = 4) => buildModel(m.V, m.F, { tau: 1, mode: 'g1', N, creases: m.creases });
const finite = (b) => b.mesh.pos.every((p) => p.every(Number.isFinite)) && b.mesh.nrm.every((p) => p.every(Number.isFinite));

test('任意の編集操作の列のあとも、ケージの位相が正しい', () => {
  fc.assert(fc.property(presetKey, fc.array(op, { maxLength: 8 }), (key, ops) => {
    let m = M.PRESETS[key].make();
    for (const o of ops) {
      m = apply(m, o);
      assert.deepEqual(M.validateModel(m), []);
    }
  }), { numRuns: 150 });
});

test('任意の編集操作の列のあとも、曲面の評価値が有限', () => {
  fc.assert(fc.property(presetKey, fc.array(op, { maxLength: 8 }), (key, ops) => {
    const m = ops.reduce((acc, o) => apply(acc, o), M.PRESETS[key].make());
    assert.ok(finite(build(m)));
  }), { numRuns: 100 });
});

test('ループ分割だけを繰り返しても、辺をまたぐ法線の差は 1° 未満（G1）', () => {
  fc.assert(fc.property(presetKey, fc.array(op, { minLength: 1, maxLength: 5 }), (key, ops) => {
    const m = ops.reduce((acc, o) => apply(acc, o, 'loop'), M.PRESETS[key].make());
    const { check } = build(m);
    assert.ok(check.g1 < 1, `g1=${check.g1}`);
    assert.ok(check.g0 < 1e-9);
  }), { numRuns: 100 });
});

test('折れ目の入れ方によらず、全辺の位置は連続で、滑らかな辺は G1', () => {
  const jitter = fc.array(fc.double({ min: -0.15, max: 0.15, noNaN: true }), { minLength: 30, maxLength: 30 });
  fc.assert(fc.property(presetKey, jitter, fc.array(fc.boolean(), { minLength: 40, maxLength: 40 }), (key, j, flags) => {
    const base = M.PRESETS[key].make();
    const V = base.V.map((v, i) => v.map((x, k) => x + j[(i * 3 + k) % j.length]));
    const keys = [...M.edgeMap(base.F).keys()];
    const creases = new Set(keys.filter((_, i) => flags[i % flags.length]));
    const { check } = build({ V, F: base.F, creases }, 8);
    assert.ok(check.g0 < 1e-9, `g0=${check.g0}`);
    assert.ok(check.pairs === 0 || check.g1 < 0.05, `g1=${check.g1}`);
  }), { numRuns: 100 });
});

test('ほぼ平らな面の間の折れ目でも、滑らかな辺の G1 が崩れない（交線が不安定な場合）', () => {
  const base = M.PRESETS.plane.make();
  base.V[4] = [-0.15, 0, 0];
  base.V[3] = [-1, 0.0012, 0];
  const creases = new Set(['4-5', '4-7']);
  const { check } = build({ V: base.V, F: base.F, creases }, 8);
  assert.ok(check.pairs > 0);
  assert.ok(check.g1 < 0.05, `g1=${check.g1}`);
});

test('面を1枚押し出すと、面の辺の数だけ側面が増え、頂点がその面の頂点数だけ増える', () => {
  fc.assert(fc.property(presetKey, fc.nat(100), fc.double({ min: 0.1, max: 2, noNaN: true }), (key, pick, d) => {
    const m = M.PRESETS[key].make();
    const fi = pick % m.F.length;
    const n = m.F[fi].length;
    const r = M.extrudeFaces(m, [fi], d);
    assert.equal(r.model.F.length, m.F.length + n);
    assert.equal(r.model.V.length, m.V.length + n);
    assert.deepEqual(r.faces, [fi]);
  }));
});

test('立方体の上面を押し出すと、頂点が12、面が10になる', () => {
  const m = M.PRESETS.cube.make();
  const r = M.extrudeFaces(m, [0], 0.5).model;
  assert.equal(r.V.length, 12);
  assert.equal(r.F.length, 10);
});

test('押し出し量が不正・極端でも、位相の正しいケージを返す（量は範囲に丸める）', () => {
  fc.assert(fc.property(presetKey, anyDist, (key, d) => {
    const m = M.PRESETS[key].make();
    const r = M.extrudeFaces(m, [0], d);
    assert.deepEqual(M.validateModel(r.model), []);
    assert.ok(finite(build(r.model)));
  }));
});

const same = (a, b) => JSON.stringify([a.V, a.F, [...a.creases]]) === JSON.stringify([b.V, b.F, [...b.creases]]);

test('有効な面番号が1つもなければ、押し出しはケージを変えない', () => {
  fc.assert(fc.property(presetKey, fc.array(fc.oneof(fc.integer({ min: -100, max: -1 }), fc.integer({ min: 100, max: 1000 }), fc.double({ noInteger: true }), fc.constant(NaN)), { maxLength: 4 }), (key, idx) => {
    const m = M.PRESETS[key].make();
    const r = M.extrudeFaces(m, idx, 0.3);
    assert.deepEqual(r.faces, []);
    assert.ok(same(r.model, m));
  }));
});

test('有効な面番号と無効な番号が混ざっていても、有効な面だけを押し出して位相が正しい', () => {
  fc.assert(fc.property(presetKey, fc.array(fc.oneof(fc.integer(), fc.double(), fc.constant(NaN)), { maxLength: 4 }), (key, idx) => {
    const m = M.PRESETS[key].make();
    const valid = new Set(idx.filter((i) => Number.isInteger(i) && i >= 0 && i < m.F.length));
    const r = M.extrudeFaces(m, idx, 0.3);
    assert.equal(r.faces.length, valid.size);
    assert.deepEqual(M.validateModel(r.model), []);
  }));
});

test('存在しない辺だけを指定したループ分割は、ケージを変えない', () => {
  fc.assert(fc.property(presetKey, fc.array(fc.string(), { maxLength: 3 }), (key, keys) => {
    const m = M.PRESETS[key].make();
    const cut = M.loopCut(m, keys.filter((k) => !M.edgeMap(m.F).has(k)));
    assert.deepEqual(cut.edges, []);
    assert.ok(same(cut.model, m));
  }));
});

test('面を削除すると、使われない頂点が残らず、位相が正しい（すべての面は残す）', () => {
  fc.assert(fc.property(presetKey, fc.array(fc.nat(100), { maxLength: 5 }), (key, picks) => {
    const m = M.PRESETS[key].make();
    if (m.F.length <= 1) return;
    const out = M.deleteFaces(m, picks.map((p) => p % m.F.length).slice(0, m.F.length - 1));
    assert.deepEqual(M.validateModel(out), []);
  }));
});

test('すべての面を削除すると、頂点も面も空になる（画面側では全削除を許さない）', () => {
  fc.assert(fc.property(presetKey, (key) => {
    const m = M.PRESETS[key].make();
    const out = M.deleteFaces(m, m.F.map((_, i) => i));
    assert.deepEqual([out.V.length, out.F.length], [0, 0]);
  }));
});

const modes = fc.constantFrom('v', 'e', 'f');
// 各モードの要素を、ケージの範囲から選ぶ
function pickItems(m, mode, picks) {
  const keys = [...M.edgeMap(m.F).keys()];
  const pool = mode === 'v' ? m.V.map((_, i) => i) : mode === 'e' ? keys : m.F.map((_, i) => i);
  return [...new Set(picks.map((p) => pool[p % pool.length]))];
}
const picksArb = fc.array(fc.nat(1000), { maxLength: 6 });

test('選択を変換すると、新しいモードの要素だけになり、範囲内に収まる', () => {
  fc.assert(fc.property(presetKey, modes, modes, picksArb, (key, from, to, picks) => {
    const m = M.PRESETS[key].make();
    const out = M.convertSelection(m, from, pickItems(m, from, picks), to);
    const edges = M.edgeMap(m.F);
    for (const it of out) {
      if (to === 'e') assert.ok(typeof it === 'string' && edges.has(it), `辺でない要素: ${it}`);
      else if (to === 'v') assert.ok(Number.isInteger(it) && it >= 0 && it < m.V.length);
      else assert.ok(Number.isInteger(it) && it >= 0 && it < m.F.length);
    }
  }));
});

test('面の選択を点に変換すると、選んだ面の頂点の和集合になる', () => {
  fc.assert(fc.property(presetKey, picksArb, (key, picks) => {
    const m = M.PRESETS[key].make();
    const faces = pickItems(m, 'f', picks);
    const want = new Set(faces.flatMap((i) => m.F[i]));
    assert.deepEqual([...M.convertSelection(m, 'f', faces, 'v')].sort((a, b) => a - b), [...want].sort((a, b) => a - b));
  }));
});

test('面の選択を辺や点を経由して面に戻しても、選んだ面は残る', () => {
  fc.assert(fc.property(presetKey, picksArb, (key, picks) => {
    const m = M.PRESETS[key].make();
    const faces = pickItems(m, 'f', picks);
    for (const via of ['v', 'e']) {
      const back = M.convertSelection(m, via, M.convertSelection(m, 'f', faces, via), 'f');
      for (const f of faces) assert.ok(back.has(f), `面${f}が失われた（${via}経由）`);
    }
  }));
});

test('面を1枚選んで辺モードにすると、その面の辺だけが選ばれ、数値は混ざらない（面番号の混入の再発防止）', () => {
  const m = M.PRESETS.cube.make();
  const edges = M.convertSelection(m, 'f', [0], 'e');
  assert.equal(edges.size, 4);
  for (const k of edges) assert.equal(typeof k, 'string');
});

test('面の全頂点を選んで面モードにすると、その面が選ばれる（空にならない）', () => {
  const m = M.PRESETS.cube.make();
  assert.deepEqual([...M.convertSelection(m, 'v', m.F[0], 'f')].includes(0), true);
});

test('不正な選択の要素（範囲外・小数・NaN・型違い・壊れた辺の名前）は、無視して例外を出さない', () => {
  const junk = fc.oneof(fc.integer(), fc.double(), fc.string(), fc.constant(undefined), fc.constant(null), fc.boolean(), fc.constant('1-'), fc.constant('-1-2'), fc.constant('a-b'));
  fc.assert(fc.property(presetKey, modes, modes, fc.array(junk, { maxLength: 6 }), (key, from, to, items) => {
    const m = M.PRESETS[key].make();
    const out = M.convertSelection(m, from, items, to);
    assert.ok(out instanceof Set);
    for (const v of M.selectionVertices(m, from, items)) assert.ok(Number.isInteger(v) && v >= 0 && v < m.V.length);
  }));
});

test('ミラーの対応付けは、組になる頂点どうしで対称で、組は互いに指し合う', () => {
  const pt = fc.tuple(fc.double({ min: 0.001, max: 5, noNaN: true }), finiteNum, finiteNum);
  const distinct = fc.uniqueArray(pt, { minLength: 1, maxLength: 8, selector: ([x, y, z]) => `${Math.round(x * 100)},${Math.round(y * 100)},${Math.round(z * 100)}` });
  fc.assert(fc.property(distinct, (pts) => {
    const V = pts.flatMap(([x, y, z]) => [[x, y, z], [-x, y, z]]);
    const mm = M.mirrorMap(V);
    mm.forEach((j, i) => {
      assert.ok(j >= 0);
      assert.equal(mm[j], i);
      assert.ok(Math.abs(V[i][0] + V[j][0]) < 1e-9);
    });
  }));
});

test('ミラーの相手がない頂点は、-1 になる', () => {
  assert.deepEqual(M.mirrorMap([[1, 0, 0]]), [-1]);
  assert.deepEqual(M.mirrorMap([[0, 1, 2]]), [0]);
  assert.deepEqual(M.mirrorMap([]), []);
});
