/* 地形の高さ関数。Inigo Quilez の "morenoise" 方式:
   勾配付きバリューノイズ(quintic 補間)を、オクターブごとに回転しながら足し、
   勾配が大きい所ほど振幅を抑える(a += amp * n.x / (1 + dot(d, d)))。 */
(function (root) {
  'use strict';

  var SEED_MAX = 9999;
  var OCTAVES = 4;
  var WAVELENGTH = 110;       // 最も粗いオクターブの波長
  var HEIGHT_BASE = 12;
  var HEIGHT_AMP = 14;
  var MAX_HEIGHT = 45;        // 飛行の高さの上限(85)より十分低く保つ
  var COORD_LIMIT = 1e6;

  // 0〜SEED_MAX の整数に丸める。数でない・有限でない値は 0 にする
  function sanitizeSeed(seed) {
    var n = typeof seed === 'number' ? seed : Number(seed);
    if (!isFinite(n)) return 0;
    n = Math.trunc(n);
    return n < 0 ? 0 : n > SEED_MAX ? SEED_MAX : n;
  }

  function sanitizeCoord(v) {
    if (typeof v !== 'number' || !isFinite(v)) return 0;
    return v < -COORD_LIMIT ? -COORD_LIMIT : v > COORD_LIMIT ? COORD_LIMIT : v;
  }

  // 格子点の値(-1〜1)。整数ハッシュなので、同じ入力なら必ず同じ値になる
  function hash(ix, iz, seed) {
    var h = Math.imul(ix, 0x27d4eb2d) ^ Math.imul(iz, 0x165667b1) ^ Math.imul(seed + 1, 0x9e3779b1);
    h = Math.imul(h ^ (h >>> 15), 0x85ebca6b);
    h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35);
    h ^= h >>> 16;
    return (h >>> 0) / 2147483648 - 1;
  }

  // 値と勾配(x, z の偏微分)を返す。[value, dvalue/dx, dvalue/dz]
  function noised(x, z, seed) {
    var ix = Math.floor(x), iz = Math.floor(z);
    var fx = x - ix, fz = z - iz;
    var ux = fx * fx * fx * (fx * (fx * 6 - 15) + 10), uz = fz * fz * fz * (fz * (fz * 6 - 15) + 10);
    var dux = 30 * fx * fx * (fx * (fx - 2) + 1), duz = 30 * fz * fz * (fz * (fz - 2) + 1);
    var a = hash(ix, iz, seed), b = hash(ix + 1, iz, seed);
    var c = hash(ix, iz + 1, seed), d = hash(ix + 1, iz + 1, seed);
    var k = a - b - c + d;
    return [
      a + (b - a) * ux + (c - a) * uz + k * ux * uz,
      dux * ((b - a) + k * uz),
      duz * ((c - a) + k * ux)
    ];
  }

  function morenoise(x, z, seed) {
    var px = x / WAVELENGTH, pz = z / WAVELENGTH;
    var a = 0, amp = 1, dx = 0, dz = 0;
    for (var i = 0; i < OCTAVES; i++) {
      var n = noised(px, pz, seed);
      dx += n[1]; dz += n[2];
      a += amp * n[0] / (1 + dx * dx + dz * dz);
      amp *= 0.5;
      var nx = 2 * (0.8 * px + 0.6 * pz), nz = 2 * (-0.6 * px + 0.8 * pz);   // 回転しながら2倍の細かさにする
      px = nx; pz = nz;
    }
    return a;
  }

  // 丸めたシードを固定した高さ関数を返す。呼び出しの多い物理計算で使う
  function makeHeight(seed) {
    var s = sanitizeSeed(seed);
    return function (x, z) {
      var h = HEIGHT_BASE + HEIGHT_AMP * morenoise(sanitizeCoord(x), sanitizeCoord(z), s);
      return h < 0 ? 0 : h > MAX_HEIGHT ? MAX_HEIGHT : h;
    };
  }

  function height(x, z, seed) {
    return makeHeight(seed)(x, z);
  }

  root.Terrain = {
    SEED_MAX: SEED_MAX,
    MAX_HEIGHT: MAX_HEIGHT,
    sanitizeSeed: sanitizeSeed,
    makeHeight: makeHeight,
    height: height,
    noised: noised
  };
})(typeof window !== 'undefined' ? window : globalThis);
