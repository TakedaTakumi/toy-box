import { add, sub, mul, dot, cross, norm, bezier3, buildModel } from './gregory.js';
import * as M from './mesh.js';

const $ = (id) => document.getElementById(id);
const cv = $('cv');
const ctx = cv.getContext('2d');
const MODE_LABEL = { v: '点', e: '辺', f: '面' };
const AXES = { x: [1, 0, 0], y: [0, 1, 0], z: [0, 0, 1] };

const state = {
  model: M.PRESETS.cube.make(),
  mode: 'v', sel: new Set(), primary: new Set(),
  mirror: false, axis: 'free', tau: 1, N: 8, extrude: 0.4,
  show: { surface: true, curves: true, cage: true, ctrl: false },
  yaw: 0.75, pitch: 0.45, dist: 8.6,
};
const undoStack = [], redoStack = [];
let built = null, dirty = true, raf = 0, W = 0, H = 0, dpr = 1;
let view = null, vscreen = [], escreen = [], drag = null, toastTimer = 0;

const invalidate = () => { dirty = true; schedule(); };
const schedule = () => { if (!raf) raf = requestAnimationFrame(draw); };

function toast(msg) {
  $('toast').textContent = msg;
  $('toast').hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { $('toast').hidden = true; }, 2600);
}

function commit(next, sel, mode) {
  undoStack.push(M.cloneModel(state.model));
  if (undoStack.length > 100) undoStack.shift();
  redoStack.length = 0;
  state.model = next;
  if (mode) state.mode = mode;
  state.sel = new Set(sel || []);
  state.primary = new Set(state.sel);
  syncUi();
  invalidate();
}
function undo() {
  if (!undoStack.length) return;
  redoStack.push(M.cloneModel(state.model));
  state.model = undoStack.pop();
  state.sel = new Set(); state.primary = new Set();
  syncUi(); invalidate();
}
function redo() {
  if (!redoStack.length) return;
  undoStack.push(M.cloneModel(state.model));
  state.model = redoStack.pop();
  state.sel = new Set(); state.primary = new Set();
  syncUi(); invalidate();
}

// ---- 選択 ----
const vertsOf = (items, mode = state.mode, model = state.model) => {
  const out = new Set();
  for (const it of items) {
    if (mode === 'v') out.add(it);
    else if (mode === 'e') it.split('-').forEach((x) => out.add(Number(x)));
    else model.F[it].forEach((v) => out.add(v));
  }
  return out;
};

function mirrorItem(item, mm) {
  const { model, mode } = state;
  if (mode === 'v') return mm[item] >= 0 ? mm[item] : null;
  if (mode === 'e') {
    const [a, b] = item.split('-').map(Number);
    if (mm[a] < 0 || mm[b] < 0) return null;
    const k = M.ekey(mm[a], mm[b]);
    return M.edgeMap(model.F).has(k) ? k : null;
  }
  const want = model.F[item].map((v) => mm[v]);
  if (want.some((v) => v < 0)) return null;
  const key = [...want].sort((p, q) => p - q).join(',');
  const idx = model.F.findIndex((f) => [...f].sort((p, q) => p - q).join(',') === key);
  return idx >= 0 ? idx : null;
}
const mirrorSet = (items) => {
  const mm = M.mirrorMap(state.model.V);
  const out = new Set();
  for (const it of items) { const m = mirrorItem(it, mm); if (m !== null) out.add(m); }
  return out;
};

function select(item, additive) {
  if (!additive) { state.sel = new Set(); state.primary = new Set(); }
  const group = state.mirror ? new Set([item, ...mirrorSet([item])]) : new Set([item]);
  if (additive && state.sel.has(item)) {
    group.forEach((g) => { state.sel.delete(g); state.primary.delete(g); });
  } else {
    group.forEach((g) => state.sel.add(g));
    state.primary.add(item);
  }
  syncUi(); schedule();
}

