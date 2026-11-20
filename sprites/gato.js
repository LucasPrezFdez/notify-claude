// Gato esmoquin procedural: una pose continua -> rejilla de píxeles. Sirve igual en Node y en el navegador.
const GW = 58, GH = 59, OY = 10, CX = 23.5, FLOOR = 46, FPS = 30;
const lerp = (a, b, t) => a + (b - a) * t;

function drawCat(q = {}) {
  const p = Object.assign({ b: 0, br: 0, hx: 0, pawL: null, pawR: null, eyeL: 1, eyeR: 1, happyL: false, happyR: false, look: 0, pupil: 1.1, earL: [0, 0], earR: [0, 0] }, q);
  const tl = Object.assign({ a: 0, tb: 0, up: 0 }, q.tail);
  const g = Array.from({ length: GH }, () => Array.from({ length: GW }, () => null));
  let ox = 0, oy = 0;
  const put = (x, y, part, mat, shade = 0) => { const gx = x + ox, gy = y + oy + OY; if (gx >= 0 && gy >= 0 && gx < GW && gy < GH) g[gy][gx] = { part, mat, shade }; };
  const get = (x, y) => (g[y + oy + OY] || [])[x + ox] || null;
  const band = n => n > 0.55 ? 1 : n < -0.6 ? -1 : 0;
  const Y0 = -OY - 4, Y1 = GH - OY + 4;
  function ellipse(cx, cy, rx, ry, part, mat, opt = {}) {
    for (let y = Math.max(Y0, Math.floor(cy - ry)); y <= Math.min(Y1, Math.ceil(cy + ry)); y++) for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x++) {
      const u = (x - cx) / rx, v = (y - cy) / ry;
      if (u * u + v * v > 1) continue;
      if (opt.maxY != null && y > opt.maxY) continue;
      if (opt.only && !(get(x, y) && opt.only.includes(get(x, y).part))) continue;
      put(x, y, part, mat, opt.flat ? 0 : band(-0.6 * u - 0.8 * v));
    }
  }
  function tri(pt, part, mat, shade = 0) {
    const s = (a, c, x, y) => (c[0] - a[0]) * (y - a[1]) - (c[1] - a[1]) * (x - a[0]);
    const xs = pt.map(v => v[0]), ys = pt.map(v => v[1]);
    for (let y = Math.floor(Math.min(...ys)); y <= Math.ceil(Math.max(...ys)); y++) for (let x = Math.floor(Math.min(...xs)); x <= Math.ceil(Math.max(...xs)); x++) {
      const d = [s(pt[0], pt[1], x, y), s(pt[1], pt[2], x, y), s(pt[2], pt[0], x, y)];
      if (d.every(v => v >= 0) || d.every(v => v <= 0)) put(x, y, part, mat, shade);
    }
  }
  function capsule(ax, ay, bx, by, r, part, mat) {
    for (let i = 0; i <= 40; i++) {
      const cx = lerp(ax, bx, i / 40), cy = lerp(ay, by, i / 40);
      for (let y = Math.floor(cy - r); y <= Math.ceil(cy + r); y++) for (let x = Math.floor(cx - r); x <= Math.ceil(cx + r); x++) {
        const dx = x - cx, dy = y - cy; if (dx * dx + dy * dy <= r * r) put(x, y, part, mat, band((-0.6 * dx - 0.8 * dy) / r));
      }
    }
  }
  const both = f => { f(x => x, 'L'); f(x => 2 * CX - x, 'R'); };
  const b = p.b, hd = Math.round(b * 2), hx = Math.round(p.hx);

  // cola
  const A = [[33, 43], [50 + tl.a, 46], [49 + 1.5 * tl.a + 1.5 * tl.tb, 31 + Math.abs(tl.tb) * 0.5], [42 + 4 * tl.tb + tl.a, 27]];
  const U = [[33, 43], [46, 46], [47, 32], [46, 14]];
  const P = A.map((v, i) => [lerp(v[0], U[i][0], tl.up), lerp(v[1], U[i][1], tl.up)]);
  for (let i = 0; i <= 220; i++) {
    const t = i / 220, a = 1 - t;
    const bx = a * a * a * P[0][0] + 3 * a * a * t * P[1][0] + 3 * a * t * t * P[2][0] + t * t * t * P[3][0];
    const by = a * a * a * P[0][1] + 3 * a * a * t * P[1][1] + 3 * a * t * t * P[2][1] + t * t * t * P[3][1];
    const r = 2.9 - 0.7 * t;
    for (let y = Math.floor(by - r); y <= Math.ceil(by + r); y++) for (let x = Math.floor(bx - r); x <= Math.ceil(bx + r); x++) {
      const dx = x - bx, dy = y - by; if (dx * dx + dy * dy > r * r || y > FLOOR) continue;
      put(x, y, 'tail', t > 0.8 ? 'W' : 'K', t > 0.8 ? (dx > r - 1.3 ? -1 : 0) : band((-0.6 * dx - 0.8 * dy) / r));
    }
  }
  // cuerpo y pecho
  ellipse(CX, 35 + b, 11.5 + b * 0.5 + p.br * 0.6, 13 - b, 'body', 'K', { maxY: FLOOR });
  for (let y = 22 + hd; y <= 42; y++) {
    const rx = 6.2 - Math.max(0, y - 33) * 0.55 + (y % 3 === 0 ? 1 : 0) + p.br * 0.5;
    for (let x = 0; x < GW; x++) if (Math.abs(x - CX) <= rx && get(x, y) && get(x, y).part === 'body') put(x, y, 'body', 'W', x - CX > rx - 2 ? -1 : 0);
  }
  // ancas y patas traseras
  both((m, s) => ellipse(m(12.5), 38.5 + b * 0.5, 6 + b * 0.5, 8 - b * 0.5, 'haunch' + s, 'K', { maxY: FLOOR }));
  both((m, s) => ellipse(m(11.5), 44.6, 4.6, 2.4, 'foot' + s, 'W', { maxY: FLOOR }));
  both((m, s) => { for (const x of [10, 13]) put(Math.round(m(x)), 45, 'foot' + s, 'M'); });
  // patas delanteras apoyadas
  const lifted = s => p['paw' + s] && p['paw' + s].lift > 0.02;
  both((m, s) => {
    if (lifted(s)) return;
    const part = 'leg' + s, c = m(20);
    for (let y = 33; y <= FLOOR; y++) for (let x = 0; x < GW; x++) if (Math.abs(x - c) <= 2.6) put(x, y, part, 'K', x - c < -1.5 ? 1 : x - c > 1.5 ? -1 : 0);
    ellipse(c, 44.4, 3.6, 2.6, part, 'W', { maxY: FLOOR });
    for (const x of [19, 21]) put(Math.round(m(x)), 45, part, 'M');
  });

  // cabeza (se desplaza entera con hx, hd)
  ox = hx; oy = hd;
  both((m, s) => {
    const e = p['ear' + s];
    tri([[m(9.5), 13], [m(11.5 - e[0]), 1 + e[1]], [m(20.5), 7]], 'head', 'K');
    tri([[m(12), 10.5], [m(12.8 - e[0] * 0.8), 4.5 + e[1] * 0.8], [m(17), 8.2]], 'head', 'P');
    tri([[m(12), 10.5], [m(12.8 - e[0] * 0.8), 4.5 + e[1] * 0.8], [m(13.6), 9.5]], 'head', 'P', -1);
  });
  ellipse(CX, 15.5, 13.2, 10.6, 'head', 'K');
  both(m => ellipse(m(12), 19.5, 4.2, 4.2, 'head', 'K', { flat: true }));
  both(m => { for (const [x, y] of [[7, 20], [7, 22], [8, 23], [10, 24]]) put(Math.round(m(x)), y, 'head', 'K'); });
  for (let y = 7; y <= 18; y++) { const w = 0.6 + (y - 7) * 0.33; for (let x = 0; x < GW; x++) if (Math.abs(x - CX) <= w) put(x, y, 'head', 'W'); }
  ellipse(CX, 20.6, 6.6, 4.3, 'head', 'W', { only: ['head'] });
  // ojos
  both((m, s) => {
    const open = s === 'L' ? p.eyeL : p.eyeR, happy = s === 'R' ? p.happyR : p.happyL;
    const ex = m(16.5), ey = 15.5, x0 = Math.floor(ex) - 2;
    if (open < 0.15) { for (let i = 0; i < 6; i++) { const mid = i > 0 && i < 5; put(x0 + i, 16 + (happy ? (mid ? -1 : 0) : (mid ? 1 : 0)), 'eye', 'L'); } return; }
    const cutTop = 13 + Math.round((1 - open) * 3), cutBot = 18 - Math.round((1 - open) * 1.4), pcx = ex + Math.round(p.look);
    for (let y = cutTop; y <= cutBot; y++) for (let x = x0 - 2; x <= x0 + 7; x++) {
      const u = (x - ex) / 3.1, v = (y - ey) / 2.9; if (u * u + v * v > 1) continue;
      const pu = (x - pcx) / p.pupil, pv = (y - ey) / 2.7;
      put(x, y, 'eye', pu * pu + pv * pv <= 1 ? 'N' : 'Y', y === cutTop ? -1 : y >= 17 ? 1 : 0);
    }
    if (open > 0.7) put(Math.floor(ex) - 1, 14, 'eye', 'S');
  });
  // nariz, boca y bigotes
  for (const [x, y] of [[22, 18], [23, 18], [24, 18], [25, 18], [23, 19], [24, 19]]) put(x, y, 'head', 'P', y === 19 ? -1 : 0);
  for (const [x, y] of [[23, 20], [24, 20], [22, 21], [21, 21], [20, 20], [25, 21], [26, 21], [27, 20], [23, 23], [24, 23]]) put(x, y, 'head', 'M');
  both(m => { for (let i = 0; i < 8; i++) {
    put(Math.round(m(15 - i)), 19 - Math.round(i * 0.3), 'whisk', 'G'); put(Math.round(m(15 - i)), 21 + Math.round(i * 0.12), 'whisk', 'G');
    if (i < 6) put(Math.round(m(15 - i)), 23 + Math.round(i * 0.3), 'whisk', 'G');
  } });
  ox = 0; oy = 0;

  // patas delanteras levantadas (por delante de todo)
  both((m, s) => {
    if (!lifted(s)) return;
    const w = p['paw' + s], k = Math.min(1, w.lift), part = 'arm' + s;
    const px = Math.round(lerp(m(20), m(w.x), k) * 2) / 2, py = Math.round(lerp(44.4, w.y, k) * 2) / 2;
    capsule(m(19), 34 + hd, px, py, 2.4, part, 'K');
    ellipse(px, py, lerp(3.6, 3.3, k), lerp(2.6, 3.0, k), part, 'W', k < 0.3 ? { maxY: FLOOR } : {});
    if (k > 0.6) for (const d of [-1, 1]) put(Math.round(px + d - 0.01), Math.round(py) - 2, part, 'M');
  });

  // sombra bajo la barbilla, separadores y contorno
  const at = (x, y) => (g[y] || [])[x] || null;
  for (let y = 0; y < GH; y++) for (let x = 0; x < GW; x++) {
    const c = at(x, y); if (!c || ['head', 'whisk', 'eye', 'armL', 'armR'].includes(c.part)) continue;
    if ([1, 2].some(d => (at(x, y - d) || {}).part === 'head')) c.shade = -1;
  }
  const sep = [];
  for (let y = 0; y < GH; y++) for (let x = 0; x < GW; x++) {
    const c = at(x, y); if (!c || !'KW'.includes(c.mat) || !['body', 'tail', 'haunchL', 'haunchR', 'head'].includes(c.part)) continue;
    for (const n of [at(x + 1, y), at(x, y + 1), at(x - 1, y), at(x, y - 1)]) if (n && n.mat === c.mat && n.part !== c.part && !['whisk', 'eye'].includes(n.part) && (c.part !== 'head' || n.part.startsWith('arm'))) sep.push(c);
  }
  sep.forEach(c => c.sep = true);
  const col = c => {
    if (c.sep) return c.mat === 'K' ? [62, 62, 76] : [176, 178, 192];
    switch (c.mat) {
      case 'K': return [[12, 12, 16], [26, 26, 31], [46, 47, 58]][c.shade + 1];
      case 'W': return [[196, 199, 212], [247, 247, 242], [255, 255, 255]][c.shade + 1];
      case 'P': return c.shade < 0 ? [198, 110, 126] : [238, 152, 164];
      case 'Y': return [[196, 140, 36], [238, 196, 60], [214, 220, 96]][c.shade + 1];
      case 'N': return [8, 8, 10];
      case 'S': return [255, 255, 255];
      case 'G': return [214, 216, 226];
      case 'L': return [186, 188, 204];
      case 'M': return [150, 152, 168];
    }
  };
  const OUT = [70, 71, 86];
  return g.map((row, y) => row.map((c, x) => c ? col(c) : ([[1, 0], [-1, 0], [0, 1], [0, -1]].some(([a, d]) => { const n = at(x + a, y + d); return n && n.part !== 'whisk'; }) ? OUT : null)));
}

