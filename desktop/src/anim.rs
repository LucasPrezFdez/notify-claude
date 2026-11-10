//! Las animaciones: cada una es una función del tiempo que devuelve una pose y un
//! desplazamiento vertical. Port literal de `ANIMS` en `sprites/gato.js`.

use crate::cat::{Paw, Pose};
use std::f64::consts::PI;

pub const FPS: f64 = 30.0;
const TAU: f64 = PI * 2.0;
const PER: f64 = 3.2;

#[derive(Clone, Copy)]
enum Ease {
    Lin,
    In,
    Out,
    Io,
    Back,
}
use Ease::*;

fn ease(e: Ease, x: f64) -> f64 {
    match e {
        Lin => x,
        In => x * x,
        Out => 1.0 - (1.0 - x).powi(3),
        Io => if x < 0.5 { 4.0 * x * x * x } else { 1.0 - (-2.0 * x + 2.0).powi(3) / 2.0 },
        Back => {
            let c = 1.70158;
            1.0 + (c + 1.0) * (x - 1.0).powi(3) + c * (x - 1.0).powi(2)
        }
    }
}

/// Pista de claves `(tiempo, valor, curva para llegar a esta clave)`.
fn tr(t: f64, keys: &[(f64, f64, Ease)]) -> f64 {
    if t <= keys[0].0 {
        return keys[0].1;
    }
    for i in 1..keys.len() {
        if t <= keys[i].0 {
            let (t0, v0, _) = keys[i - 1];
            let (t1, v1, e) = keys[i];
            return v0 + (v1 - v0) * ease(e, (t - t0) / (t1 - t0));
        }
    }
    keys[keys.len() - 1].1
}

fn blink(t: f64, t0: f64, d: f64) -> f64 {
    let u = (t - t0) / d;
    if u <= 0.0 || u >= 1.0 {
        1.0
    } else if u < 0.35 {
        1.0 - ease(Io, u / 0.35)
    } else if u < 0.5 {
        0.0
    } else {
        ease(Io, (u - 0.5) / 0.5)
    }
}

fn pulse(t: f64, t0: f64) -> f64 {
    let u = (t - t0) / 0.24;
    if u <= 0.0 || u >= 1.0 { 0.0 } else { (PI * u).sin() }
}

/// Capa de reposo: respiración y cola con la punta retrasada. `w` la atenúa.
fn idle(tg: f64, w: f64) -> Pose {
    Pose {
        br: (0.5 - 0.5 * (TAU * tg / PER).cos()) * w,
        tail_a: 1.2 * (TAU * tg / PER).sin() * w,
        tail_tb: 1.5 * (TAU * tg / PER - 1.0).sin() * w,
        ..Pose::default()
    }
}

fn ear(f: f64) -> [f64; 2] {
    [2.5 * f, 3.0 * f]
}

// Quieto, Oculto y Reposo solo se usan, de momento, en la prueba de paridad.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Quieto,
    Oculto,
    Reposo,
    Entrada,
    Pregunta,
    Permiso,
    Salida,
}

impl Anim {
    pub fn dur(self) -> f64 {
        match self {
            Anim::Quieto => 1.2,
            Anim::Oculto => 0.9,
            Anim::Reposo => PER * 2.0,
            Anim::Entrada => 3.4,
            Anim::Pregunta => 4.8,
            Anim::Permiso => 5.2,
            Anim::Salida => 1.9,
        }
    }

    /// Momento a partir del cual la animación vuelve a la postura sentada.
    /// Sirve para cortar un bucle sin que el gato dé un tirón.
    pub fn wind_down(self) -> f64 {
        match self {
            Anim::Pregunta => 4.3,
            Anim::Permiso => 4.45,
            _ => 0.0,
        }
    }

    #[cfg(test)]
    pub fn from_name(s: &str) -> Option<Anim> {
        Some(match s {
            "quieto" => Anim::Quieto,
            "oculto" => Anim::Oculto,
            "reposo" => Anim::Reposo,
            "entrada" => Anim::Entrada,
            "pregunta" => Anim::Pregunta,
            "permiso" => Anim::Permiso,
            "salida" => Anim::Salida,
            _ => return None,
        })
    }
}