function setMode(next) {
  if (next === state.mode) return;
  const verts = vertsOf(state.sel);
  const { F } = state.model;
  const sel = new Set();
  if (next === 'v') verts.forEach((v) => sel.add(v));
  else if (next === 'e') for (const [k, e] of M.edgeMap(F)) if (verts.has(e.a) && verts.has(e.b)) sel.add(k);
  else F.forEach((f, i) => { if (f.every((v) => verts.has(v))) sel.add(i); });
  state.mode = next;
  state.sel = sel; state.primary = new Set(sel);
  syncUi(); schedule();
}

function selectAll() {
  const { F } = state.model;
  state.sel = new Set(state.mode === 'v' ? state.model.V.map((_, i) => i) : state.mode === 'e' ? [...M.edgeMap(F).keys()] : F.map((_, i) => i));
  state.primary = new Set(state.sel);
  syncUi(); schedule();
}
function clearSel() { state.sel = new Set(); state.primary = new Set(); syncUi(); schedule(); }

// ---- 編集操作 ----
function facesOfSelection() {
  const { F } = state.model;
  if (state.mode === 'f') return [...state.sel];
  const verts = vertsOf(state.sel);
  if (state.mode === 'v') return F.map((f, i) => (f.some((v) => verts.has(v)) ? i : -1)).filter((i) => i >= 0);
  const edges = M.edgeMap(F);
  const out = new Set();
  for (const k of state.sel) edges.get(k)?.faces.forEach((i) => out.add(i));
  return [...out];
}

function opExtrude() {
  if (state.mode !== 'f' || !state.sel.size) { toast('押し出しは、面モードで面を選んでから使います'); return; }
  const r = M.extrudeFaces(state.model, [...state.sel], state.extrude);
  commit(r.model, r.faces, 'f');
}
function opLoopCut() {
  if (state.mode !== 'e' || !state.sel.size) { toast('ループ分割は、辺モードで辺を選んでから使います'); return; }
  const mids = new Map();
  if (built) for (const e of built.edges) mids.set(M.ekey(e.a, e.b), bezier3(e.E, 0.5));
  const r = M.loopCut(state.model, [...state.sel], mids);
  if (!r.edges.length) return;
  commit(r.model, r.edges, 'e');
}
function opDelete() {
  const faces = facesOfSelection();
  if (!state.sel.size || !faces.length) { toast('削除する要素を選んでください'); return; }
  if (faces.length >= state.model.F.length) { toast('すべての面は削除できません'); return; }
  commit(M.deleteFaces(state.model, faces), [], null);
}
function opCrease() {
  if (state.mode !== 'e' || !state.sel.size) { toast('折れ目の切り替えは、辺モードで辺を選んでから使います'); return; }
  commit(M.toggleCreases(state.model, [...state.sel]), state.sel, null);
}
function newShape(key) {
  commit(M.PRESETS[key].make(), [], 'v');
}

// ---- カメラ ----
function camera() {
  const cp = Math.cos(state.pitch), sp = Math.sin(state.pitch), cy = Math.cos(state.yaw), sy = Math.sin(state.yaw);
  const eye = [state.dist * cp * cy, state.dist * cp * sy, state.dist * sp];
  const f = norm(mul(eye, -1));
  const r = norm(cross(f, [0, 0, 1]));
  return { eye, f, r, u: cross(r, f), F: Math.min(W, H) * 1.55 };
}
function project(cam, p) {
  const d = sub(p, cam.eye);
  const z = dot(d, cam.f);
  const k = cam.F / z;
  return [W / 2 + dot(d, cam.r) * k, H / 2 - dot(d, cam.u) * k, z];
}
const facing = (cam, p, n) => dot(n, sub(cam.eye, p)) > 0;
const isDark = () => {
  const t = document.documentElement.getAttribute('data-theme');
  return t ? t === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
};
const hexToRgb = (h) => { const s = h.replace('#', ''); return [0, 2, 4].map((i) => parseInt(s.slice(i, i + 2), 16)); };

