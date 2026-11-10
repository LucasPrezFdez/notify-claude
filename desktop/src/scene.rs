//! La escena: qué animación toca en cada momento (`Director`) y cómo se componen el gato
//! y el bocadillo en el búfer de la ventana (`Layout`, `compose`). Sin nada de Win32.

use crate::anim::{frame_at, Anim};
use crate::bubble::Bubble;
use crate::cat::{Frame, Pose, GH, GW, OY};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Pregunta,
    Permiso,
}

impl Kind {
    fn anim(self) -> Anim {
        match self {
            Kind::Pregunta => Anim::Pregunta,
            Kind::Permiso => Anim::Permiso,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Entrada,
    Bucle,
    Salida,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pending {
    Salir,
    Cambiar(Kind),
}

/// Momento de la entrada en que el gato ya ha aterrizado y puede hablar.
const BUBBLE_FROM: f64 = 2.6;

pub struct Director {
    phase: Phase,
    kind: Kind,
    pending: Option<Pending>,
    t: f64,
    tg: f64,
    bubble_age: f64,
}

impl Director {
    pub fn new(kind: Kind) -> Director {
        Director { phase: Phase::Entrada, kind, pending: None, t: 0.0, tg: 0.0, bubble_age: 0.0 }
    }

    fn anim(&self) -> Anim {
        match self.phase {
            Phase::Entrada => Anim::Entrada,
            Phase::Bucle => self.kind.anim(),
            Phase::Salida => Anim::Salida,
        }
    }

    /// Avanza `dt` segundos. Devuelve `false` cuando el gato ya se ha ido y hay que cerrar.
    pub fn advance(&mut self, dt: f64) -> bool {
        self.t += dt;
        self.tg += dt;
        while self.t >= self.anim().dur() {
            self.t -= self.anim().dur();
            match (self.phase, self.pending.take()) {
                (Phase::Entrada, Some(Pending::Salir)) | (Phase::Bucle, Some(Pending::Salir)) => self.phase = Phase::Salida,
                (Phase::Entrada, Some(Pending::Cambiar(k))) | (Phase::Bucle, Some(Pending::Cambiar(k))) => {
                    self.kind = k;
                    self.phase = Phase::Bucle;
                }
                (Phase::Entrada, None) => self.phase = Phase::Bucle,
                (Phase::Bucle, None) => {}
                (Phase::Salida, Some(Pending::Cambiar(k))) => {
                    self.kind = k;
                    self.phase = Phase::Entrada;
                    self.t = 0.0;
                }
                (Phase::Salida, _) => return false,
            }
        }
        self.bubble_age = if self.bubble_visible() { self.bubble_age + dt } else { 0.0 };
        true
    }

    /// El aviso se ha atendido (clic o `--dismiss`): baja la pata, guiña y se va.
    pub fn dismiss(&mut self) {
        match self.phase {
            Phase::Entrada => self.pending = Some(Pending::Salir),
            Phase::Bucle => {
                self.pending = Some(Pending::Salir);
                self.t = self.t.max(self.anim().wind_down());
            }
            Phase::Salida => self.pending = None,
        }
    }

    /// Llega un aviso nuevo, que sustituye al que se esté viendo.
    pub fn alert(&mut self, kind: Kind) {
        self.bubble_age = 0.0;
        match self.phase {
            Phase::Entrada => {
                self.kind = kind;
                self.pending = None;
            }
            Phase::Bucle if kind == self.kind => self.pending = None,
            Phase::Bucle => {
                self.pending = Some(Pending::Cambiar(kind));
                self.t = self.t.max(self.anim().wind_down());
            }
            Phase::Salida => self.pending = Some(Pending::Cambiar(kind)),
        }
    }

    pub fn bubble_visible(&self) -> bool {
        match self.phase {
            Phase::Entrada => self.t >= BUBBLE_FROM && self.pending != Some(Pending::Salir),
            Phase::Bucle => self.pending != Some(Pending::Salir),
            Phase::Salida => false,
        }
    }

    /// Desplazamiento del bocadillo, en sus píxeles, para el pequeño rebote al aparecer.
    pub fn bubble_offset(&self) -> i32 {
        if self.bubble_age < 0.07 {
            2
        } else if self.bubble_age < 0.14 {
            -1
        } else {
            0
        }
    }

    pub fn frame(&self) -> (Pose, i32) {
        frame_at(self.anim(), self.t, self.tg)
    }
}

/// Margen, en píxeles del sprite, por encima del gato sentado: lo que sube en el salto.
const TOP: i32 = 12;
/// Fila de la rejilla del sprite que coincide con el borde de la barra de tareas.
const EDGE: i32 = OY + 48;
/// Punto del sprite al que apunta el piquito del bocadillo: junto a la oreja izquierda.
const TIP_AT: (i32, i32) = (6, OY + 5);

/// Geometría de la ventana en píxeles físicos. El borde inferior de la ventana es el
/// borde inferior del área de trabajo, así que lo que baja de ahí queda recortado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub w: i32,
    pub h: i32,
    /// Tamaño en pantalla de un píxel del gato y de uno del bocadillo.
    pub s: i32,
    pub sb: i32,
    pub cat_x: i32,
    pub cat_y: i32,
    pub bub_x: i32,
    pub bub_y: i32,
}

impl Layout {
    pub fn new(bubble: &Bubble, s: i32, sb: i32) -> Layout {
        let (tx, ty) = (bubble.tip.0 as i32 * sb, bubble.tip.1 as i32 * sb);
        let cat_x = (tx - TIP_AT.0 * s).max(0);
        let h = ((TOP + EDGE) * s).max((EDGE - TIP_AT.1) * s + ty + sb * 3);
        let cat_y = h - EDGE * s;
        Layout { w: cat_x + GW as i32 * s, h, s, sb, cat_x, cat_y, bub_x: cat_x + TIP_AT.0 * s - tx, bub_y: cat_y + TIP_AT.1 * s - ty }
    }
}

/// Escalas enteras para un monitor con ese DPI: el gato a 3x y el bocadillo a 2x al 100 %.
pub fn scales(dpi: u32) -> (i32, i32) {
    let k = dpi as f64 / 96.0;
    (((3.0 * k) + 0.5).floor().max(2.0) as i32, ((2.0 * k) + 0.5).floor().max(1.0) as i32)
}

fn bgra(c: [u8; 4]) -> u32 {
    // opaco, así que premultiplicar no cambia nada
    (c[3] as u32) << 24 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

fn block(buf: &mut [u32], lay: &Layout, x: i32, y: i32, size: i32, color: u32) {
    for py in y.max(0)..(y + size).min(lay.h) {
        let row = (py * lay.w) as usize;
        for px in x.max(0)..(x + size).min(lay.w) {
            buf[row + px as usize] = color;
        }
    }
}

/// Dibuja el fotograma completo en `buf` (`w * h` píxeles BGRA premultiplicados).
pub fn compose(buf: &mut [u32], lay: &Layout, frame: &Frame, dy: i32, bubble: Option<(&Bubble, i32)>) {
    buf.fill(0);
    if let Some((b, off)) = bubble {
        for y in 0..b.h {
            for x in 0..b.w {
                let c = b.px[y * b.w + x];
                if c[3] != 0 {
                    block(buf, lay, lay.bub_x + x as i32 * lay.sb, lay.bub_y + (y as i32 + off) * lay.sb, lay.sb, bgra(c));
                }
            }
        }
    }
    for y in 0..GH {
        for x in 0..GW {
            let c = frame[y * GW + x];
            if c[3] != 0 {
                block(buf, lay, lay.cat_x + x as i32 * lay.s, lay.cat_y + (y as i32 + dy) * lay.s, lay.s, bgra(c));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::FPS;
    use crate::cat::draw_cat;

    fn run(d: &mut Director, secs: f64) -> bool {
        (0..(secs * FPS) as usize).all(|_| d.advance(1.0 / FPS))
    }

    #[test]
    fn sin_atender_no_se_va_nunca() {
        let mut d = Director::new(Kind::Pregunta);
        assert!(run(&mut d, 120.0));
        assert!(d.bubble_visible());
    }

    #[test]
    fn el_bocadillo_sale_al_aterrizar_y_se_quita_al_pulsar() {
        let mut d = Director::new(Kind::Permiso);
        run(&mut d, 2.0);
        assert!(!d.bubble_visible());
        run(&mut d, 2.0);
        assert!(d.bubble_visible());
        d.dismiss();
        assert!(!d.bubble_visible());
        assert!(!run(&mut d, 4.0), "tras atender el aviso, el gato se va");
    }

    #[test]
    fn atender_durante_la_entrada_espera_a_que_aterrice() {
        let mut d = Director::new(Kind::Pregunta);
        run(&mut d, 0.5);
        d.dismiss();
        assert!(run(&mut d, 3.0));
        assert!(!run(&mut d, 3.0));
    }

    #[test]
    fn un_aviso_nuevo_cancela_la_salida_y_cambia_de_gesto() {
        let mut d = Director::new(Kind::Pregunta);
        run(&mut d, 5.0);
        d.dismiss();
        d.alert(Kind::Permiso);
        assert!(run(&mut d, 60.0), "el aviso nuevo sigue pendiente");
        assert_eq!(d.kind, Kind::Permiso);
        assert!(d.bubble_visible());
    }

    #[test]
    fn un_aviso_durante_la_salida_hace_que_vuelva_a_entrar() {
        let mut d = Director::new(Kind::Pregunta);
        run(&mut d, 5.0);
        d.dismiss();
        run(&mut d, 1.0);
        assert_eq!(d.phase, Phase::Salida);
        d.alert(Kind::Permiso);
        assert!(run(&mut d, 30.0));
        assert_eq!((d.phase, d.kind), (Phase::Bucle, Kind::Permiso));
    }

    #[test]
    fn escalas_enteras_por_dpi() {
        assert_eq!(scales(96), (3, 2));
        assert_eq!(scales(120), (4, 3));
        assert_eq!(scales(144), (5, 3));
        assert_eq!(scales(192), (6, 4));
    }

    #[test]
    fn el_bocadillo_cabe_entero_en_la_ventana() {
        for (msg, proj) in [("¿Puedo usar Bash?", "notify-claude"), ("", ""), (&*"palabra ".repeat(40), &*"x".repeat(80))] {
            for dpi in [96, 120, 144, 192] {
                let b = Bubble::new(msg, proj);
                let (s, sb) = scales(dpi);
                let l = Layout::new(&b, s, sb);
                assert!(l.bub_x >= 0 && l.bub_y >= sb * 2, "{l:?}");
                assert!(l.bub_x + b.w as i32 * sb <= l.w && l.bub_y + b.h as i32 * sb <= l.h, "{l:?}");
            }
        }
    }

    // ---- imagen de revisión: se escribe en target/ para mirarla, no afirma nada ----

    fn png(path: &std::path::Path, w: u32, h: u32, rgb: &[u8]) {
        fn crc(data: &[u8]) -> u32 {
            let mut c = !0u32;
            for b in data {
                c ^= *b as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
                }
            }
            !c
        }
        fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let mut body = tag.to_vec();
            body.extend_from_slice(data);
            out.extend_from_slice(&body);
            out.extend_from_slice(&crc(&body).to_be_bytes());
        }
        let mut raw = Vec::new();
        for row in rgb.chunks(w as usize * 3) {
            raw.push(0);
            raw.extend_from_slice(row);
        }
        // zlib sin comprimir: bloques "stored" de hasta 65535 bytes
        let mut z = vec![0x78, 0x01];
        let n = raw.chunks(65535).count();
        for (i, part) in raw.chunks(65535).enumerate() {
            z.push((i + 1 == n) as u8);
            z.extend_from_slice(&(part.len() as u16).to_le_bytes());
            z.extend_from_slice(&(!(part.len() as u16)).to_le_bytes());
            z.extend_from_slice(part);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for v in &raw {
            a = (a + *v as u32) % 65521;
            b = (b + a) % 65521;
        }
        z.extend_from_slice(&(b << 16 | a).to_be_bytes());
        let mut out = vec![137, 80, 78, 71, 13, 10, 26, 10];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &z);
        chunk(&mut out, b"IEND", &[]);
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn imagen_de_revision() {
        // (tipo, mensaje, proyecto, segundos transcurridos)
        let shots = [
            (Kind::Pregunta, "¿Qué base de datos prefieres para guardar las sesiones?", "notify-claude", 0.8),
            (Kind::Pregunta, "¿Qué base de datos prefieres para guardar las sesiones?", "notify-claude", 2.2),
            (Kind::Pregunta, "¿Qué base de datos prefieres para guardar las sesiones?", "notify-claude", 4.6),
            (Kind::Permiso, "¿Puedo usar Bash?", "DesktopFriend", 5.6),
        ];
        let (s, sb) = scales(96);
        let cells: Vec<(Layout, Vec<u32>)> = shots
            .iter()
            .map(|(kind, msg, proj, secs)| {
                let b = Bubble::new(msg, proj);
                let lay = Layout::new(&b, s, sb);
                let mut d = Director::new(*kind);
                run(&mut d, *secs);
                let (pose, dy) = d.frame();
                let mut buf = vec![0u32; (lay.w * lay.h) as usize];
                compose(&mut buf, &lay, &draw_cat(&pose), dy, d.bubble_visible().then(|| (&b, d.bubble_offset())));
                (lay, buf)
            })
            .collect();
        let (bar, gap) = (24, 16);
        let w: i32 = cells.iter().map(|(l, _)| l.w + gap).sum();
        let h: i32 = cells.iter().map(|(l, _)| l.h).max().unwrap() + bar + gap;
        let mut rgb = vec![0u8; (w * h * 3) as usize];
        for (i, px) in rgb.chunks_mut(3).enumerate() {
            let y = i as i32 / w;
            px.copy_from_slice(if y >= h - bar { &[27, 31, 42] } else { &[58, 110, 165] });
        }
        let mut x0 = gap / 2;
        for (lay, buf) in &cells {
            for y in 0..lay.h {
                for x in 0..lay.w {
                    let v = buf[(y * lay.w + x) as usize];
                    if v >> 24 != 0 {
                        let o = (((h - bar - lay.h + y) * w + x0 + x) * 3) as usize;
                        rgb[o..o + 3].copy_from_slice(&[(v >> 16) as u8, (v >> 8) as u8, v as u8]);
                    }
                }
            }
            x0 += lay.w + gap;
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        std::fs::create_dir_all(&dir).unwrap();
        png(&dir.join("revision.png"), w as u32, h as u32, &rgb);
    }
}