// ---------- animación: pistas con curvas de aceleración ----------
const E = {
  lin: x => x, in: x => x * x, out: x => 1 - Math.pow(1 - x, 3),
  io: x => x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2,
  back: x => { const c = 1.70158; return 1 + (c + 1) * Math.pow(x - 1, 3) + c * Math.pow(x - 1, 2); },
};
function tr(t, keys) {
  if (t <= keys[0][0]) return keys[0][1];
  for (let i = 1; i < keys.length; i++) if (t <= keys[i][0]) { const [t0, v0] = keys[i - 1], [t1, v1, e] = keys[i]; return v0 + (v1 - v0) * E[e || 'io']((t - t0) / (t1 - t0)); }
  return keys[keys.length - 1][1];
}
const blink = (t, t0, d = 0.3) => { const u = (t - t0) / d; return u <= 0 || u >= 1 ? 1 : u < 0.35 ? 1 - E.io(u / 0.35) : u < 0.5 ? 0 : E.io((u - 0.5) / 0.5); };
const pulse = (t, t0, d = 0.24) => { const u = (t - t0) / d; return u <= 0 || u >= 1 ? 0 : Math.sin(Math.PI * u); };
const TAU = Math.PI * 2, PER = 3.2;
// capa de reposo (respiración y cola con la punta retrasada); w la atenúa
const idle = (T, w = 1) => ({ br: (0.5 - 0.5 * Math.cos(TAU * T / PER)) * w, tail: { a: 1.2 * Math.sin(TAU * T / PER) * w, tb: 1.5 * Math.sin(TAU * T / PER - 1.0) * w, up: 0 } });
const ear = f => [2.5 * f, 3 * f];
// saltito parabólico de altura h que empieza en t0 y dura d
const hop = (t, t0, d, h) => { const u = (t - t0) / d; return u <= 0 || u >= 1 ? 0 : -h * 4 * u * (1 - u); };

