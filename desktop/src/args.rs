//! Lo que se le pide a la app por la línea de órdenes, y cómo viaja a la instancia abierta.
//!
//! ```text
//! mascota --kind pregunta|permiso --session <id> [--message <texto>] [--project <nombre> | --cwd <ruta>]
//! mascota --dismiss --session <id>
//! ```

use crate::scene::Kind;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Request {
    Show { kind: Kind, session: String, message: String, project: String },
    Dismiss { session: String },
}

/// Separador de campos al reenviar la petición con `WM_COPYDATA`.
const SEP: char = '\u{1f}';

fn folder_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Request, String> {
    let (mut kind, mut session, mut message, mut project, mut dismiss) = (None, String::new(), None, String::new(), false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("falta el valor de {name}"));
        match a.as_str() {
            "--dismiss" => dismiss = true,
            "--kind" => {
                kind = Some(match value("--kind")?.as_str() {
                    "pregunta" | "question" => Kind::Pregunta,
                    "permiso" | "permission" => Kind::Permiso,
                    other => return Err(format!("tipo de aviso desconocido: {other}")),
                })
            }
            "--session" => session = value("--session")?,
            "--message" => message = Some(value("--message")?),
            "--project" => project = value("--project")?,
            "--cwd" => project = folder_name(&value("--cwd")?),
            other => return Err(format!("argumento desconocido: {other}")),
        }
    }
    if dismiss {
        return Ok(Request::Dismiss { session });
    }
    let kind = kind.ok_or("hace falta --kind pregunta|permiso o --dismiss")?;
    let message = message.filter(|m| !m.trim().is_empty()).unwrap_or_else(|| {
        match kind {
            Kind::Pregunta => "Tengo una pregunta",
            Kind::Permiso => "Necesito tu permiso",
        }
        .to_string()
    });
    Ok(Request::Show { kind, session, message, project })
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Show { kind, session, message, project } => {
                let k = if *kind == Kind::Pregunta { "pregunta" } else { "permiso" };
                [k, session, message, project].join(&SEP.to_string())
            }
            Request::Dismiss { session } => ["dismiss", session].join(&SEP.to_string()),
        }
    }

    pub fn decode(s: &str) -> Option<Request> {
        let f: Vec<&str> = s.split(SEP).collect();
        match f.as_slice() {
            ["dismiss", session] => Some(Request::Dismiss { session: session.to_string() }),
            [k @ ("pregunta" | "permiso"), session, message, project] => Some(Request::Show {
                kind: if *k == "pregunta" { Kind::Pregunta } else { Kind::Permiso },
                session: session.to_string(),
                message: message.to_string(),
                project: project.to_string(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Request, String> {
        parse(a.iter().map(|s| s.to_string()))
    }

    #[test]
    fn aviso_de_permiso() {
        let r = p(&["--kind", "permiso", "--session", "abc", "--message", "¿Puedo usar Bash?", "--cwd", r"C:\Users\x\Documents\notify-claude\"]).unwrap();
        assert_eq!(r, Request::Show { kind: Kind::Permiso, session: "abc".into(), message: "¿Puedo usar Bash?".into(), project: "notify-claude".into() });
    }

    #[test]
    fn mensaje_por_defecto() {
        let Request::Show { message, .. } = p(&["--kind", "question"]).unwrap() else { panic!() };
        assert_eq!(message, "Tengo una pregunta");
    }

    #[test]
    fn cierre() {
        assert_eq!(p(&["--dismiss", "--session", "abc"]).unwrap(), Request::Dismiss { session: "abc".into() });
    }

    #[test]
    fn errores() {
        assert!(p(&[]).is_err());
        assert!(p(&["--kind", "otro"]).is_err());
        assert!(p(&["--kind"]).is_err());
        assert!(p(&["--nada"]).is_err());
    }

    #[test]
    fn ida_y_vuelta() {
        for r in [p(&["--kind", "pregunta", "--session", "s 1", "--message", "¿Qué hago?\ncon dos líneas", "--project", "x"]).unwrap(), p(&["--dismiss", "--session", "s"]).unwrap()] {
            assert_eq!(Request::decode(&r.encode()), Some(r));
        }
        assert_eq!(Request::decode("basura"), None);
    }
}
