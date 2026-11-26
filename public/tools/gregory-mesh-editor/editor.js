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
  mirror: false, axis: 'free', shading: 'smooth', tau: 1, N: 8, extrude: 0.4,
  show: { surface: true, curves: true, cage: true, ctrl: false, gizmo: true },
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
const vertsOf = (items, mode = state.mode, model = state.model) => M.selectionVertices(model, mode, items);

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
  const sel = M.convertSelection(state.model, state.mode, state.sel, next);
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
  const eye = [state.dist * cp * cy, state.dist * sp, -state.dist * cp * sy];
  const f = norm(mul(eye, -1));
  const r = norm(cross(f, [0, 1, 0]));
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

// ---- 曲面のラスタライズ ----
// 2倍の解像度で描いて縮小し、輪郭を滑らかにする
const SS = 2;
const surfCanvas = document.createElement('canvas');
const surfCtx = surfCanvas.getContext('2d');
let surfImg = null, zbuf = null;
const LIGHT = norm([-0.4, 0.6, -0.7]);
const HALF = norm([LIGHT[0], LIGHT[1], LIGHT[2] - 1]);

function drawSurface(s) {
  const w = Math.round(W * SS), h = Math.round(H * SS);
  if (surfCanvas.width !== w || surfCanvas.height !== h || !surfImg) {
    surfCanvas.width = w; surfCanvas.height = h;
    surfImg = surfCtx.createImageData(w, h);
    zbuf = new Float32Array(w * h);
  }
  const px = surfImg.data;
  px.fill(0); zbuf.fill(0);
  const flat = state.shading === 'flat';
  const { cam, pos, tris, base, warm } = s;
  for (let t = 0; t < tris.length; t++) {
    const [a, b, c] = tris[t];
    if ((s.vnz[a] + s.vnz[b] + s.vnz[c]) / 3 > 0.06) continue;
    const x0 = s.sx[a] * SS, y0 = s.sy[a] * SS, x1 = s.sx[b] * SS, y1 = s.sy[b] * SS, x2 = s.sx[c] * SS, y2 = s.sy[c] * SS;
    if (!Number.isFinite(x0 + y0 + x1 + y1 + x2 + y2)) continue;
    const area = (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0);
    if (Math.abs(area) < 1e-9) continue;
    // 奥行きの逆数は画面上で線形に補間できる。法線も逆数を掛けて補間し、遠近を正しく反映する
    const i0 = 1 / s.sz[a], i1 = 1 / s.sz[b], i2 = 1 / s.sz[c];
    const nx0 = s.vnx[a] * i0, nx1 = s.vnx[b] * i1, nx2 = s.vnx[c] * i2;
    const ny0 = s.vny[a] * i0, ny1 = s.vny[b] * i1, ny2 = s.vny[c] * i2;
    const nz0 = s.vnz[a] * i0, nz1 = s.vnz[b] * i1, nz2 = s.vnz[c] * i2;
    let flatShade = 0, flatSpec = 0;
    if (flat) {
      let fn = norm(cross(sub(pos[b], pos[a]), sub(pos[c], pos[a])));
      if (fn[0] === 0 && fn[1] === 0 && fn[2] === 0) fn = [0, 0, -1];
      const l = lighting(dot(fn, cam.r), dot(fn, cam.u), dot(fn, cam.f));
      flatShade = l[0]; flatSpec = l[1];
    }
    const tint = s.faceSel.has(s.triFace[t]);
    const minX = Math.max(0, Math.floor(Math.min(x0, x1, x2))), maxX = Math.min(w - 1, Math.ceil(Math.max(x0, x1, x2)));
    const minY = Math.max(0, Math.floor(Math.min(y0, y1, y2))), maxY = Math.min(h - 1, Math.ceil(Math.max(y0, y1, y2)));
    for (let y = minY; y <= maxY; y++) {
      const py = y + 0.5;
      for (let x = minX; x <= maxX; x++) {
        const pxc = x + 0.5;
        const w1 = ((pxc - x0) * (y2 - y0) - (py - y0) * (x2 - x0)) / area;
        const w2 = ((x1 - x0) * (py - y0) - (y1 - y0) * (pxc - x0)) / area;
        const w0 = 1 - w1 - w2;
        if (w0 < -1e-6 || w1 < -1e-6 || w2 < -1e-6) continue;
        const iz = w0 * i0 + w1 * i1 + w2 * i2;
        const idx = y * w + x;
        if (iz <= zbuf[idx]) continue;
        zbuf[idx] = iz;
        let shade = flatShade, spec = flatSpec;
        if (!flat) {
          const nx = (w0 * nx0 + w1 * nx1 + w2 * nx2) / iz;
          const ny = (w0 * ny0 + w1 * ny1 + w2 * ny2) / iz;
          const nzz = (w0 * nz0 + w1 * nz1 + w2 * nz2) / iz;
          const len = Math.hypot(nx, ny, nzz);
          const l = len > 1e-9 ? lighting(nx / len, ny / len, nzz / len) : [0.3, 0];
          shade = l[0]; spec = l[1];
        }
        let r = base[0] * shade + 255 * spec, g = base[1] * shade + 255 * spec, bl = base[2] * shade + 255 * spec;
        if (tint) { r = r * 0.45 + warm[0] * 0.55; g = g * 0.45 + warm[1] * 0.55; bl = bl * 0.45 + warm[2] * 0.55; }
        const o = idx * 4;
        px[o] = r > 255 ? 255 : r; px[o + 1] = g > 255 ? 255 : g; px[o + 2] = bl > 255 ? 255 : bl; px[o + 3] = 255;
      }
    }
  }
  surfCtx.putImageData(surfImg, 0, 0);
}

