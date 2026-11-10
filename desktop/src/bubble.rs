//! El bocadillo en pixel art: texto en Terminus 6x12, esquinas escalonadas y un piquito
//! abajo a la derecha que apunta al gato.

use crate::font_data::{FONT_H, FONT_W, GLYPHS};

pub const MAX_COLS: usize = 26;
pub const MAX_LINES: usize = 3;

const FILL: [u8; 4] = [247, 247, 242, 255];
const SHADE: [u8; 4] = [206, 208, 220, 255];
const INK: [u8; 4] = [26, 26, 31, 255];
const DIM: [u8; 4] = [104, 106, 124, 255];
const GOLD: [u8; 4] = [238, 196, 60, 255];
const GOLD_D: [u8; 4] = [196, 140, 36, 255];

const PAD_X: usize = 6;
const PAD_Y: usize = 5;
const TAIL: usize = 3;
const ICON_W: usize = 9;

pub struct Bubble {
    pub w: usize,
    pub h: usize,
    /// `w * h` píxeles RGBA, con alfa 0 fuera del bocadillo.
    pub px: Vec<[u8; 4]>,
    /// Punta del piquito, en píxeles del bocadillo.
    pub tip: (usize, usize),
}

/// Pasa el texto a Latin-1 (lo que cubre la fuente) y normaliza los espacios.
fn latin1(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for ch in s.chars() {
        match ch {
            '…' => out.extend_from_slice(b"..."),
            '“' | '”' => out.push(b'"'),
            '‘' | '’' => out.push(b'\''),
            '–' | '—' => out.push(b'-'),
            c if c.is_whitespace() || c.is_control() => {
                if !matches!(out.last(), Some(b' ') | None) {
                    out.push(b' ');
                }
            }
            c if (0x21..=0x7e).contains(&(c as u32)) || (0xa1..=0xff).contains(&(c as u32)) => out.push(c as u32 as u8),
            _ => out.push(b'?'),
        }
    }
    while out.last() == Some(&b' ') {
        out.pop();
    }
    out
}

/// Parte el texto en un máximo de `MAX_LINES` líneas de `MAX_COLS` columnas, por palabras.
/// Si no cabe, la última línea termina en "...".
pub fn wrap(text: &str) -> Vec<Vec<u8>> {
    let bytes = latin1(text);
    let mut lines: Vec<Vec<u8>> = vec![Vec::new()];
    let mut truncated = false;
    'words: for word in bytes.split(|b| *b == b' ').filter(|w| !w.is_empty()) {
        let mut word = word;
        loop {
            let line = lines.last_mut().unwrap();
            let need = if line.is_empty() { word.len() } else { word.len() + 1 };
            if line.len() + need <= MAX_COLS {
                if !line.is_empty() {
                    line.push(b' ');
                }
                line.extend_from_slice(word);
                continue 'words;
            }
            if line.is_empty() {
                // palabra más larga que una línea: se corta
                line.extend_from_slice(&word[..MAX_COLS]);
                word = &word[MAX_COLS..];
            }
            if lines.len() == MAX_LINES {
                truncated = true;
                break 'words;
            }
            lines.push(Vec::new());
        }
    }
    if truncated {
        let last = lines.last_mut().unwrap();
        last.truncate(MAX_COLS - 3);
        while last.last() == Some(&b' ') {
            last.pop();
        }
        last.extend_from_slice(b"...");
    }
    lines.retain(|l| !l.is_empty());
    lines
}

fn fit(text: &str, cols: usize) -> Vec<u8> {
    let mut b = latin1(text);
    if b.len() > cols {
        b.truncate(cols - 3);
        b.extend_from_slice(b"...");
    }
    b
}

