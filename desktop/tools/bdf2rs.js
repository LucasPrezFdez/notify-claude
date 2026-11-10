// Convierte una fuente BDF de Terminus en una tabla Rust con los glifos de Latin-1 (0x20-0xFF).
// Uso: node bdf2rs.js <ruta/ter-u12n.bdf> > ../src/font_data.rs
const fs = require('fs');
const src = fs.readFileSync(process.argv[2], 'latin1').split(/\r?\n/);
let W = 0, H = 0;
const glyphs = new Map();
for (let i = 0; i < src.length; i++) {
  const l = src[i];
  if (l.startsWith('FONTBOUNDINGBOX')) { const p = l.split(' '); W = +p[1]; H = +p[2]; }
  if (!l.startsWith('ENCODING')) continue;
  const code = +l.split(' ')[1];
  while (!src[i].startsWith('BITMAP')) i++;
  const rows = [];
  for (i++; !src[i].startsWith('ENDCHAR'); i++) rows.push(parseInt(src[i].slice(0, 2), 16));
  if (code >= 32 && code <= 255) glyphs.set(code, rows);
}
if (W > 8) throw new Error('solo se admiten fuentes de hasta 8 px de ancho');
const out = [];
out.push('// Generado por tools/bdf2rs.js a partir de Terminus Font (SIL OFL 1.1). No editar a mano.');
out.push(`pub const FONT_W: usize = ${W};`);
out.push(`pub const FONT_H: usize = ${H};`);
out.push('/// Glifos de 0x20 a 0xFF; cada fila es un byte con el bit alto a la izquierda.');
out.push(`pub static GLYPHS: [[u8; ${H}]; 224] = [`);
for (let c = 32; c <= 255; c++) {
  const r = glyphs.get(c) || Array(H).fill(0);
  out.push('    [' + r.map(v => '0x' + v.toString(16).padStart(2, '0')).join(', ') + '],');
}
out.push('];');
process.stdout.write(out.join('\n') + '\n');
