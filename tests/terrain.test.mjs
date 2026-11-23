import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createContext, runInContext } from 'node:vm';
import fc from 'fast-check';

// terrain.js はブラウザ用の classic script なので、サンドボックスに読み込んで Terrain を取り出す
const ctx = createContext({});
runInContext(readFileSync(new URL('../public/art/arrow-trail/terrain.js', import.meta.url), 'utf8'), ctx);
const T = ctx.Terrain;

const coord = fc.double({ min: -2000, max: 2000, noNaN: true });
const seed = fc.integer({ min: 0, max: T.SEED_MAX });
// NaN・無限大・巨大な値・数でない値も含めた、不正な入力
const anyNumber = fc.double();
const anySeed = fc.oneof(fc.double(), fc.integer(), fc.string(), fc.constant(undefined), fc.constant(null), fc.boolean());

const MAX_SLOPE = 3;   // 実測の最大は約1.8。余裕を持たせた上限

test('同じシードと座標なら、必ず同じ高さになる', () => {
  fc.assert(fc.property(coord, coord, seed, (x, z, s) => {
    assert.equal(T.height(x, z, s), T.height(x, z, s));
    assert.equal(T.makeHeight(s)(x, z), T.height(x, z, s));
  }));
});

test('高さは、有限で、0以上かつ上限以下', () => {
  fc.assert(fc.property(coord, coord, seed, (x, z, s) => {
    const h = T.height(x, z, s);
    assert.ok(Number.isFinite(h));
    assert.ok(h >= 0 && h <= T.MAX_HEIGHT);
  }));
});

test('近い2点の高さの差は、距離に比例する範囲に収まる（傾斜の上限）', () => {
  fc.assert(fc.property(
    coord, coord, seed, fc.double({ min: 0, max: 2 * Math.PI, noNaN: true }), fc.double({ min: 0.01, max: 3, noNaN: true }),
    (x, z, s, ang, d) => {
      const h = T.makeHeight(s);
      const diff = Math.abs(h(x, z) - h(x + Math.cos(ang) * d, z + Math.sin(ang) * d));
      assert.ok(diff <= MAX_SLOPE * d + 1e-9, `傾斜が上限を超えた: ${diff / d}`);
    }
  ), { numRuns: 2000 });
});

test('不正な入力（NaN・無限大・範囲外・数でない値）でも、例外を出さず、有効な高さを返す', () => {
  fc.assert(fc.property(anyNumber, anyNumber, anySeed, (x, z, s) => {
    const h = T.height(x, z, s);
    assert.ok(Number.isFinite(h));
    assert.ok(h >= 0 && h <= T.MAX_HEIGHT);
  }), { numRuns: 2000 });
});

test('シードは、0〜上限の整数に丸められ、丸めた値をもう一度丸めても変わらない', () => {
  fc.assert(fc.property(anySeed, (s) => {
    const r = T.sanitizeSeed(s);
    assert.ok(Number.isInteger(r) && r >= 0 && r <= T.SEED_MAX);
    assert.equal(T.sanitizeSeed(r), r);
  }), { numRuns: 2000 });
  fc.assert(fc.property(seed, (s) => T.sanitizeSeed(s) === s));
});

test('範囲外のシードは、端の値に丸められる', () => {
  assert.equal(T.sanitizeSeed(-5), 0);
  assert.equal(T.sanitizeSeed(1e9), T.SEED_MAX);
  assert.equal(T.sanitizeSeed(NaN), 0);
  assert.equal(T.sanitizeSeed(Infinity), 0);
  assert.equal(T.sanitizeSeed('12'), 12);
  assert.equal(T.sanitizeSeed(3.9), 3);
});

test('シードが違うと、地形は違う', () => {
  fc.assert(fc.property(seed, seed, (a, b) => {
    fc.pre(a !== b);
    const ha = T.makeHeight(a), hb = T.makeHeight(b);
    let diff = 0;
    for (let x = -150; x <= 150; x += 25) for (let z = -150; z <= 150; z += 25) diff += Math.abs(ha(x, z) - hb(x, z));
    assert.ok(diff > 0);
  }), { numRuns: 200 });
});

// 格子点から離れた座標（整数部 + 0.01〜0.99）。格子の境界では、数値微分が食い違うため
const offLattice = fc.tuple(fc.integer({ min: -50, max: 50 }), fc.double({ min: 0.01, max: 0.99, noNaN: true })).map(([i, f]) => i + f);

test('ノイズの勾配が、数値微分と合う', () => {
  fc.assert(fc.property(offLattice, offLattice, seed, (x, z, s) => {
    const e = 1e-6;
    const [, dx, dz] = T.noised(x, z, s);
    const ndx = (T.noised(x + e, z, s)[0] - T.noised(x - e, z, s)[0]) / (2 * e);
    const ndz = (T.noised(x, z + e, s)[0] - T.noised(x, z - e, s)[0]) / (2 * e);
    assert.ok(Math.abs(dx - ndx) < 1e-3, `dx ${dx} と ${ndx}`);
    assert.ok(Math.abs(dz - ndz) < 1e-3, `dz ${dz} と ${ndz}`);
  }), { numRuns: 1000 });
});

test('いくつかのシードで、高さの分布が、以前の地形に近い（平均7〜17、0以上、上限以下）', () => {
  for (const s of [0, 1, 2, 7, 42, 100, 9999]) {
    const h = T.makeHeight(s);
    let sum = 0, n = 0, min = Infinity, max = -Infinity;
    for (let x = -160; x <= 160; x += 4) for (let z = -160; z <= 160; z += 4) {
      const v = h(x, z); sum += v; n++; min = Math.min(min, v); max = Math.max(max, v);
    }
    const mean = sum / n;
    assert.ok(mean > 7 && mean < 17, `シード ${s} の平均 ${mean}`);
    assert.ok(min >= 0 && max <= T.MAX_HEIGHT, `シード ${s} の範囲 ${min}〜${max}`);
  }
});
