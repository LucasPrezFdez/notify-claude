//! El gato: una pose continua se convierte en una rejilla de píxeles.
//!
//! Es un port literal de `sprites/gato.js`, que es la referencia del dibujo. Las pruebas
//! comparan cada fotograma con `sprites/paridad.txt`; si se cambia uno, hay que cambiar el otro.

pub const GW: usize = 58;
pub const GH: usize = 59;
pub const OY: i32 = 10;
const CX: f64 = 23.5;
const FLOOR: i32 = 46;

/// `Math.round` de JavaScript (los medios redondean hacia +infinito).
pub fn jsr(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Tail,
    Body,
    HaunchL,
    HaunchR,
    FootL,
    FootR,
    LegL,
    LegR,
    Head,
    Eye,
    Whisk,
    ArmL,
    ArmR,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mat {
    K,
    W,
    P,
    Y,
    N,
    S,
    G,
    L,
    M,
}

#[derive(Clone, Copy)]
struct Cell {
    part: Part,
    mat: Mat,
    shade: i8,
    sep: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Paw {
    pub lift: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// Aplastamiento (positivo) o estiramiento (negativo) del cuerpo.
    pub b: f64,
    /// Respiración, de 0 a 1.
    pub br: f64,
    /// Desplazamiento lateral de la cabeza.
    pub hx: f64,
    pub paw_l: Option<Paw>,
    pub paw_r: Option<Paw>,
    pub eye_l: f64,
    pub eye_r: f64,
    /// Ojo cerrado sonriendo (un arco hacia arriba) en lugar de una raya.
    pub happy_l: bool,
    pub happy_r: bool,
    pub look: f64,
    pub pupil: f64,
    pub ear_l: [f64; 2],
    pub ear_r: [f64; 2],
    pub tail_a: f64,
    pub tail_tb: f64,
    pub tail_up: f64,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            b: 0.0,
            br: 0.0,
            hx: 0.0,
            paw_l: None,
            paw_r: None,
            eye_l: 1.0,
            eye_r: 1.0,
            happy_l: false,
            happy_r: false,
            look: 0.0,
            pupil: 1.1,
            ear_l: [0.0, 0.0],
            ear_r: [0.0, 0.0],
            tail_a: 0.0,
            tail_tb: 0.0,
            tail_up: 0.0,
        }
    }
}

/// Un fotograma: `GW * GH` píxeles RGBA, con alfa 0 en los transparentes.
pub type Frame = Vec<[u8; 4]>;

#[derive(Default, Clone, Copy)]
struct Opt {
    max_y: Option<i32>,
    only_head: bool,
    flat: bool,
}

struct Canvas {
    g: Vec<Option<Cell>>,
    ox: i32,
    oy: i32,
}

fn band(n: f64) -> i8 {
    if n > 0.55 {
        1
    } else if n < -0.6 {
        -1
    } else {
        0
    }
}

impl Canvas {
    fn put(&mut self, x: i32, y: i32, part: Part, mat: Mat, shade: i8) {
        let (gx, gy) = (x + self.ox, y + self.oy + OY);
        if gx >= 0 && gy >= 0 && (gx as usize) < GW && (gy as usize) < GH {
            self.g[gy as usize * GW + gx as usize] = Some(Cell { part, mat, shade, sep: false });
        }
    }
    fn get(&self, x: i32, y: i32) -> Option<Cell> {
        self.at(x + self.ox, y + self.oy + OY)
    }
    fn at(&self, gx: i32, gy: i32) -> Option<Cell> {
        if gx >= 0 && gy >= 0 && (gx as usize) < GW && (gy as usize) < GH {
            self.g[gy as usize * GW + gx as usize]
        } else {
            None
        }
    }
    fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, part: Part, mat: Mat, opt: Opt) {
        let (y0, y1) = (-OY - 4, GH as i32 - OY + 4);
        let ya = y0.max((cy - ry).floor() as i32);
        let yb = y1.min((cy + ry).ceil() as i32);
        for y in ya..=yb {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (u, v) = ((x as f64 - cx) / rx, (y as f64 - cy) / ry);
                if u * u + v * v > 1.0 {
                    continue;
                }
                if matches!(opt.max_y, Some(m) if y > m) {
                    continue;
                }
                if opt.only_head && !matches!(self.get(x, y), Some(c) if c.part == Part::Head) {
                    continue;
                }
                let shade = if opt.flat { 0 } else { band(-0.6 * u - 0.8 * v) };
                self.put(x, y, part, mat, shade);
            }
        }
    }
    fn tri(&mut self, p: [[f64; 2]; 3], part: Part, mat: Mat, shade: i8) {
        let s = |a: [f64; 2], c: [f64; 2], x: f64, y: f64| (c[0] - a[0]) * (y - a[1]) - (c[1] - a[1]) * (x - a[0]);
        let min = |i: usize| p.iter().map(|v| v[i]).fold(f64::INFINITY, f64::min).floor() as i32;
        let max = |i: usize| p.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
        for y in min(1)..=max(1) {
            for x in min(0)..=max(0) {
                let (xf, yf) = (x as f64, y as f64);
                let d = [s(p[0], p[1], xf, yf), s(p[1], p[2], xf, yf), s(p[2], p[0], xf, yf)];
                if d.iter().all(|v| *v >= 0.0) || d.iter().all(|v| *v <= 0.0) {
                    self.put(x, y, part, mat, shade);
                }
            }
        }
    }
    fn capsule(&mut self, ax: f64, ay: f64, bx: f64, by: f64, r: f64, part: Part, mat: Mat) {
        for i in 0..=40 {
            let t = i as f64 / 40.0;
            let (cx, cy) = (lerp(ax, bx, t), lerp(ay, by, t));
            for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
                for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                    let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                    if dx * dx + dy * dy <= r * r {
                        self.put(x, y, part, mat, band((-0.6 * dx - 0.8 * dy) / r));
                    }
                }
            }
        }
    }
}