function lighting(nx, ny, nzz) {
  const d = nx * LIGHT[0] + ny * LIGHT[1] + nzz * LIGHT[2];
  const hh = nx * HALF[0] + ny * HALF[1] + nzz * HALF[2];
  return [0.3 + 0.1 * ny + 0.68 * Math.max(0, d), 0.22 * Math.pow(Math.max(0, hh), 28)];
}

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
  const nz = new Float32Array(n), vnx = new Float32Array(n), vny = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const p = project(cam, pos[i]);
    sx[i] = p[0]; sy[i] = p[1]; sz[i] = p[2];
    vnx[i] = dot(nrm[i], cam.r); vny[i] = dot(nrm[i], cam.u); nz[i] = dot(nrm[i], cam.f);
  }
  view = { sx, sy, sz, nz, tris, triFace };
  const faceSel = state.mode === 'f' ? state.sel : new Set();
  if (state.show.surface) {
    drawSurface({ cam, pos, tris, triFace, sx, sy, sz, vnx, vny, vnz: nz, faceSel, base, warm });
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(surfCanvas, 0, 0, W, H);
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

  computeGizmo(cam);
  drawGizmo(tok);
}

// ---- 移動ハンドル ----
const GIZMO_PX = 84;
const PLANES = { xy: ['x', 'y', 'z'], yz: ['y', 'z', 'x'], zx: ['z', 'x', 'y'] };
let gizmo = null, hoverHandle = null;

function gizmoCenter() {
  const items = state.primary.size ? state.primary : state.sel;
  if (!items.size) return null;
  const verts = [...vertsOf(items)].filter((v) => state.model.V[v]);
  if (!verts.length) return null;
  return mul(verts.reduce((acc, v) => add(acc, state.model.V[v]), [0, 0, 0]), 1 / verts.length);
}

function computeGizmo(cam) {
  const c = state.show.gizmo ? gizmoCenter() : null;
  const z = c ? dot(sub(c, cam.eye), cam.f) : 0;
  if (!c || z < 0.1) { gizmo = null; return; }
  const Lw = (GIZMO_PX * z) / cam.F;
  const at = (a, u, b, v) => project(cam, add(c, mul(add(mul(AXES[a], u), mul(AXES[b], v)), Lw)));
  const tips = {};
  for (const k of ['x', 'y', 'z']) tips[k] = project(cam, add(c, mul(AXES[k], Lw)));
  const planes = {};
  for (const [name, [a, b]] of Object.entries(PLANES)) {
    planes[name] = [at(a, 0.22, b, 0.22), at(a, 0.5, b, 0.22), at(a, 0.5, b, 0.5), at(a, 0.22, b, 0.5)];
  }
  gizmo = { c, Lw, c0: project(cam, c), tips, planes };
}