const ANIMS = {
  quieto: { dur: 1.2, fn: (t, T) => ({ dy: 0, pose: idle(T) }) },
  oculto: { dur: 0.9, fn: () => ({ dy: 70, pose: {} }) },

  reposo: { dur: PER * 2, fn: (t, T) => {
    const e = Math.min(blink(t, 1.8, 0.34), blink(t, 5.0, 0.3));
    return { dy: 0, pose: Object.assign(idle(T), {
      eyeL: e, eyeR: e, look: t > 3.0 && t < 3.9 ? -1 : 0,
      earR: [1.6 * pulse(t, 2.6), 0.6 * pulse(t, 2.6)], earL: [1.6 * pulse(t, 4.4), 0.6 * pulse(t, 4.4)],
    }) };
  } },

  entrada: { dur: 3.4, fn: (t, T) => {
    const J0 = 1.95, JD = 0.5, u = (t - J0) / JD, s = t - (J0 + JD);
    const dy = t < J0 ? tr(t, [[0.45, 50], [0.9, 29, 'back'], [1.6, 29], [J0, 35]]) : u < 1 ? 35 * (1 - u) - 88 * u * (1 - u) : 0;
    const w = tr(t, [[2.8, 0], [3.4, 1]]), pose = idle(T, w);
    const edge = 48 - dy - 1.5;
    if (t < J0) {
      pose.pawL = { lift: 1, x: 5, y: edge + tr(t, [[0, 6], [0.22, 0, 'back']]) };
      pose.pawR = { lift: 1, x: 5, y: edge + tr(t, [[0.14, 6], [0.36, 0, 'back']]) };
    } else if (u < 1) {
      const pw = { lift: tr(u, [[0.4, 1], [0.95, 0, 'in']]), x: tr(u, [[0, 5], [0.35, 10]]), y: tr(u, [[0, 10.5], [0.35, 27]]) };
      pose.pawL = pw; pose.pawR = pw;
      pose.tail = { a: 0.65 * u, tb: -2.2 * u, up: 0 };
    } else {
      const d = Math.exp(-3.2 * s);
      pose.tail.a += 0.65 * d * Math.cos(9 * s) * (1 - w); pose.tail.tb += -2.2 * d * Math.cos(9 * s) * (1 - w);
    }
    pose.b = tr(t, [[1.6, 0], [1.95, 0.8], [2.05, -1.2, 'out'], [2.35, -0.6], [2.45, 0.2, 'lin'], [2.52, 1.7, 'out'], [2.72, -0.4], [2.95, 0.15], [3.15, 0]]);
    const f = tr(t, [[0.9, 0.6], [1.05, 0, 'back'], [1.6, 0], [1.9, 0.5], [2.1, 0.8], [2.45, 0.6], [2.55, 1], [2.9, 0, 'back']]);
    pose.earL = ear(f); pose.earR = [ear(f)[0] + 1.6 * pulse(t, 1.15), ear(f)[1]];
    pose.look = t > 0.98 && t < 1.22 ? -1 : t > 1.28 && t < 1.52 ? 1 : 0;
    const e = Math.min(blink(t, 1.55, 0.22), tr(t, [[2.45, 1], [2.52, 0.35], [2.85, 1]]));
    pose.eyeL = e; pose.eyeR = e;
    pose.pupil = tr(t, [[1.5, 1.1], [1.9, 2.0], [2.45, 2.0], [2.95, 1.1]]);
    return { dy, pose };
  } },

  pregunta: { dur: 4.8, fn: (t, T) => {
    const pose = idle(T), env = Math.max(tr(t, [[0.5, 0], [0.7, 1], [2.2, 1], [2.5, 0]]), tr(t, [[2.9, 0], [3.1, 1], [3.8, 1], [4.1, 0]]));
    const ph = TAU * 2.2 * (t - 0.6);
    pose.pawL = { lift: tr(t, [[0, 0], [0.35, 1, 'back'], [4.3, 1], [4.7, 0]]), x: 8.5 - 1.2 * env * Math.sin(ph), y: 28 - 2.6 * env * (0.5 - 0.5 * Math.cos(ph)) };
    pose.hx = -tr(t, [[0.2, 0], [0.45, 1], [4.2, 1], [4.5, 0]]);
    pose.pupil = tr(t, [[0, 1.1], [0.4, 1.5], [4.3, 1.5], [4.7, 1.1]]);
    const e = blink(t, 2.6, 0.3); pose.eyeL = e; pose.eyeR = e;
    pose.earR = [1.6 * pulse(t, 1.6), 0.6 * pulse(t, 1.6)];
    return { dy: 0, pose };
  } },

  permiso: { dur: 5.2, fn: (t, T) => {
    const pose = idle(T);
    pose.b = tr(t, [[0, 0], [0.15, 0.6], [0.5, -1.3, 'back'], [4.5, -1.3], [4.75, 0.5], [5.0, 0, 'out']]);
    const hd = Math.round(pose.b * 2), lift = tr(t, [[0.1, 0], [0.5, 1, 'out'], [4.45, 1], [4.8, 0]]);
    const env = Math.max(tr(t, [[1.1, 0], [1.3, 1], [1.9, 1], [2.1, 0]]), tr(t, [[3.0, 0], [3.2, 1], [3.8, 1], [4.0, 0]]));
    const bob = d => -1.8 * env * Math.abs(Math.sin(TAU * 1.25 * (t - d)));
    pose.pawL = { lift, x: 12.5, y: 30 + hd + bob(0) }; pose.pawR = { lift, x: 12.5, y: 30 + hd + bob(0.09) };
    pose.pupil = tr(t, [[0.1, 1.1], [0.6, 1.8], [4.4, 1.8], [4.9, 1.1]]);
    const e = Math.min(blink(t, 2.4, 0.3), blink(t, 4.15, 0.26)); pose.eyeL = e; pose.eyeR = e;
    return { dy: 0, pose };
  } },

  terminado: { dur: 4.8, fn: (t, T) => {
    // contento: cierra los ojos sonriendo, da dos saltitos con la cola en alto y se balancea
    const pose = idle(T);
    pose.happyL = true; pose.happyR = true;
    const e = tr(t, [[0.1, 1], [0.3, 0], [3.9, 0], [4.25, 1]]); pose.eyeL = e; pose.eyeR = e;
    pose.b = tr(t, [[0.3, 0], [0.5, 1.0], [0.6, -0.9, 'out'], [0.85, -0.4], [0.95, 1.1, 'out'], [1.1, -0.2], [1.25, 0.9], [1.35, -0.8, 'out'], [1.6, -0.3], [1.7, 1.0, 'out'], [1.85, -0.2], [2.0, 0]]);
    const f = tr(t, [[0.3, 0], [0.5, 0.5], [0.6, 0, 'back'], [0.95, 0.6], [1.1, 0], [1.25, 0.4], [1.35, 0, 'back'], [1.7, 0.6], [1.9, 0, 'back']]);
    pose.earL = ear(f); pose.earR = [ear(f)[0] + 1.6 * pulse(t, 2.8), ear(f)[1] + 0.6 * pulse(t, 2.8)];
    pose.tail.up = tr(t, [[0.5, 0], [0.9, 0.85], [3.6, 0.85], [4.2, 0]]);
    const env = tr(t, [[2.0, 0], [2.3, 1], [3.3, 1], [3.6, 0]]);
    pose.hx = 1.2 * env * Math.sin(TAU * 0.8 * (t - 2.1));
    return { dy: hop(t, 0.6, 0.35, 5) + hop(t, 1.35, 0.35, 4), pose };
  } },

  salida: { dur: 1.9, fn: (t, T) => {
    const pose = idle(T, tr(t, [[0.6, 1], [0.95, 0]]));
    pose.happyR = true; pose.eyeR = tr(t, [[0.05, 1], [0.2, 0], [0.5, 0], [0.68, 1]]); pose.eyeL = tr(t, [[0.05, 1], [0.2, 0.75], [0.5, 0.75], [0.68, 1]]);
    pose.b = tr(t, [[0.7, 0], [0.95, 1.3], [1.05, -0.8, 'out']]);
    const f = tr(t, [[0.7, 0], [0.95, 0.7], [1.15, 0.2]]); pose.earL = ear(f); pose.earR = ear(f);
    pose.tail.up = tr(t, [[0.95, 0], [1.3, 1]]);
    return { dy: tr(t, [[0.95, 0], [1.08, -4, 'out'], [1.75, 66, 'in']]), pose };
  } },
};

// escena: sprite + recorte en el borde de la barra. Devuelve colores o null (fondo) y 'bar'.
const STAGE = { W: GW + 4, TOP: 12, BAR: 6 }; STAGE.EDGE = STAGE.TOP + OY + 48; STAGE.H = STAGE.EDGE + STAGE.BAR;
function frameAt(name, t, T) { const r = ANIMS[name].fn(t, T == null ? t : T); return { px: drawCat(r.pose), dy: Math.round(r.dy) }; }

if (typeof module !== 'undefined') module.exports = { GW, GH, OY, FPS, drawCat, ANIMS, STAGE, frameAt };
