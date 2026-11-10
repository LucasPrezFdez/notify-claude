const zlib = require('zlib'), fs = require('fs'), path = require('path');
const { GW, GH, FPS, ANIMS, STAGE, frameAt } = require('./gato.js');
const OUT = path.join(__dirname, 'salida'); fs.mkdirSync(OUT, { recursive: true });

function png(file, W, H, raw) {
  const chunk = (t, d) => { const b = Buffer.concat([Buffer.from(t), d]); const o = Buffer.alloc(8 + b.length); o.writeUInt32BE(d.length, 0); b.copy(o, 4); o.writeUInt32BE(zlib.crc32(b) >>> 0, 4 + b.length); return o; };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4); ihdr[8] = 8; ihdr[9] = 2;
  fs.writeFileSync(path.join(OUT, file), Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
}
// tira de fotogramas: cells = [[anim, t], ...]
function strip(file, cells, cols, S) {
  const rows = Math.ceil(cells.length / cols), W = STAGE.W * cols * S, H = STAGE.H * rows * S, raw = Buffer.alloc((W * 3 + 1) * H);
  cells.forEach(([name, t], i) => {
    const { px, dy } = frameAt(name, t), cx = (i % cols) * STAGE.W, cy = Math.floor(i / cols) * STAGE.H, bg = (i + Math.floor(i / cols)) % 2 ? [58, 110, 165] : [52, 100, 152];
    for (let y = 0; y < STAGE.H; y++) for (let x = 0; x < STAGE.W; x++) {
      let c = bg;
      if (y >= STAGE.EDGE) c = [27, 31, 42]; else { const v = (px[y - STAGE.TOP - dy] || [])[x - 2]; if (v) c = v; }
      for (let a = 0; a < S; a++) for (let b = 0; b < S; b++) raw.set(c, ((cy + y) * S + a) * (W * 3 + 1) + 1 + ((cx + x) * S + b) * 3);
    }
  });
  png(file, W, H, raw);
}
const sample = (name, n, t0 = 0, t1 = ANIMS[name].dur) => Array.from({ length: n }, (_, i) => [name, t0 + (t1 - t0) * i / (n - 1)]);

const which = process.argv[2] || 'all';
if (which === 'all' || which === 'entrada') strip('tira-entrada.png', sample('entrada', 24, 0.1, 3.0), 8, 3);
if (which === 'all' || which === 'poses') strip('tira-poses.png', [...sample('pregunta', 8, 0, 1.4), ...sample('permiso', 8, 0, 1.7), ...sample('salida', 8, 0.1, 1.7)], 8, 3);
if (which === 'all' || which === 'reposo') strip('tira-reposo.png', sample('reposo', 16, 0, 6.2), 8, 3);

// comprobar todas las animaciones a 30 fps, sin excepciones
let n = 0; for (const name in ANIMS) for (let t = 0; t < ANIMS[name].dur; t += 1 / FPS) { frameAt(name, t); n++; }
// vista previa: el mismo código, incrustado
const html = fs.readFileSync(path.join(__dirname, 'plantilla.html'), 'utf8').replace('/*CORE*/', () => fs.readFileSync(path.join(__dirname, 'gato.js'), 'utf8'));
fs.writeFileSync(path.join(OUT, 'animaciones.html'), html);
console.log('fotogramas generados sin errores:', n, 'a', FPS, 'fps');

// paridad con la app en Rust: un hash por fotograma (t = i / FPS), que desktop/ comprueba en sus pruebas
const lines = [];
for (const name in ANIMS) for (let i = 0; i / FPS < ANIMS[name].dur; i++) {
  const { px, dy } = frameAt(name, i / FPS);
  let h = 0x811c9dc5;
  for (const row of px) for (const c of row) for (const v of c ? [c[0], c[1], c[2], 255] : [0, 0, 0, 0]) { h ^= v; h = Math.imul(h, 0x01000193) >>> 0; }
  lines.push([name, i, dy, h.toString(16).padStart(8, '0')].join(' '));
}
fs.writeFileSync(path.join(__dirname, 'paridad.txt'), lines.join('\n') + '\n');
