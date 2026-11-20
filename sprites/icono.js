// Icono del plugin: el gato sentado sobre la barra de tareas, en una rejilla de 64x64
// ampliada a 512x512 sin suavizar. `node sprites/icono.js` escribe plugin/.claude-plugin/icon.png.
const zlib = require('zlib'), fs = require('fs'), path = require('path');
const { frameAt } = require('./gato.js');

const N = 64, S = 8, BAR = 7, FONDO = [58, 110, 165], BARRA = [27, 31, 42];
const { px } = frameAt('quieto', 0);

// caja del gato dentro de su rejilla
let x0 = Infinity, x1 = -1, y0 = Infinity, y1 = -1;
px.forEach((row, y) => row.forEach((c, x) => { if (c) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); } }));
const w = x1 - x0 + 1, h = y1 - y0 + 1;
if (w > N || h > N - BAR) throw new Error(`el gato (${w}x${h}) no cabe en el icono`);
const ox = Math.round((N - w) / 2), oy = N - BAR - h; // centrado, con los pies en la barra

const W = N * S, raw = Buffer.alloc((W * 3 + 1) * W);
for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
  const c = (px[y - oy + y0] || [])[x - ox + x0] || (y >= N - BAR ? BARRA : FONDO);
  for (let a = 0; a < S; a++) for (let b = 0; b < S; b++) raw.set(c, (y * S + a) * (W * 3 + 1) + 1 + (x * S + b) * 3);
}

const chunk = (t, d) => { const b = Buffer.concat([Buffer.from(t), d]); const o = Buffer.alloc(8 + b.length); o.writeUInt32BE(d.length, 0); b.copy(o, 4); o.writeUInt32BE(zlib.crc32(b) >>> 0, 4 + b.length); return o; };
const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(W, 4); ihdr[8] = 8; ihdr[9] = 2;
const out = path.join(__dirname, '..', 'plugin', '.claude-plugin', 'icon.png');
fs.writeFileSync(out, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
console.log(`icono ${W}x${W} (gato de ${w}x${h}) en ${out}`);