impl Bubble {
    pub fn new(message: &str, project: &str) -> Bubble {
        let lines = wrap(message);
        let proj = fit(project, MAX_COLS - 2);
        let proj_w = if proj.is_empty() { 0 } else { ICON_W + 3 + proj.len() * FONT_W };
        let text_w = lines.iter().map(|l| l.len() * FONT_W).max().unwrap_or(0).max(proj_w).max(8 * FONT_W);
        let text_h = lines.len() * FONT_H + if proj.is_empty() { 0 } else { FONT_H + 3 } - if lines.is_empty() && !proj.is_empty() { 3 } else { 0 };
        let w = text_w + 2 * PAD_X;
        let body = text_h + 2 * PAD_Y;
        let h = body + TAIL;

        // silueta: rectángulo con esquinas escalonadas más el piquito
        let mut inside = vec![false; w * h];
        for y in 0..body {
            let cut = match y.min(body - 1 - y) {
                0 => 2,
                1 => 1,
                _ => 0,
            };
            for x in cut..w - cut {
                inside[y * w + x] = true;
            }
        }
        for i in 1..=TAIL {
            for x in w - 10 + 2 * i..=w - 4 {
                inside[(body - 1 + i) * w + x] = true;
            }
        }
        let at = |x: isize, y: isize| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && inside[y as usize * w + x as usize];
        let mut px = vec![[0u8; 4]; w * h];
        for y in 0..h as isize {
            for x in 0..w as isize {
                if !at(x, y) {
                    continue;
                }
                let border = !at(x - 1, y) || !at(x + 1, y) || !at(x, y - 1) || !at(x, y + 1);
                // una línea de sombra sobre el borde inferior da algo de volumen
                let shade = !border && (y as usize) < body && !(at(x, y + 2) && at(x - 1, y + 1) && at(x + 1, y + 1));
                px[y as usize * w + x as usize] = if border { INK } else if shade { SHADE } else { FILL };
            }
        }

        let mut b = Bubble { w, h, px, tip: (w - 4, h - 1) };
        let mut y = PAD_Y;
        for line in &lines {
            b.text(PAD_X, y, line, INK);
            y += FONT_H;
        }
        if !proj.is_empty() {
            if !lines.is_empty() {
                y += 3;
            }
            b.folder(PAD_X, y + 3);
            b.text(PAD_X + ICON_W + 3, y, &proj, DIM);
        }
        b
    }

    fn set(&mut self, x: usize, y: usize, c: [u8; 4]) {
        if x < self.w && y < self.h {
            self.px[y * self.w + x] = c;
        }
    }

    fn text(&mut self, x: usize, y: usize, s: &[u8], color: [u8; 4]) {
        for (i, ch) in s.iter().enumerate() {
            let glyph = &GLYPHS[(*ch as usize).saturating_sub(32).min(223)];
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..FONT_W {
                    if bits & (0x80 >> col) != 0 {
                        self.set(x + i * FONT_W + col, y + row, color);
                    }
                }
            }
        }
    }

    fn folder(&mut self, x: usize, y: usize) {
        const ICON: [&[u8; ICON_W]; 7] = [b".KKK.....", b"KYYYKKKK.", b"KOOOOOOOK", b"KYYYYYYYK", b"KYYYYYYYK", b"KYYYYYYYK", b".KKKKKKK."];
        for (dy, row) in ICON.iter().enumerate() {
            for (dx, c) in row.iter().enumerate() {
                match c {
                    b'K' => self.set(x + dx, y + dy, INK),
                    b'Y' => self.set(x + dx, y + dy, GOLD),
                    b'O' => self.set(x + dx, y + dy, GOLD_D),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(lines: &[Vec<u8>]) -> Vec<String> {
        lines.iter().map(|l| l.iter().map(|b| *b as char).collect()).collect()
    }

    #[test]
    fn una_linea_corta_no_se_parte() {
        assert_eq!(s(&wrap("¿Puedo usar Bash?")), ["¿Puedo usar Bash?"]);
    }

    #[test]
    fn parte_por_palabras() {
        let l = s(&wrap("¿Qué base de datos prefieres para guardar las sesiones?"));
        assert_eq!(l, ["¿Qué base de datos", "prefieres para guardar las", "sesiones?"]);
        assert!(l.iter().all(|x| x.chars().count() <= MAX_COLS));
    }

    #[test]
    fn recorta_con_puntos_suspensivos() {
        let l = s(&wrap(&"palabra ".repeat(40)));
        assert_eq!(l.len(), MAX_LINES);
        assert!(l[2].ends_with("..."));
        assert!(l.iter().all(|x| x.chars().count() <= MAX_COLS));
    }

    #[test]
    fn corta_palabras_larguisimas() {
        let l = s(&wrap(&"x".repeat(60)));
        assert_eq!(l[0].len(), MAX_COLS);
        assert!(l.len() <= MAX_LINES);
    }

    #[test]
    fn caracteres_fuera_de_latin1() {
        assert_eq!(s(&wrap("hola 🐱 —  fin…\n\tya")), ["hola ? - fin... ya"]);
    }

    #[test]
    fn el_bocadillo_tiene_borde_cerrado_y_punta() {
        let b = Bubble::new("¿Puedo usar Bash?", "notify-claude");
        assert_eq!(b.px.len(), b.w * b.h);
        assert_eq!(b.px[b.tip.1 * b.w + b.tip.0], INK);
        assert_eq!(b.px[0][3], 0, "la esquina es transparente");
        let vacio = Bubble::new("", "");
        assert!(vacio.w > 0 && vacio.h > TAIL);
    }
}
