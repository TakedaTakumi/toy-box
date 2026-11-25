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

test('範囲外・空・重複の面番号や、存在しない辺は、無視して元のケージのままにする', () => {
  fc.assert(fc.property(presetKey, fc.array(fc.oneof(fc.integer(), fc.double(), fc.constant(NaN)), { maxLength: 4 }), fc.array(fc.string(), { maxLength: 3 }), (key, idx, keys) => {
    const m = M.PRESETS[key].make();
    const valid = (i) => Number.isInteger(i) && i >= 0 && i < m.F.length;
    const r = M.extrudeFaces(m, idx, 0.3);
    assert.equal(r.faces.length, new Set(idx.filter(valid)).size);
    assert.deepEqual(M.validateModel(r.model), []);
    const cut = M.loopCut(m, keys.filter((k) => !M.edgeMap(m.F).has(k)));
    assert.deepEqual(cut.edges, []);
    assert.equal(cut.model.F.length, m.F.length);
  }));
});

test('すべての面を削除する指定でも、使われない頂点は残らない', () => {
  fc.assert(fc.property(presetKey, fc.array(fc.nat(100), { maxLength: 5 }), (key, picks) => {
    const m = M.PRESETS[key].make();
    if (m.F.length <= 1) return;
    const out = M.deleteFaces(m, picks.map((p) => p % m.F.length).slice(0, m.F.length - 1));
    assert.deepEqual(M.validateModel(out), []);
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