/// Lado del gato visto de frente: `m` refleja la coordenada x respecto al eje del cuerpo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    L,
    R,
}
impl Side {
    fn m(self, x: f64) -> f64 {
        match self {
            Side::L => x,
            Side::R => 2.0 * CX - x,
        }
    }
}
const SIDES: [Side; 2] = [Side::L, Side::R];

pub fn draw_cat(p: &Pose) -> Frame {
    let mut c = Canvas { g: vec![None; GW * GH], ox: 0, oy: 0 };
    let b = p.b;
    let hd = jsr(b * 2.0);
    let hx = jsr(p.hx);
    let hdf = hd as f64;
    let paw = |s: Side| if s == Side::L { p.paw_l } else { p.paw_r };
    let lifted = |s: Side| matches!(paw(s), Some(w) if w.lift > 0.02);

    // cola
    let (ta, tb) = (p.tail_a, p.tail_tb);
    let a_pts = [[33.0, 43.0], [50.0 + ta, 46.0], [49.0 + 1.5 * ta + 1.5 * tb, 31.0 + tb.abs() * 0.5], [42.0 + 4.0 * tb + ta, 27.0]];
    let u_pts = [[33.0, 43.0], [46.0, 46.0], [47.0, 32.0], [46.0, 14.0]];
    let mut pt = [[0.0f64; 2]; 4];
    for i in 0..4 {
        pt[i] = [lerp(a_pts[i][0], u_pts[i][0], p.tail_up), lerp(a_pts[i][1], u_pts[i][1], p.tail_up)];
    }
    for i in 0..=220 {
        let t = i as f64 / 220.0;
        let a = 1.0 - t;
        let bez = |k: usize| a * a * a * pt[0][k] + 3.0 * a * a * t * pt[1][k] + 3.0 * a * t * t * pt[2][k] + t * t * t * pt[3][k];
        let (bx, by) = (bez(0), bez(1));
        let r = 2.9 - 0.7 * t;
        for y in (by - r).floor() as i32..=(by + r).ceil() as i32 {
            for x in (bx - r).floor() as i32..=(bx + r).ceil() as i32 {
                let (dx, dy) = (x as f64 - bx, y as f64 - by);
                if dx * dx + dy * dy > r * r || y > FLOOR {
                    continue;
                }
                if t > 0.8 {
                    c.put(x, y, Part::Tail, Mat::W, if dx > r - 1.3 { -1 } else { 0 });
                } else {
                    c.put(x, y, Part::Tail, Mat::K, band((-0.6 * dx - 0.8 * dy) / r));
                }
            }
        }
    }

    // cuerpo y pecho
    let floor = Opt { max_y: Some(FLOOR), ..Opt::default() };
    c.ellipse(CX, 35.0 + b, 11.5 + b * 0.5 + p.br * 0.6, 13.0 - b, Part::Body, Mat::K, floor);
    for y in 22 + hd..=42 {
        let rx = 6.2 - (0.0f64).max(y as f64 - 33.0) * 0.55 + if y % 3 == 0 { 1.0 } else { 0.0 } + p.br * 0.5;
        for x in 0..GW as i32 {
            let d = x as f64 - CX;
            if d.abs() <= rx && matches!(c.get(x, y), Some(cell) if cell.part == Part::Body) {
                c.put(x, y, Part::Body, Mat::W, if d > rx - 2.0 { -1 } else { 0 });
            }
        }
    }
    // ancas y patas traseras
    for s in SIDES {
        let part = if s == Side::L { Part::HaunchL } else { Part::HaunchR };
        c.ellipse(s.m(12.5), 38.5 + b * 0.5, 6.0 + b * 0.5, 8.0 - b * 0.5, part, Mat::K, floor);
    }
    for s in SIDES {
        let part = if s == Side::L { Part::FootL } else { Part::FootR };
        c.ellipse(s.m(11.5), 44.6, 4.6, 2.4, part, Mat::W, floor);
    }
    for s in SIDES {
        let part = if s == Side::L { Part::FootL } else { Part::FootR };
        for x in [10.0, 13.0] {
            c.put(jsr(s.m(x)), 45, part, Mat::M, 0);
        }
    }
    // patas delanteras apoyadas
    for s in SIDES {
        if lifted(s) {
            continue;
        }
        let part = if s == Side::L { Part::LegL } else { Part::LegR };
        let cx = s.m(20.0);
        for y in 33..=FLOOR {
            for x in 0..GW as i32 {
                let d = x as f64 - cx;
                if d.abs() <= 2.6 {
                    c.put(x, y, part, Mat::K, if d < -1.5 { 1 } else if d > 1.5 { -1 } else { 0 });
                }
            }
        }
        c.ellipse(cx, 44.4, 3.6, 2.6, part, Mat::W, floor);
        for x in [19.0, 21.0] {
            c.put(jsr(s.m(x)), 45, part, Mat::M, 0);
        }
    }

    // cabeza (se desplaza entera)
    c.ox = hx;
    c.oy = hd;
    for s in SIDES {
        let e = if s == Side::L { p.ear_l } else { p.ear_r };
        let tip = [s.m(12.8 - e[0] * 0.8), 4.5 + e[1] * 0.8];
        c.tri([[s.m(9.5), 13.0], [s.m(11.5 - e[0]), 1.0 + e[1]], [s.m(20.5), 7.0]], Part::Head, Mat::K, 0);
        c.tri([[s.m(12.0), 10.5], tip, [s.m(17.0), 8.2]], Part::Head, Mat::P, 0);
        c.tri([[s.m(12.0), 10.5], tip, [s.m(13.6), 9.5]], Part::Head, Mat::P, -1);
    }
    c.ellipse(CX, 15.5, 13.2, 10.6, Part::Head, Mat::K, Opt::default());
    for s in SIDES {
        c.ellipse(s.m(12.0), 19.5, 4.2, 4.2, Part::Head, Mat::K, Opt { flat: true, ..Opt::default() });
    }
    for s in SIDES {
        for (x, y) in [(7.0, 20), (7.0, 22), (8.0, 23), (10.0, 24)] {
            c.put(jsr(s.m(x)), y, Part::Head, Mat::K, 0);
        }
    }
    for y in 7..=18 {
        let w = 0.6 + (y as f64 - 7.0) * 0.33;
        for x in 0..GW as i32 {
            if (x as f64 - CX).abs() <= w {
                c.put(x, y, Part::Head, Mat::W, 0);
            }
        }
    }
    c.ellipse(CX, 20.6, 6.6, 4.3, Part::Head, Mat::W, Opt { only_head: true, ..Opt::default() });
    // ojos
    for s in SIDES {
        let open = if s == Side::L { p.eye_l } else { p.eye_r };
        let happy = if s == Side::R { p.happy_r } else { p.happy_l };
        let (ex, ey) = (s.m(16.5), 15.5);
        let x0 = ex.floor() as i32 - 2;
        if open < 0.15 {
            for i in 0..6 {
                let mid = i > 0 && i < 5;
                let dy = if happy { if mid { -1 } else { 0 } } else if mid { 1 } else { 0 };
                c.put(x0 + i, 16 + dy, Part::Eye, Mat::L, 0);
            }
            continue;
        }
        let cut_top = 13 + jsr((1.0 - open) * 3.0);
        let cut_bot = 18 - jsr((1.0 - open) * 1.4);
        let pcx = ex + jsr(p.look) as f64;
        for y in cut_top..=cut_bot {
            for x in x0 - 2..=x0 + 7 {
                let (u, v) = ((x as f64 - ex) / 3.1, (y as f64 - ey) / 2.9);
                if u * u + v * v > 1.0 {
                    continue;
                }
                let (pu, pv) = ((x as f64 - pcx) / p.pupil, (y as f64 - ey) / 2.7);
                let mat = if pu * pu + pv * pv <= 1.0 { Mat::N } else { Mat::Y };
                c.put(x, y, Part::Eye, mat, if y == cut_top { -1 } else if y >= 17 { 1 } else { 0 });
            }
        }
        if open > 0.7 {
            c.put(ex.floor() as i32 - 1, 14, Part::Eye, Mat::S, 0);
        }
    }
    // nariz, boca y bigotes
    for (x, y) in [(22, 18), (23, 18), (24, 18), (25, 18), (23, 19), (24, 19)] {
        c.put(x, y, Part::Head, Mat::P, if y == 19 { -1 } else { 0 });
    }
    for (x, y) in [(23, 20), (24, 20), (22, 21), (21, 21), (20, 20), (25, 21), (26, 21), (27, 20), (23, 23), (24, 23)] {
        c.put(x, y, Part::Head, Mat::M, 0);
    }
    for s in SIDES {
        for i in 0..8 {
            let (x, f) = (jsr(s.m(15.0 - i as f64)), i as f64);
            c.put(x, 19 - jsr(f * 0.3), Part::Whisk, Mat::G, 0);
            c.put(x, 21 + jsr(f * 0.12), Part::Whisk, Mat::G, 0);
            if i < 6 {
                c.put(x, 23 + jsr(f * 0.3), Part::Whisk, Mat::G, 0);
            }
        }
    }
    c.ox = 0;
    c.oy = 0;

    // patas delanteras levantadas (por delante de todo)
    for s in SIDES {
        if !lifted(s) {
            continue;
        }
        let w = paw(s).unwrap();
        let k = w.lift.min(1.0);
        let part = if s == Side::L { Part::ArmL } else { Part::ArmR };
        let px = jsr(lerp(s.m(20.0), s.m(w.x), k) * 2.0) as f64 / 2.0;
        let py = jsr(lerp(44.4, w.y, k) * 2.0) as f64 / 2.0;
        c.capsule(s.m(19.0), 34.0 + hdf, px, py, 2.4, part, Mat::K);
        let opt = if k < 0.3 { floor } else { Opt::default() };
        c.ellipse(px, py, lerp(3.6, 3.3, k), lerp(2.6, 3.0, k), part, Mat::W, opt);
        if k > 0.6 {
            for d in [-1.0, 1.0] {
                c.put(jsr(px + d - 0.01), jsr(py) - 2, part, Mat::M, 0);
            }
        }
    }

    // sombra bajo la barbilla
    for y in 0..GH as i32 {
        for x in 0..GW as i32 {
            let Some(cell) = c.at(x, y) else { continue };
            if matches!(cell.part, Part::Head | Part::Whisk | Part::Eye | Part::ArmL | Part::ArmR) {
                continue;
            }
            if (1..=2).any(|d| matches!(c.at(x, y - d), Some(n) if n.part == Part::Head)) {
                c.g[y as usize * GW + x as usize].as_mut().unwrap().shade = -1;
            }
        }
    }
    // separadores entre partes del mismo color
    for y in 0..GH as i32 {
        for x in 0..GW as i32 {
            let Some(cell) = c.at(x, y) else { continue };
            if !matches!(cell.mat, Mat::K | Mat::W) || !matches!(cell.part, Part::Body | Part::Tail | Part::HaunchL | Part::HaunchR | Part::Head) {
                continue;
            }
            let hit = [(1, 0), (0, 1), (-1, 0), (0, -1)].iter().any(|(dx, dy)| {
                matches!(c.at(x + dx, y + dy), Some(n) if n.mat == cell.mat
                    && n.part != cell.part
                    && !matches!(n.part, Part::Whisk | Part::Eye)
                    && (cell.part != Part::Head || matches!(n.part, Part::ArmL | Part::ArmR)))
            });
            if hit {
                c.g[y as usize * GW + x as usize].as_mut().unwrap().sep = true;
            }
        }
    }

    // colores y contorno
    let mut out = vec![[0u8; 4]; GW * GH];
    for y in 0..GH as i32 {
        for x in 0..GW as i32 {
            let rgb = match c.at(x, y) {
                Some(cell) => Some(color(&cell)),
                None => {
                    let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| matches!(c.at(x + dx, y + dy), Some(n) if n.part != Part::Whisk));
                    if edge { Some([70, 71, 86]) } else { None }
                }
            };
            if let Some([r, g, b]) = rgb {
                out[y as usize * GW + x as usize] = [r, g, b, 255];
            }
        }
    }
    out
}

fn color(c: &Cell) -> [u8; 3] {
    let i = (c.shade + 1) as usize;
    if c.sep {
        return if c.mat == Mat::K { [62, 62, 76] } else { [176, 178, 192] };
    }
    match c.mat {
        Mat::K => [[12, 12, 16], [26, 26, 31], [46, 47, 58]][i],
        Mat::W => [[196, 199, 212], [247, 247, 242], [255, 255, 255]][i],
        Mat::P => if c.shade < 0 { [198, 110, 126] } else { [238, 152, 164] },
        Mat::Y => [[196, 140, 36], [238, 196, 60], [214, 220, 96]][i],
        Mat::N => [8, 8, 10],
        Mat::S => [255, 255, 255],
        Mat::G => [214, 216, 226],
        Mat::L => [186, 188, 204],
        Mat::M => [150, 152, 168],
    }
}
