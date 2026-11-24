const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const mul = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const len = (a) => Math.hypot(a[0], a[1], a[2]);
const norm = (a) => { const l = len(a); return l < 1e-12 ? [0, 0, 0] : mul(a, 1 / l); };

// 角で重みが 0/0 になる点は 2 つの候補の平均にする
function blend(wa, A, wb, B) {
  const s = wa + wb;
  if (s < 1e-12) return mul(add(A, B), 0.5);
  return add(mul(A, wa / s), mul(B, wb / s));
}

const bern3 = (u) => { const s = 1 - u; return [s * s * s, 3 * u * s * s, 3 * u * u * s, u * u * u]; };
const FACT = [1, 1, 2, 6, 24];

function bezier3(E, t) {
  const B = bern3(t);
  return [0, 1, 2].map((k) => E[0][k] * B[0] + E[1][k] * B[1] + E[2][k] * B[2] + E[3][k] * B[3]);
}
function bezier3d(E, t) {
  const s = 1 - t;
  const a = mul(sub(E[1], E[0]), 3 * s * s);
  const b = mul(sub(E[2], E[1]), 6 * s * t);
  const c = mul(sub(E[3], E[2]), 3 * t * t);
  return add(add(a, b), c);
}

// 角や斜辺で差分が取れるよう、内側へ少し寄せてから差分する
function normalAt(at, a, b, tri) {
  const e = 2e-3, h = 3e-4;
  const a2 = a * (1 - e) + (tri ? e / 3 : e / 2);
  const b2 = b * (1 - e) + (tri ? e / 3 : e / 2);
  const da = sub(at(a2 + h, b2), at(a2 - h, b2));
  const db = sub(at(a2, b2 + h), at(a2, b2 - h));
  return norm(cross(da, db));
}

function quadPatch(ed) {
  const E = ed.map((e) => e.E);
  const b = [[], [], [], []];
  for (let m = 0; m < 4; m++) {
    b[m][0] = E[0][m];
    b[3][m] = E[1][m];
    b[3 - m][3] = E[2][m];
    b[0][3 - m] = E[3][m];
  }
  const q = ed.map((e) => [null, add(e.E[1], mul(e.c[1], 1 / 3)), add(e.E[2], mul(e.c[2], 1 / 3))]);
  const at = (u, v) => {
    const Bu = bern3(u), Bv = bern3(v);
    const P11 = blend(u, q[0][1], v, q[3][2]);
    const P21 = blend(1 - u, q[0][2], v, q[1][1]);
    const P22 = blend(1 - u, q[2][1], 1 - v, q[1][2]);
    const P12 = blend(u, q[2][2], 1 - v, q[3][1]);
    const r = [0, 0, 0];
    for (let i = 0; i < 4; i++) {
      for (let j = 0; j < 4; j++) {
        let p;
        if (i === 1 && j === 1) p = P11;
        else if (i === 2 && j === 1) p = P21;
        else if (i === 2 && j === 2) p = P22;
        else if (i === 1 && j === 2) p = P12;
        else p = b[i][j];
        const w = Bu[i] * Bv[j];
        r[0] += p[0] * w; r[1] += p[1] * w; r[2] += p[2] * w;
      }
    }
    return r;
  };
  const edgePoint = (k, t, h) => {
    const P = [[t, 0], [1, t], [1 - t, 1], [0, 1 - t]][k];
    const I = [[t, h], [1 - h, t], [1 - t, 1 - h], [h, 1 - t]][k];
    return [at(P[0], P[1]), at(I[0], I[1])];
  };
  return { type: 'q', ed, q, at, normal: (a, c) => normalAt(at, a, c, false), edgePoint };
}