/// Pose y desplazamiento vertical (en píxeles del sprite, positivo hacia abajo) de la
/// animación `a` en su segundo `t`. `tg` es el reloj global, que mantiene la capa de reposo
/// continua al encadenar animaciones.
pub fn frame_at(a: Anim, t: f64, tg: f64) -> (Pose, i32) {
    let (pose, dy) = match a {
        Anim::Quieto => (idle(tg, 1.0), 0.0),
        Anim::Oculto => (Pose::default(), 70.0),
        Anim::Reposo => {
            let e = blink(t, 1.8, 0.34).min(blink(t, 5.0, 0.3));
            let mut p = idle(tg, 1.0);
            p.eye_l = e;
            p.eye_r = e;
            p.look = if t > 3.0 && t < 3.9 { -1.0 } else { 0.0 };
            p.ear_r = [1.6 * pulse(t, 2.6), 0.6 * pulse(t, 2.6)];
            p.ear_l = [1.6 * pulse(t, 4.4), 0.6 * pulse(t, 4.4)];
            (p, 0.0)
        }
        Anim::Entrada => {
            let (j0, jd) = (1.95, 0.5);
            let u = (t - j0) / jd;
            let s = t - (j0 + jd);
            let dy = if t < j0 {
                tr(t, &[(0.45, 50.0, Io), (0.9, 29.0, Back), (1.6, 29.0, Io), (j0, 35.0, Io)])
            } else if u < 1.0 {
                35.0 * (1.0 - u) - 88.0 * u * (1.0 - u)
            } else {
                0.0
            };
            let w = tr(t, &[(2.8, 0.0, Io), (3.4, 1.0, Io)]);
            let mut p = idle(tg, w);
            let edge = 48.0 - dy - 1.5;
            if t < j0 {
                p.paw_l = Some(Paw { lift: 1.0, x: 5.0, y: edge + tr(t, &[(0.0, 6.0, Io), (0.22, 0.0, Back)]) });
                p.paw_r = Some(Paw { lift: 1.0, x: 5.0, y: edge + tr(t, &[(0.14, 6.0, Io), (0.36, 0.0, Back)]) });
            } else if u < 1.0 {
                let pw = Paw {
                    lift: tr(u, &[(0.4, 1.0, Io), (0.95, 0.0, In)]),
                    x: tr(u, &[(0.0, 5.0, Io), (0.35, 10.0, Io)]),
                    y: tr(u, &[(0.0, 10.5, Io), (0.35, 27.0, Io)]),
                };
                p.paw_l = Some(pw);
                p.paw_r = Some(pw);
                p.tail_a = 0.65 * u;
                p.tail_tb = -2.2 * u;
                p.tail_up = 0.0;
            } else {
                let d = (-3.2 * s).exp();
                p.tail_a += 0.65 * d * (9.0 * s).cos() * (1.0 - w);
                p.tail_tb += -2.2 * d * (9.0 * s).cos() * (1.0 - w);
            }
            p.b = tr(
                t,
                &[(1.6, 0.0, Io), (1.95, 0.8, Io), (2.05, -1.2, Out), (2.35, -0.6, Io), (2.45, 0.2, Lin), (2.52, 1.7, Out), (2.72, -0.4, Io), (2.95, 0.15, Io), (3.15, 0.0, Io)],
            );
            let f = tr(t, &[(0.9, 0.6, Io), (1.05, 0.0, Back), (1.6, 0.0, Io), (1.9, 0.5, Io), (2.1, 0.8, Io), (2.45, 0.6, Io), (2.55, 1.0, Io), (2.9, 0.0, Back)]);
            p.ear_l = ear(f);
            p.ear_r = [ear(f)[0] + 1.6 * pulse(t, 1.15), ear(f)[1]];
            p.look = if t > 0.98 && t < 1.22 {
                -1.0
            } else if t > 1.28 && t < 1.52 {
                1.0
            } else {
                0.0
            };
            let e = blink(t, 1.55, 0.22).min(tr(t, &[(2.45, 1.0, Io), (2.52, 0.35, Io), (2.85, 1.0, Io)]));
            p.eye_l = e;
            p.eye_r = e;
            p.pupil = tr(t, &[(1.5, 1.1, Io), (1.9, 2.0, Io), (2.45, 2.0, Io), (2.95, 1.1, Io)]);
            (p, dy)
        }
        Anim::Pregunta => {
            let mut p = idle(tg, 1.0);
            let env = tr(t, &[(0.5, 0.0, Io), (0.7, 1.0, Io), (2.2, 1.0, Io), (2.5, 0.0, Io)]).max(tr(t, &[(2.9, 0.0, Io), (3.1, 1.0, Io), (3.8, 1.0, Io), (4.1, 0.0, Io)]));
            let ph = TAU * 2.2 * (t - 0.6);
            p.paw_l = Some(Paw {
                lift: tr(t, &[(0.0, 0.0, Io), (0.35, 1.0, Back), (4.3, 1.0, Io), (4.7, 0.0, Io)]),
                x: 8.5 - 1.2 * env * ph.sin(),
                y: 28.0 - 2.6 * env * (0.5 - 0.5 * ph.cos()),
            });
            p.hx = -tr(t, &[(0.2, 0.0, Io), (0.45, 1.0, Io), (4.2, 1.0, Io), (4.5, 0.0, Io)]);
            p.pupil = tr(t, &[(0.0, 1.1, Io), (0.4, 1.5, Io), (4.3, 1.5, Io), (4.7, 1.1, Io)]);
            let e = blink(t, 2.6, 0.3);
            p.eye_l = e;
            p.eye_r = e;
            p.ear_r = [1.6 * pulse(t, 1.6), 0.6 * pulse(t, 1.6)];
            (p, 0.0)
        }
        Anim::Permiso => {
            let mut p = idle(tg, 1.0);
            p.b = tr(t, &[(0.0, 0.0, Io), (0.15, 0.6, Io), (0.5, -1.3, Back), (4.5, -1.3, Io), (4.75, 0.5, Io), (5.0, 0.0, Out)]);
            let hd = crate::cat::jsr(p.b * 2.0) as f64;
            let lift = tr(t, &[(0.1, 0.0, Io), (0.5, 1.0, Out), (4.45, 1.0, Io), (4.8, 0.0, Io)]);
            let env = tr(t, &[(1.1, 0.0, Io), (1.3, 1.0, Io), (1.9, 1.0, Io), (2.1, 0.0, Io)]).max(tr(t, &[(3.0, 0.0, Io), (3.2, 1.0, Io), (3.8, 1.0, Io), (4.0, 0.0, Io)]));
            let bob = |d: f64| -1.8 * env * (TAU * 1.25 * (t - d)).sin().abs();
            p.paw_l = Some(Paw { lift, x: 12.5, y: 30.0 + hd + bob(0.0) });
            p.paw_r = Some(Paw { lift, x: 12.5, y: 30.0 + hd + bob(0.09) });
            p.pupil = tr(t, &[(0.1, 1.1, Io), (0.6, 1.8, Io), (4.4, 1.8, Io), (4.9, 1.1, Io)]);
            let e = blink(t, 2.4, 0.3).min(blink(t, 4.15, 0.26));
            p.eye_l = e;
            p.eye_r = e;
            (p, 0.0)
        }
        Anim::Salida => {
            let mut p = idle(tg, tr(t, &[(0.6, 1.0, Io), (0.95, 0.0, Io)]));
            p.happy_r = true;
            p.eye_r = tr(t, &[(0.05, 1.0, Io), (0.2, 0.0, Io), (0.5, 0.0, Io), (0.68, 1.0, Io)]);
            p.eye_l = tr(t, &[(0.05, 1.0, Io), (0.2, 0.75, Io), (0.5, 0.75, Io), (0.68, 1.0, Io)]);
            p.b = tr(t, &[(0.7, 0.0, Io), (0.95, 1.3, Io), (1.05, -0.8, Out)]);
            let f = tr(t, &[(0.7, 0.0, Io), (0.95, 0.7, Io), (1.15, 0.2, Io)]);
            p.ear_l = ear(f);
            p.ear_r = ear(f);
            p.tail_up = tr(t, &[(0.95, 0.0, Io), (1.3, 1.0, Io)]);
            (p, tr(t, &[(0.95, 0.0, Io), (1.08, -4.0, Out), (1.75, 66.0, In)]))
        }
    };
    (pose, crate::cat::jsr(dy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::draw_cat;

    /// Cada fotograma tiene que salir idéntico al de `sprites/gato.js`.
    #[test]
    fn paridad_con_el_generador_js() {
        let mut total = 0;
        let mut bad = Vec::new();
        for line in include_str!("../../sprites/paridad.txt").lines() {
            let f: Vec<&str> = line.split(' ').collect();
            let anim = Anim::from_name(f[0]).expect("animación desconocida");
            let i: f64 = f[1].parse().unwrap();
            let (pose, dy) = frame_at(anim, i / FPS, i / FPS);
            let mut h: u32 = 0x811c9dc5;
            for px in draw_cat(&pose) {
                for v in px {
                    h = (h ^ v as u32).wrapping_mul(0x01000193);
                }
            }
            total += 1;
            if format!("{h:08x}") != f[3] || dy.to_string() != f[2] {
                bad.push(format!("{} {}", f[0], f[1]));
            }
        }
        assert!(total > 700, "faltan fotogramas en paridad.txt");
        assert!(bad.is_empty(), "{} de {} fotogramas distintos: {:?}", bad.len(), total, &bad[..bad.len().min(12)]);
    }
}
