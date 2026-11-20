# notify-claude

Un gato esmoquin en pixel art que asoma por la barra de tareas cuando Claude Code te hace una pregunta o te pide permiso, y se despide en cuanto respondes. También te avisa cuando Claude termina un trabajo largo. Así no se te queda Claude esperando en un terminal que no estás mirando.

Es un mod de Claude Code y una pequeña app de Windows escrita en Rust. Solo funciona en Windows.

## Instalación

En Claude Code, en el terminal:

```
/plugin install notify-claude --marketplace LucasPrezFdez/notify-claude
```

O desde una consola:

```
claude plugin install notify-claude --marketplace LucasPrezFdez/notify-claude
```

Contesta `y` para añadir el marketplace y elige dónde instalarlo (con Enter, para tu usuario). No hace falta reiniciar: el mod se activa en esa misma sesión.

## Qué hace

| Aviso | El gato aparece | El gato se va |
| --- | --- | --- |
| Pregunta | Cuando Claude te pregunta algo (`AskUserQuestion`) | En cuanto respondes |
| Permiso | Cuando sale el diálogo de permiso | Al permitirlo o rechazarlo |
| Terminado | Cuando Claude acaba de responder tras 30 s o más trabajando sin ti | Solo, unos 10 s después |

- El bocadillo dice qué pasa ("¿Puedo usar Bash?", el principio de la pregunta o "¡He terminado!") y de qué carpeta viene el aviso.
- Los 30 s del aviso de terminado se cuentan desde lo último que hiciste tú: escribir el prompt, responder una pregunta o un permiso. El rato que Claude pasa esperándote no cuenta. Tampoco avisa si interrumpes el turno, si acaba en error o si quien termina es un subagente.
- Si lo pulsas, guiña un ojo y se va.
- También se va si escribes otro prompt o cierras la sesión, y la pregunta o el permiso, si termina el turno.
- Con varios terminales a la vez, solo se ve el último aviso.
- Aparece en el monitor de la ventana activa, en la esquina inferior derecha, encima de la barra de tareas. No te quita el foco y los clics atraviesan lo transparente.

Con Bash y PowerShell, el gato se va cuando aparece su aviso de «ctrl+b to run in background» o cuando el comando termina. Claude Code no da el momento exacto en que pulsas "Permitir" sin quitarle a esas herramientas la opción de mandarse a segundo plano.

## Opciones

En `/config`, **Gato en el escritorio** lo activa o lo desactiva (activado de serie).

## Requisitos

- Windows 11 (probado). La app es Win32 nativa y pesa unos 180 KB.
- Claude Code con soporte de mods. Probado con la 2.1.329; la API de mods todavía puede cambiar entre versiones.

## Cómo está hecho

```
notify-claude/
├── .claude-plugin/marketplace.json   el repositorio es también el marketplace
├── plugin/                           lo que se instala
│   ├── hooks/register.ts             el mod: decide cuándo abrir y cerrar el gato
│   ├── tests/                        pruebas del mod
│   └── bin/mascota.exe               la app, ya compilada
├── desktop/                          la app en Rust + Win32 (ventana layered con alfa por píxel)
├── sprites/                          generador del gato y sus animaciones, con vista previa
└── INVESTIGACION.md                  el diseño, con lo que se decidió y por qué
```

El mod lanza `mascota.exe --kind permiso|pregunta|terminado --session <id> --message <texto> --cwd <ruta>` para avisar y `mascota.exe --dismiss --session <id>` para despedirlo. La app es de instancia única: si ya hay un gato en pantalla, la segunda copia le pasa el aviso y se cierra.

## Desarrollo

Compilar la app y copiarla al plugin (PowerShell):

```
cargo build --release --manifest-path desktop/Cargo.toml
Copy-Item desktop/target/release/mascota.exe plugin/bin/
```

Probar el mod sin instalarlo:

```
claude --plugin-dir ./plugin
```

Pruebas:

```
cargo test --manifest-path desktop/Cargo.toml   # la app, incluida la paridad de los 858 fotogramas con sprites/gato.js
claude plugin test ./plugin                     # el mod
claude plugin validate .                        # el marketplace y el manifiesto
node sprites/vista-previa.js                    # genera sprites/salida/animaciones.html
```

## Licencia

MIT. La app lleva dentro la fuente [Terminus](https://terminus-font.sourceforge.net/), con licencia SIL Open Font License 1.1 (`desktop/licencias/Terminus-OFL.txt`).