function triPatch(ed) {
  const E = ed.map((e) => e.E);
  const fE = E.map((e) => [
    e[0],
    add(mul(e[0], 0.25), mul(e[1], 0.75)),
    add(mul(e[1], 0.5), mul(e[2], 0.5)),
    add(mul(e[2], 0.75), mul(e[3], 0.25)),
    e[3],
  ]);
  const c = Array.from({ length: 5 }, () => []);
  for (let m = 0; m <= 4; m++) {
    c[4 - m][m] = fE[0][m];
    c[0][4 - m] = fE[1][m];
    c[m][0] = fE[2][m];
  }
  const q = ed.map((e) => [null, add(e.E[1], mul(e.c[1], 0.25)), add(e.E[2], mul(e.c[2], 0.25))]);
  const at = (s, t) => {
    const b0 = 1 - s - t, b1 = s, b2 = t;
    const P0 = blend(b1, q[0][1], b2, q[2][2]);
    const P1 = blend(b0, q[0][2], b2, q[1][1]);
    const P2 = blend(b1, q[1][2], b0, q[2][1]);
    const r = [0, 0, 0];
    for (let i = 0; i <= 4; i++) {
      for (let j = 0; i + j <= 4; j++) {
        const k = 4 - i - j;
        let p;
        if (i === 2 && j === 1) p = P0;
        else if (i === 1 && j === 2) p = P1;
        else if (i === 1 && j === 1) p = P2;
        else p = c[i][j];
        const w = (FACT[4] / (FACT[i] * FACT[j] * FACT[k])) * Math.pow(b0, i) * Math.pow(b1, j) * Math.pow(b2, k);
        r[0] += p[0] * w; r[1] += p[1] * w; r[2] += p[2] * w;
      }
    }
    return r;
  };
  const edgePoint = (k, t, h) => {
    const P = [[t, 0], [1 - t, t], [0, 1 - t]][k];
    const I = [[t * (1 - h), h], [(1 - t) * (1 - h), t * (1 - h)], [h, (1 - t) * (1 - h)]][k];
    return [at(P[0], P[1]), at(I[0], I[1])];
  };
  return { type: 't', ed, q, at, normal: (a, d) => normalAt(at, a, d, true), edgePoint };
}

function tessellate(patches, N) {
  const pos = [], nrm = [], tris = [], triFace = [];
  for (let pi = 0; pi < patches.length; pi++) {
    const p = patches[pi];
    const base = pos.length;
    const t0 = tris.length;
    if (p.type === 'q') {
      for (let j = 0; j <= N; j++) {
        for (let i = 0; i <= N; i++) {
          pos.push(p.at(i / N, j / N));
          nrm.push(p.normal(i / N, j / N));
        }
      }
      for (let j = 0; j < N; j++) {
        for (let i = 0; i < N; i++) {
          const a = base + j * (N + 1) + i;
          tris.push([a, a + 1, a + N + 1], [a + 1, a + N + 2, a + N + 1]);
        }
      }
    } else {
      const rowStart = [];
      let cnt = 0;
      for (let t = 0; t <= N; t++) {
        rowStart[t] = cnt;
        for (let s = 0; s <= N - t; s++) {
          pos.push(p.at(s / N, t / N));
          nrm.push(p.normal(s / N, t / N));
          cnt++;
        }
      }
      for (let t = 0; t < N; t++) {
        for (let s = 0; s < N - t; s++) {
          const a = base + rowStart[t] + s, b = a + 1, c = base + rowStart[t + 1] + s;
          tris.push([a, b, c]);
          if (s < N - t - 1) tris.push([b, base + rowStart[t + 1] + s + 1, c]);
        }
      }
    }
    for (let i = t0; i < tris.length; i++) triFace.push(pi);
  }
  return { pos, nrm, tris, triFace };
}