// ---- 描画 ----
function draw() {
  raf = 0;
  if (!W) return;
  if (dirty) {
    built = buildModel(state.model.V, state.model.F, { tau: state.tau, mode: 'g1', N: state.N, creases: state.model.creases });
    dirty = false;
    updateReadouts();
  }
  const css = getComputedStyle(document.documentElement);
  const tok = (n) => css.getPropertyValue(n).trim();
  const base = isDark() ? [126, 170, 198] : [86, 132, 162];
  const warm = hexToRgb(tok('--warm'));
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = tok('--stage');
  ctx.fillRect(0, 0, W, H);
  const cam = camera();
  const { pos, nrm, tris, triFace } = built.mesh;

  // 座標軸とミラー面
  const line = (a, b, color, w) => {
    const p = project(cam, a), q = project(cam, b);
    ctx.strokeStyle = color; ctx.lineWidth = w;
    ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(q[0], q[1]); ctx.stroke();
  };
  if (state.mirror) {
    const R = 2.6, c = [[0, -R, -R], [0, R, -R], [0, R, R], [0, -R, R]].map((p) => project(cam, p));
    ctx.fillStyle = tok('--plane'); ctx.beginPath();
    c.forEach((p, i) => (i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1])));
    ctx.closePath(); ctx.fill();
  }
  line([-2.8, 0, 0], [2.8, 0, 0], tok('--ax-x'), 1.2);
  line([0, -2.8, 0], [0, 2.8, 0], tok('--ax-y'), 1.2);
  line([0, 0, -2.8], [0, 0, 2.8], tok('--ax-z'), 1.2);

  const n = pos.length;
  const sx = new Float32Array(n), sy = new Float32Array(n), sz = new Float32Array(n);
  const nz = new Float32Array(n), shade = new Float32Array(n), spec = new Float32Array(n);
  const Lv = norm([-0.4, 0.6, -0.7]);
  const Hv = norm([Lv[0], Lv[1], Lv[2] - 1]);
  for (let i = 0; i < n; i++) {
    const p = project(cam, pos[i]);
    sx[i] = p[0]; sy[i] = p[1]; sz[i] = p[2];
    const nv = [dot(nrm[i], cam.r), dot(nrm[i], cam.u), dot(nrm[i], cam.f)];
    nz[i] = nv[2];
    shade[i] = 0.3 + 0.1 * nv[1] + 0.68 * Math.max(0, dot(nv, Lv));
    spec[i] = 0.22 * Math.pow(Math.max(0, dot(nv, Hv)), 28);
  }
  view = { sx, sy, sz, nz, tris, triFace };
  const faceSel = state.mode === 'f' ? state.sel : new Set();

  if (state.show.surface) {
    const order = [];
    for (let t = 0; t < tris.length; t++) {
      const [a, b, c] = tris[t];
      if ((nz[a] + nz[b] + nz[c]) / 3 > 0.06) continue;
      if (!Number.isFinite(sx[a] + sx[b] + sx[c] + sy[a] + sy[b] + sy[c])) continue;
      order.push([t, sz[a] + sz[b] + sz[c]]);
    }
    order.sort((p, q) => q[1] - p[1]);
    ctx.lineWidth = 0.8; ctx.lineJoin = 'round';
    for (const [t] of order) {
      const [a, b, c] = tris[t];
      const s = (shade[a] + shade[b] + shade[c]) / 3, sp = (spec[a] + spec[b] + spec[c]) / 3;
      let col = [base[0] * s + 255 * sp, base[1] * s + 255 * sp, base[2] * s + 255 * sp];
      if (faceSel.has(triFace[t])) col = col.map((x, i) => x * 0.45 + warm[i] * 0.55);
      const css2 = `rgb(${Math.min(255, col[0]) | 0},${Math.min(255, col[1]) | 0},${Math.min(255, col[2]) | 0})`;
      ctx.fillStyle = css2; ctx.strokeStyle = css2;
      ctx.beginPath(); ctx.moveTo(sx[a], sy[a]); ctx.lineTo(sx[b], sy[b]); ctx.lineTo(sx[c], sy[c]); ctx.closePath();
      ctx.fill(); ctx.stroke();
    }
  }

  const { V } = state.model;
  const vn = built.nrm;
  const edgeFacing = (e, t, p) => facing(cam, p, norm(add(mul(vn[e.a], 1 - t), mul(vn[e.b], t))));

  if (state.show.cage) {
    ctx.strokeStyle = tok('--muted'); ctx.lineWidth = 1; ctx.setLineDash([4, 4]);
    for (const e of built.edges) {
      const A = V[e.a], B = V[e.b];
      if (!edgeFacing(e, 0.5, mul(add(A, B), 0.5))) continue;
      const pa = project(cam, A), pb = project(cam, B);
      ctx.beginPath(); ctx.moveTo(pa[0], pa[1]); ctx.lineTo(pb[0], pb[1]); ctx.stroke();
    }
    ctx.setLineDash([]);
  }

  escreen = built.edges.map((e) => {
    const runs = [];
    let run = null;
    for (let i = 0; i <= 24; i++) {
      const t = i / 24, p = bezier3(e.E, t);
      if (!edgeFacing(e, t, p)) { run = null; continue; }
      if (!run) { run = []; runs.push(run); }
      run.push(project(cam, p));
    }
    return { e, key: M.ekey(e.a, e.b), runs };
  });
  if (state.show.curves) {
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    for (const { e, key, runs } of escreen) {
      const picked = state.mode === 'e' && state.sel.has(key);
      ctx.strokeStyle = picked ? tok('--warm') : e.crease ? tok('--crease') : tok('--curve');
      ctx.lineWidth = picked ? 3.4 : e.crease ? 2.4 : 1.6;
      for (const run of runs) {
        ctx.beginPath();
        run.forEach((s, i) => (i ? ctx.lineTo(s[0], s[1]) : ctx.moveTo(s[0], s[1])));
        ctx.stroke();
      }
    }
  } else if (state.mode === 'e') {
    ctx.strokeStyle = tok('--warm'); ctx.lineWidth = 3.4;
    for (const { key, runs } of escreen) {
      if (!state.sel.has(key)) continue;
      for (const run of runs) { ctx.beginPath(); run.forEach((s, i) => (i ? ctx.lineTo(s[0], s[1]) : ctx.moveTo(s[0], s[1]))); ctx.stroke(); }
    }
  }

  if (state.show.ctrl) {
    const accent = tok('--accent'), curve = tok('--curve');
    ctx.lineWidth = 1;
    for (const e of built.edges) {
      if (!edgeFacing(e, 0.5, bezier3(e.E, 0.5))) continue;
      ctx.strokeStyle = curve; ctx.fillStyle = curve;
      for (const [i, j] of [[0, 1], [3, 2]]) {
        const a = project(cam, e.E[i]), b = project(cam, e.E[j]);
        ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke();
        ctx.fillRect(b[0] - 2.5, b[1] - 2.5, 5, 5);
      }
    }
    state.model.F.forEach((f, fi) => {
      const cen = mul(f.reduce((s, i) => add(s, V[i]), [0, 0, 0]), 1 / f.length);
      const fn = norm(f.reduce((s, i) => add(s, vn[i]), [0, 0, 0]));
      if (!facing(cam, cen, fn)) return;
      const patch = built.patches[fi];
      ctx.strokeStyle = accent; ctx.fillStyle = accent;
      patch.ed.forEach((ed, k) => {
        for (const m of [1, 2]) {
          const a = project(cam, ed.E[m]), b = project(cam, patch.q[k][m]);
          ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke();
          ctx.beginPath(); ctx.moveTo(b[0], b[1] - 4); ctx.lineTo(b[0] + 4, b[1]); ctx.lineTo(b[0], b[1] + 4); ctx.lineTo(b[0] - 4, b[1]); ctx.closePath(); ctx.fill();
        }
      });
    });
  }

  vscreen = V.map((p, i) => {
    if (!facing(cam, p, vn[i])) return null;
    const s = project(cam, p);
    const picked = state.mode === 'v' && state.sel.has(i);
    const r = picked ? 7 : state.mode === 'v' ? 5 : 3;
    ctx.beginPath(); ctx.arc(s[0], s[1], r, 0, Math.PI * 2);
    ctx.fillStyle = picked ? tok('--warm') : tok('--curve');
    ctx.fill();
    ctx.lineWidth = 2; ctx.strokeStyle = tok('--stage'); ctx.stroke();
    return s;
  });
}