function drawGizmo(tok) {
  if (!gizmo) return;
  const active = drag && drag.kind === 'move' ? drag.handle : null;
  const lit = (name) => hoverHandle === name || active === name;
  const col = { x: tok('--ax-x'), y: tok('--ax-y'), z: tok('--ax-z') };
  const warm = tok('--warm');
  for (const [name, pts] of Object.entries(gizmo.planes)) {
    const rgb = hexToRgb(lit(name) ? warm : col[PLANES[name][2]]);
    ctx.fillStyle = `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${lit(name) ? 0.75 : 0.4})`;
    ctx.strokeStyle = `rgb(${rgb[0]},${rgb[1]},${rgb[2]})`;
    ctx.lineWidth = 1;
    ctx.beginPath();
    pts.forEach((q, i) => (i ? ctx.lineTo(q[0], q[1]) : ctx.moveTo(q[0], q[1])));
    ctx.closePath(); ctx.fill(); ctx.stroke();
  }
  ctx.lineCap = 'round';
  ctx.font = '600 12px ui-monospace, Menlo, Consolas, monospace';
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  for (const k of ['x', 'y', 'z']) {
    const tip = gizmo.tips[k], c0 = gizmo.c0;
    const color = lit(k) ? warm : col[k];
    const len = Math.hypot(tip[0] - c0[0], tip[1] - c0[1]) || 1;
    const ux = (tip[0] - c0[0]) / len, uy = (tip[1] - c0[1]) / len;
    ctx.strokeStyle = color; ctx.fillStyle = color; ctx.lineWidth = 3;
    ctx.beginPath(); ctx.moveTo(c0[0], c0[1]); ctx.lineTo(tip[0] - ux * 8, tip[1] - uy * 8); ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(tip[0], tip[1]);
    ctx.lineTo(tip[0] - ux * 14 - uy * 5.5, tip[1] - uy * 14 + ux * 5.5);
    ctx.lineTo(tip[0] - ux * 14 + uy * 5.5, tip[1] - uy * 14 - ux * 5.5);
    ctx.closePath(); ctx.fill();
    ctx.fillText(k.toUpperCase(), tip[0] + ux * 13, tip[1] + uy * 13);
  }
  ctx.beginPath(); ctx.arc(gizmo.c0[0], gizmo.c0[1], 4, 0, Math.PI * 2);
  ctx.fillStyle = tok('--curve'); ctx.fill();
}

function pickHandle(x, y) {
  if (!gizmo) return null;
  for (const [name, pts] of Object.entries(gizmo.planes)) {
    let sign = 0, inside = true;
    for (let i = 0; i < 4 && inside; i++) {
      const a = pts[i], b = pts[(i + 1) % 4];
      const cr = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]);
      if (cr !== 0) { if (sign && Math.sign(cr) !== sign) inside = false; sign = Math.sign(cr); }
    }
    if (inside) return name;
  }
  let best = null, bd = 9;
  for (const k of ['x', 'y', 'z']) {
    const [ax, ay] = gizmo.c0, [bx, by] = gizmo.tips[k];
    const l2 = (bx - ax) ** 2 + (by - ay) ** 2 || 1;
    const t = Math.max(0, Math.min(1, ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / l2));
    const d = Math.hypot(x - (ax + t * (bx - ax)), y - (ay + t * (by - ay)));
    if (d < bd) { bd = d; best = k; }
  }
  return best;
}

// 画面上の点を通る視線と、中心を通る平面（法線 n）の交点
function rayHit(cam, p, n, c) {
  const a = (p[0] - W / 2) / cam.F, b = -(p[1] - H / 2) / cam.F;
  const dir = add(cam.f, add(mul(cam.r, a), mul(cam.u, b)));
  const den = dot(n, dir);
  if (Math.abs(den) < 1e-6) return null;
  const t = dot(n, sub(c, cam.eye)) / den;
  return t > 0 ? add(cam.eye, mul(dir, t)) : null;
}
const gizmoInfo = () => gizmo;

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
  document.querySelectorAll('#shading button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.shading === state.shading)));
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
  const handle = d0.handle || (state.axis !== 'free' ? state.axis : null);
  if (handle && handle.length === 2) {
    const hit = rayHit(cam, d0.pos, AXES[PLANES[handle][2]], d0.center);
    d = hit && d0.hit0 ? sub(hit, d0.hit0) : [0, 0, 0];
  } else if (handle) {
    const ax = AXES[handle];
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
  const handle = forceOrbit ? null : pickHandle(p[0], p[1]);
  if (handle) {
    drag = beginMove();
    Object.assign(drag, { last: p, pos: p, handle });
    if (handle.length === 2) drag.hit0 = rayHit(camera(), p, AXES[PLANES[handle][2]], drag.center);
    cv.classList.add('dragging');
    schedule();
    return;
  }
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
cv.addEventListener('pointerleave', () => { if (hoverHandle) { hoverHandle = null; cv.style.cursor = ''; schedule(); } });
cv.addEventListener('pointermove', (e) => {
  if (!pointers.has(e.pointerId)) {
    const h = pickHandle(...local(e));
    if (h !== hoverHandle) { hoverHandle = h; cv.style.cursor = h ? 'pointer' : ''; schedule(); }
    return;
  }
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
    drag.pos = p;
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
  else if (k === 's') setShading(state.shading === 'smooth' ? 'flat' : 'smooth');
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
function setShading(v) { state.shading = v; syncUi(); schedule(); }

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
document.querySelectorAll('#shading button').forEach((b) => b.addEventListener('click', () => setShading(b.dataset.shading)));
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
bindShow('sSurface', 'surface'); bindShow('sCurves', 'curves'); bindShow('sCage', 'cage'); bindShow('sCtrl', 'ctrl'); bindShow('sGizmo', 'gizmo');

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

export { state, gizmoInfo };