function buildModel(V, F, opts) {
  const { tau, mode, N } = opts;
  const creases = opts.creases || new Set();
  const ekey = (a, b) => Math.min(a, b) + '-' + Math.max(a, b);

  const edgeMap = new Map();
  F.forEach((f, fi) => {
    const m = f.length;
    for (let k = 0; k < m; k++) {
      const x = f[k], y = f[(k + 1) % m];
      const key = ekey(x, y);
      if (!edgeMap.has(key)) edgeMap.set(key, { a: Math.min(x, y), b: Math.max(x, y), faces: [] });
      edgeMap.get(key).faces.push({ fi, k });
    }
  });

  const cornerWeighted = (fi, k) => {
    const f = F[fi], m = f.length;
    const p = V[f[k]], ea = sub(V[f[(k + 1) % m]], p), eb = sub(V[f[(k + m - 1) % m]], p);
    const theta = Math.atan2(len(cross(ea, eb)), dot(ea, eb));
    return mul(norm(cross(ea, eb)), theta);
  };
  // 頂点の全周の法線。可視判定の目安にだけ使う
  const nrmV = V.map(() => [0, 0, 0]);
  F.forEach((f, fi) => f.forEach((v, k) => { nrmV[v] = add(nrmV[v], cornerWeighted(fi, k)); }));
  const nv = nrmV.map(norm);

  // 頂点ごとに、折れ目でない辺でつながる面の集まり（滑らかな群）ごとの法線を持つ
  const corner = F.map((f) => f.map(() => null));
  const incident = V.map(() => []);
  F.forEach((f, fi) => f.forEach((v, k) => incident[v].push({ fi, k })));
  incident.forEach((list, v) => {
    const parent = list.map((_, i) => i);
    const find = (i) => (parent[i] === i ? i : (parent[i] = find(parent[i])));
    const nbrs = list.map(({ fi, k }) => { const f = F[fi], m = f.length; return [f[(k + 1) % m], f[(k + m - 1) % m]]; });
    for (let i = 0; i < list.length; i++) {
      for (let j = i + 1; j < list.length; j++) {
        const shared = nbrs[i].some((w) => nbrs[j].includes(w) && !creases.has(ekey(v, w)));
        if (shared) parent[find(i)] = find(j);
      }
    }
    const sums = new Map();
    list.forEach(({ fi, k }, i) => {
      const r = find(i);
      sums.set(r, add(sums.get(r) || [0, 0, 0], cornerWeighted(fi, k)));
    });
    // 接平面へ射影したとき、どの面の角も裏返らない法線を選ぶ（角度重み、重みなし、各面の法線の順）
    const flat = (fi, k) => { const f = F[fi], m = f.length, p = V[f[k]]; return norm(cross(sub(V[f[(k + 1) % m]], p), sub(V[f[(k + m - 1) % m]], p))); };
    const folded = (n, members) => members.some(({ fi, k }) => {
      const f = F[fi], m = f.length, p = V[f[k]];
      const e1 = sub(V[f[(k + 1) % m]], p), e2 = sub(V[f[(k + m - 1) % m]], p);
      const p1 = sub(e1, mul(n, dot(e1, n))), p2 = sub(e2, mul(n, dot(e2, n)));
      return dot(cross(p1, p2), n) <= 1e-6 * len(p1) * len(p2);
    });
    const chosen = new Map();
    sums.forEach((sum, r) => {
      const members = list.filter((_, i) => find(i) === r);
      const unweighted = norm(members.reduce((a, { fi, k }) => add(a, flat(fi, k)), [0, 0, 0]));
      const candidates = [norm(sum), unweighted, ...members.map(({ fi, k }) => flat(fi, k))];
      chosen.set(r, candidates.find((n) => len(n) > 0.5 && !folded(n, members)) || candidates[0]);
    });
    list.forEach(({ fi, k }, i) => { corner[fi][k] = chosen.get(find(i)); });
  });
  const cornerAt = (fi, v) => corner[fi][F[fi].indexOf(v)];

  const tangent = (x, y) => {
    const d = sub(V[y], V[x]);
    const L = len(d);
    const fs = edgeMap.get(ekey(x, y)).faces;
    const n1 = cornerAt(fs[0].fi, x);
    if (creases.has(ekey(x, y)) && fs.length === 2) {
      const n2 = cornerAt(fs[1].fi, x);
      const line = cross(n1, n2);
      if (len(line) > 1e-3) {
        const dir = norm(line);
        return mul(dot(dir, d) < 0 ? mul(dir, -1) : dir, L * tau);
      }
    }
    const pr = sub(d, mul(n1, dot(d, n1)));
    const pl = len(pr);
    return pl < 1e-9 ? mul(d, tau) : mul(pr, (L * tau) / pl);
  };
  const cache = new Map();
  const curve = (x, y) => {
    const key = x + ',' + y;
    if (cache.has(key)) return cache.get(key);
    const X = V[x], Y = V[y];
    const E = [X, add(X, mul(tangent(x, y), 1 / 3)), add(Y, mul(tangent(y, x), 1 / 3)), Y];
    cache.set(key, E);
    cache.set(y + ',' + x, [E[3], E[2], E[1], E[0]]);
    return E;
  };

  const edgeData = (f, fi, k) => {
    const m = f.length;
    const nx = corner[fi][k], ny = corner[fi][(k + 1) % m];
    const x = f[k], y = f[(k + 1) % m], w = f[(k + m - 1) % m], z = f[(k + 2) % m];
    const E = curve(x, y);
    const Tx = mul(sub(E[1], E[0]), 3), Ty = mul(sub(E[3], E[2]), 3);
    const ax = mul(sub(curve(x, w)[1], V[x]), 3);
    const ay = mul(sub(curve(y, z)[1], V[y]), 3);
    let D;
    if (mode === 'g1') {
      const C0 = cross(nx, Tx), C1 = cross(ny, Ty);
      const inv = (x) => (x > 1e-18 ? 1 / x : 0);
      const a0 = dot(ax, C0) * inv(dot(C0, C0)), b0 = dot(ax, Tx) * inv(dot(Tx, Tx));
      const a1 = dot(ay, C1) * inv(dot(C1, C1)), b1 = dot(ay, Ty) * inv(dot(Ty, Ty));
      D = (t) => {
        const s = 1 - t;
        const C = add(mul(C0, s), mul(C1, t));
        const T = bezier3d(E, t);
        return add(mul(C, s * a0 + t * a1), mul(T, s * b0 + t * b1));
      };
    } else {
      D = (t) => add(mul(ax, 1 - t), mul(ay, t));
    }
    const c0 = D(0), c3 = D(1), d1 = D(1 / 3), d2 = D(2 / 3);
    const A = sub(mul(d1, 27), add(mul(c0, 8), c3));
    const B = sub(mul(d2, 27), add(c0, mul(c3, 8)));
    const c1 = mul(sub(mul(A, 2), B), 1 / 18);
    const c2 = mul(sub(mul(B, 2), A), 1 / 18);
    return { E, c: [c0, c1, c2, c3] };
  };

  const patches = F.map((f, fi) => {
    const ed = f.map((_, k) => edgeData(f, fi, k));
    return f.length === 4 ? quadPatch(ed) : triPatch(ed);
  });

  const edges = [...edgeMap.values()].map((e) => ({ ...e, E: curve(e.a, e.b), crease: creases.has(ekey(e.a, e.b)) }));

  const sampleEdge = (face, t) => {
    const p = patches[face.fi];
    const h = 1e-6;
    const [p0, p1] = p.edgePoint(face.k, t, h);
    const T = bezier3d(p.ed[face.k].E, t);
    return { p: p0, n: norm(cross(T, mul(sub(p1, p0), 1 / h))) };
  };
  let g0 = 0, g1 = 0, pairs = 0, creased = 0;
  const samples = [];
  for (const e of edges) {
    if (e.faces.length !== 2) continue;
    if (e.crease) creased++; else pairs++;
    for (let i = 1; i <= 9; i++) {
      const s = i / 10;
      const A = sampleEdge(e.faces[0], F[e.faces[0].fi][e.faces[0].k] === e.a ? s : 1 - s);
      const B = sampleEdge(e.faces[1], F[e.faces[1].fi][e.faces[1].k] === e.a ? s : 1 - s);
      g0 = Math.max(g0, len(sub(A.p, B.p)));
      if (e.crease) continue;
      const ang = Math.acos(Math.max(-1, Math.min(1, dot(A.n, B.n)))) * (180 / Math.PI);
      g1 = Math.max(g1, ang);
      if (i === 3 || i === 5 || i === 7) samples.push({ p: A.p, na: A.n, nb: B.n });
    }
  }

  const mesh = tessellate(patches, N);
  return {
    nrm: nv, patches, edges, mesh,
    check: { g0, g1, pairs, creased, samples },
    count: { q: F.filter((f) => f.length === 4).length, t: F.filter((f) => f.length === 3).length },
  };
}

export { buildModel, bezier3, add, sub, mul, dot, cross, len, norm };