function updateReadouts() {
  const c = built.check;
  const m = state.model;
  $('mCount').textContent = `${m.V.length} / ${built.edges.length} / ${m.F.length}`;
  $('mKinds').textContent = `四角形 ${built.count.q} / 三角形 ${built.count.t}`;
  $('mEdges').textContent = `滑らか ${c.pairs} / 折れ目 ${c.creased}`;
  $('mG0').textContent = c.g0 < 1e-9 ? '< 1e-9' : c.g0.toExponential(1);
  const ok = c.g1 < 0.5;
  $('mG1').innerHTML = c.pairs === 0 ? '対象の辺なし' : `${c.g1.toFixed(3)}°<span class="pill ${ok ? 'ok' : 'bad'}">${ok ? 'G1 成立' : 'G1 不成立'}</span>`;
  $('g1warn').hidden = c.pairs === 0 || ok;
}

function syncUi() {
  document.querySelectorAll('#modes button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.mode === state.mode)));
  document.querySelectorAll('#axes button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.axis === state.axis)));
  $('undo').disabled = !undoStack.length;
  $('redo').disabled = !redoStack.length;
  const n = state.sel.size;
  let text = `${MODE_LABEL[state.mode]} ${n} 個を選択中`;
  if (state.mode === 'v' && n === 1) text += `　(${state.model.V[[...state.sel][0]].map((x) => x.toFixed(2)).join(', ')})`;
  $('selInfo').textContent = n ? text : `${MODE_LABEL[state.mode]}モード（未選択）`;
}

// ---- ピック ----
function pickVertex(x, y) {
  let best = null, bd = 13;
  vscreen.forEach((s, i) => { if (!s) return; const d = Math.hypot(s[0] - x, s[1] - y); if (d < bd) { bd = d; best = i; } });
  return best;
}
function pickEdge(x, y) {
  let best = null, bd = 10;
  for (const { key, runs } of escreen) {
    for (const run of runs) {
      for (let i = 1; i < run.length; i++) {
        const [ax, ay] = run[i - 1], [bx, by] = run[i];
        const l2 = (bx - ax) ** 2 + (by - ay) ** 2 || 1;
        const t = Math.max(0, Math.min(1, ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / l2));
        const d = Math.hypot(x - (ax + t * (bx - ax)), y - (ay + t * (by - ay)));
        if (d < bd) { bd = d; best = key; }
      }
    }
  }
  return best;
}
function pickFace(x, y) {
  if (!view) return null;
  const { sx, sy, sz, nz, tris, triFace } = view;
  let best = null, bz = Infinity;
  for (let t = 0; t < tris.length; t++) {
    const [a, b, c] = tris[t];
    if ((nz[a] + nz[b] + nz[c]) / 3 > 0.06) continue;
    const d = (sy[b] - sy[c]) * (sx[a] - sx[c]) + (sx[c] - sx[b]) * (sy[a] - sy[c]);
    if (Math.abs(d) < 1e-9) continue;
    const l1 = ((sy[b] - sy[c]) * (x - sx[c]) + (sx[c] - sx[b]) * (y - sy[c])) / d;
    const l2 = ((sy[c] - sy[a]) * (x - sx[c]) + (sx[a] - sx[c]) * (y - sy[c])) / d;
    if (l1 < 0 || l2 < 0 || l1 + l2 > 1) continue;
    const z = sz[a] * l1 + sz[b] * l2 + sz[c] * (1 - l1 - l2);
    if (z < bz) { bz = z; best = triFace[t]; }
  }
  return best;
}
const pickItem = (x, y) => (state.mode === 'v' ? pickVertex(x, y) : state.mode === 'e' ? pickEdge(x, y) : pickFace(x, y));

// ---- 移動 ----
function beginMove() {
  const model = state.model;
  const P = vertsOf(state.primary);
  const mm = state.mirror ? M.mirrorMap(model.V) : null;
  const drivers = [...P].filter((v) => !(mm && mm[v] >= 0 && mm[v] !== v && P.has(mm[v]) && mm[v] < v));
  const start = model.V.map((v) => v.slice());
  const center = mul(drivers.reduce((s, v) => add(s, start[v]), [0, 0, 0]), 1 / Math.max(1, drivers.length));
  return { kind: 'move', drivers, mm, start, center, before: M.cloneModel(model), total: [0, 0], moved: 0 };
}
function applyMove(d0) {
  const cam = camera();
  const [dx, dy] = d0.total;
  const z = dot(sub(d0.center, cam.eye), cam.f);
  const k = z / cam.F;
  let d = add(mul(cam.r, dx * k), mul(cam.u, -dy * k));
  if (state.axis !== 'free') {
    const ax = AXES[state.axis];
    const p0 = project(cam, d0.center), p1 = project(cam, add(d0.center, ax));
    const sv = [p1[0] - p0[0], p1[1] - p0[1]];
    const l2 = sv[0] ** 2 + sv[1] ** 2;
    d = mul(ax, l2 < 1e-6 ? 0 : (dx * sv[0] + dy * sv[1]) / l2);
  }
  const V = state.model.V;
  for (const v of d0.drivers) {
    const p = add(d0.start[v], d);
    if (d0.mm && d0.mm[v] === v) p[0] = 0;
    V[v] = p;
    if (d0.mm && d0.mm[v] >= 0 && d0.mm[v] !== v) V[d0.mm[v]] = [-p[0], p[1], p[2]];
  }
  invalidate();
}

// ---- 入力 ----
const pointers = new Map();
let pinch = 0;
const local = (e) => { const r = cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };

cv.addEventListener('contextmenu', (e) => e.preventDefault());
cv.addEventListener('pointerdown', (e) => {
  cv.setPointerCapture(e.pointerId);
  const p = local(e);
  pointers.set(e.pointerId, p);
  if (pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    pinch = Math.hypot(a[0] - b[0], a[1] - b[1]); drag = null; return;
  }
  const forceOrbit = e.button === 2 || e.button === 1 || e.altKey;
  const hit = forceOrbit ? null : pickItem(p[0], p[1]);
  const additive = e.shiftKey || e.ctrlKey || e.metaKey;
  if (hit === null || hit === undefined) {
    drag = { kind: 'orbit', last: p, moved: 0, clear: !forceOrbit && !additive };
  } else {
    if (state.sel.has(hit) && !additive) {
      if (state.mirror && !state.primary.has(hit)) state.primary = mirrorSet(state.primary);
    } else select(hit, additive);
    drag = beginMove();
    drag.last = p;
    syncUi(); schedule();
  }
  cv.classList.add('dragging');
});
cv.addEventListener('pointermove', (e) => {
  if (!pointers.has(e.pointerId)) return;
  const p = local(e);
  pointers.set(e.pointerId, p);
  if (pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
    if (pinch > 0 && d > 0) zoom(pinch / d);
    pinch = d; return;
  }
  if (!drag) return;
  const dx = p[0] - drag.last[0], dy = p[1] - drag.last[1];
  drag.last = p;
  drag.moved += Math.abs(dx) + Math.abs(dy);
  if (drag.kind === 'orbit') {
    state.yaw -= dx * 0.008;
    state.pitch = Math.max(-1.45, Math.min(1.45, state.pitch + dy * 0.008));
    schedule();
  } else if (drag.moved > 3) {
    drag.total = [drag.total[0] + dx, drag.total[1] + dy];
    applyMove(drag);
  }
});
const end = (e) => {
  pointers.delete(e.pointerId);
  if (pointers.size < 2) pinch = 0;
  if (pointers.size) return;
  if (drag && e.type === 'pointerup') {
    if (drag.kind === 'move' && drag.moved > 3) {
      undoStack.push(drag.before);
      if (undoStack.length > 100) undoStack.shift();
      redoStack.length = 0;
      syncUi();
    } else if (drag.kind === 'orbit' && drag.moved < 4 && drag.clear) clearSel();
  }
  drag = null;
  cv.classList.remove('dragging');
};
cv.addEventListener('pointerup', end);
cv.addEventListener('pointercancel', (e) => {
  if (drag && drag.kind === 'move' && drag.moved > 3) { state.model = drag.before; invalidate(); }
  end(e);
});
const zoom = (f) => { state.dist = Math.max(3, Math.min(16, state.dist * f)); schedule(); };
cv.addEventListener('wheel', (e) => { e.preventDefault(); zoom(Math.exp(e.deltaY * 0.0012)); }, { passive: false });
$('zin').addEventListener('click', () => zoom(0.85));
$('zout').addEventListener('click', () => zoom(1 / 0.85));

window.addEventListener('keydown', (e) => {
  if (/^(INPUT|SELECT|TEXTAREA)$/.test(e.target.tagName) && e.target.type !== 'checkbox' && e.target.type !== 'range') return;
  const mod = e.ctrlKey || e.metaKey;
  const k = e.key.toLowerCase();
  if (mod && k === 'z') { e.preventDefault(); if (e.shiftKey) redo(); else undo(); return; }
  if (mod && k === 'y') { e.preventDefault(); redo(); return; }
  if (mod && k === 'a') { e.preventDefault(); selectAll(); return; }
  if (mod) return;
  if (k === '1') setMode('v');
  else if (k === '2') setMode('e');
  else if (k === '3') setMode('f');
  else if (k === 'e') opExtrude();
  else if (k === 'l') opLoopCut();
  else if (k === 'c') opCrease();
  else if (k === 'm') setMirror(!state.mirror);
  else if (k === 'delete' || k === 'backspace') { e.preventDefault(); opDelete(); }
  else if (k === 'escape') clearSel();
  else if (k === 'x' || k === 'y' || k === 'z') setAxis(state.axis === k ? 'free' : k);
});

function setMirror(on) {
  state.mirror = on;
  $('mirror').checked = on;
  if (on) {
    const m = mirrorSet(state.primary);
    m.forEach((x) => state.sel.add(x));
    toast('ミラー編集：x=0 の面で左右対称に編集します');
  }
  syncUi(); schedule();
}
function setAxis(a) { state.axis = a; syncUi(); }

// ---- 画面の部品 ----
const presets = $('presets');
Object.entries(M.PRESETS).forEach(([key, p]) => {
  const b = document.createElement('button');
  b.textContent = p.name;
  b.addEventListener('click', () => newShape(key));
  presets.appendChild(b);
});
document.querySelectorAll('#modes button').forEach((b) => b.addEventListener('click', () => setMode(b.dataset.mode)));
document.querySelectorAll('#axes button').forEach((b) => b.addEventListener('click', () => setAxis(b.dataset.axis)));
$('selAll').addEventListener('click', selectAll);
$('selNone').addEventListener('click', clearSel);
$('extrude').addEventListener('click', opExtrude);
$('loopcut').addEventListener('click', opLoopCut);
$('del').addEventListener('click', opDelete);
$('crease').addEventListener('click', opCrease);
$('undo').addEventListener('click', undo);
$('redo').addEventListener('click', redo);
$('mirror').addEventListener('change', (e) => setMirror(e.target.checked));
$('extAmount').addEventListener('input', (e) => { state.extrude = +e.target.value; $('extAmountV').textContent = state.extrude.toFixed(2); });
$('tau').addEventListener('input', (e) => { state.tau = +e.target.value; $('tauV').textContent = state.tau.toFixed(2); invalidate(); });
$('div').addEventListener('input', (e) => { state.N = +e.target.value; $('divV').textContent = state.N; invalidate(); });
const bindShow = (id, key) => $(id).addEventListener('change', (e) => { state.show[key] = e.target.checked; schedule(); });
bindShow('sSurface', 'surface'); bindShow('sCurves', 'curves'); bindShow('sCage', 'cage'); bindShow('sCtrl', 'ctrl');

function resize() {
  const r = cv.getBoundingClientRect();
  W = r.width; H = r.height; dpr = Math.min(window.devicePixelRatio || 1, 2);
  cv.width = Math.round(W * dpr); cv.height = Math.round(H * dpr);
  schedule();
}
new ResizeObserver(resize).observe(cv);
matchMedia('(prefers-color-scheme: dark)').addEventListener('change', schedule);
new MutationObserver(schedule).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
syncUi();
invalidate();

export { state };
