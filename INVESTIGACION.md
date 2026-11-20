# Investigación: plugin `notify-claude`

Objetivo: usar la mascota de Claude Code (el bichito) para avisar cuando Claude termina de responder o necesita atención (permiso, datos, etc.), de dos maneras:

- **En el escritorio**: la mascota aparece en pantalla solo cuando tiene que avisar, y te da el aviso.
- **En el terminal** (y solo ahí): la mascota vive a la derecha, encima de la caja de escribir, y se pasea por encima de ella.

Entorno de destino: Windows 11, Windows Terminal, PowerShell 5.1 y Node instalados; sin `pwsh` ni el módulo BurntToast. Rust y Tauri ya se usan en DesktopFriend.

Fechas: 08-11-2026, actualizado el 09-11-2026 (Claude Code 2.1.319).

---

## 1. Qué es un plugin de Claude Code

Un plugin es una carpeta con un manifiesto y una serie de piezas opcionales:

```
notify-claude/
├── .claude-plugin/
│   └── plugin.json        ← solo el manifiesto va aquí dentro
├── hooks/
│   └── hooks.json         ← eventos a los que reacciona (o el módulo del mod)
├── scripts/               ← lo que ejecutan los hooks clásicos
└── assets/                ← imágenes, sprites
```

Puede incluir también skills, commands, agents, servidores MCP/LSP, output styles, themes, monitors, workflows y `bin/`.

### Manifiesto (`plugin.json`)

Solo `name` es obligatorio. Ejemplo mínimo:

```json
{
  "name": "notify-claude",
  "version": "0.1.0",
  "description": "La mascota de Claude Code te avisa cuando Claude termina o necesita atención",
  "author": { "name": "Lucas" }
}
```

- El nombre tiene que ir en kebab-case y **no puede empezar por** `claude-`, `anthropic-`, `anthropics-` ni `cc-plugin-`, porque la validación lo rechaza. Por eso se llama `notify-claude` y no `claude-notify`.
- Otros campos útiles: `homepage`, `repository`, `license`, `keywords`, `userConfig` (opciones configurables por el usuario).

### Variables disponibles en los hooks

| Variable | Qué es |
| --- | --- |
| `${CLAUDE_PLUGIN_ROOT}` | Carpeta instalada del plugin. Cambia al actualizar: no guardar estado ahí. |
| `${CLAUDE_PLUGIN_DATA}` | `~/.claude/plugins/data/<id>/`. Persiste entre actualizaciones. |
| `${CLAUDE_PROJECT_DIR}` | Raíz del proyecto abierto. |
| `CLAUDE_PLUGIN_OPTION_<KEY>` | Valores de `userConfig`, exportados al entorno del hook. |

---

## 2. Eventos (hooks) que sirven para el objetivo

`hooks/hooks.json` lleva la clave raíz `"hooks"`, con la misma forma que en `settings.json`.

Todos los eventos reciben por stdin un JSON con `session_id`, `transcript_path`, `cwd`, `permission_mode` y `hook_event_name`, más los campos propios de cada evento.

| Evento | Cuándo salta | Datos útiles | Notas |
| --- | --- | --- | --- |
| `Stop` | Claude termina de responder | `last_assistant_message`, `stop_hook_active` | No salta si el usuario interrumpe. Un error de API dispara `StopFailure`, no `Stop`. |
| `Notification` · `permission_prompt` | Claude lleva unos 6 s esperando un permiso | `message`, `title` | No puede bloquear nada; su código de salida se ignora. |
| `Notification` · `elicitation_dialog` | Un servidor MCP pide datos (tras unos 6 s) | `message` | |
| `Notification` · `idle_prompt` | 60 s después de terminar sin que escribas | `message` | **Se solapa con `Stop`**: elegir uno para no avisar dos veces. |
| `PermissionRequest` | En cuanto Claude necesita un permiso, sin esperar | `tool_name`, `tool_input` | Si no devuelve `decision`, no altera el flujo. |
| `SubagentStop` | Termina un subagente | `agent_id`, `agent_type` | También salta con agentes internos: puede ser ruidoso. |

Otros valores de `notification_type`: `auth_success`, `elicitation_url_dialog`, `agent_needs_input`, `agent_completed`, `plugin_notification`, `quota_auto_resume_*`… El momento exacto de `agent_needs_input` y `agent_completed` **no está documentado**.

Ejemplo de entrada de `Stop`:

```json
{
  "session_id": "abc123",
  "hook_event_name": "Stop",
  "stop_hook_active": false,
  "last_assistant_message": "He terminado la refactorización. Resumen: ..."
}
```

Ejemplo de entrada de `Notification`:

```json
{
  "hook_event_name": "Notification",
  "message": "Claude needs your permission",
  "title": "Permission needed",
  "notification_type": "permission_prompt"
}
```

> Importante: un hook de `Stop` que sale con código 2, o que devuelve `decision: "block"`, impide que Claude pare. Cualquier script que lance el aviso debe salir **siempre con 0**.

### Opciones de cada hook

`type` (`command`, `http`, `mcp_tool`, `prompt`, `agent`), `command`, `args`, `timeout` (segundos), `async` (no bloquea a Claude), `shell` (`"bash"` o `"powershell"`), `statusMessage` y `onFailure`.

---

## 3. Hooks en Windows

- **Forma shell** (`command` sin `args`): en Windows usa Git Bash si está instalado; si no, PowerShell.
- **Forma exec** (`command` + `args`): no pasa por ningún shell. `command` tiene que ser un ejecutable real (`.exe`), como `powershell.exe` o el `.exe` de una app propia.
- Las rutas sustituidas conservan las barras invertidas en los hooks exec y en los que pasan por PowerShell.

Opción recomendada: forma exec y `async: true`, para no depender de Git Bash ni frenar a Claude.

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [ {
          "type": "command",
          "command": "powershell.exe",
          "args": ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
                   "${CLAUDE_PLUGIN_ROOT}/scripts/notify.ps1", "-Kind", "done"],
          "async": true,
          "timeout": 15
      } ] }
    ],
    "Notification": [
      { "hooks": [ {
          "type": "command",
          "command": "powershell.exe",
          "args": ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
                   "${CLAUDE_PLUGIN_ROOT}/scripts/notify.ps1", "-Kind", "attention"],
          "async": true,
          "timeout": 15
      } ] }
    ]
  }
}
```

El script filtraría por `notification_type` en lugar de usar un matcher con `|`, que no está verificado. Para leer el JSON de stdin en PowerShell:

```powershell
$payload = [Console]::In.ReadToEnd() | ConvertFrom-Json
```

---

## 4. Notificación toast de Windows (descartada)

> **Descartada el 09-11-2026**: no se quiere una notificación como tal, sino que la mascota aparezca en el escritorio (sección 6). Se conserva como referencia.

### Lo que ofrece Claude Code de serie

- Las notificaciones nativas (`preferredNotifChannel`) solo muestran aviso de escritorio en **iTerm2, kitty y Ghostty**. Valores conocidos: `terminal_bell`, `iterm2`, `kitty`, `ghostty`.
- **Windows Terminal no tiene canal nativo**: como mucho, un pitido (`terminal_bell`), sin texto ni icono.
- Un hook de `Notification` se ejecuta junto a la notificación nativa, no la sustituye.
- El ejemplo oficial para Windows usa un `MessageBox` modal: funciona, pero es un cuadro de diálogo que interrumpe.

### Opciones técnicas que se estudiaron

1. **Toast nativo de Windows desde PowerShell 5.1 (WinRT)**:
   - `Windows.UI.Notifications.ToastNotificationManager` con una plantilla XML e `<image placement="appLogoOverride" src="...png"/>`.
   - Con el AUMID de PowerShell aparece como "Windows PowerShell"; para que ponga "Claude Code" hace falta un acceso directo en el menú Inicio.
2. **`NotifyIcon` (System.Windows.Forms)**: globo de la bandeja del sistema. Necesita un `.ico` y tiene un aspecto más antiguo.

---

## 5. Cómo probar el plugin

1. Probar a mano lo que ejecuta cada hook, simulando la entrada:
   ```
   echo {"hook_event_name":"Notification","message":"Necesito permiso","notification_type":"permission_prompt"} | powershell -NoProfile -File .\scripts\notify.ps1 -Kind attention
   ```
2. Validar el plugin: `claude plugin validate ./notify-claude --strict`
3. Cargarlo sin instalarlo: `claude --plugin-dir ./notify-claude`
4. Tras cada cambio: `/reload-plugins`
5. Comprobar la carga: `/plugin` (pestañas Installed y Errors), `/hooks`, `claude plugin details notify-claude`
6. Depurar: `claude --debug` muestra qué hooks coinciden y su código de salida.
7. Para el mod: `claude plugin test ./notify-claude` ejecuta sus pruebas `*.test.ts`, y `tsc -p ./notify-claude` comprueba los tipos.

### Instalación permanente (marketplace local)

```
mi-marketplace/
├── .claude-plugin/marketplace.json
└── plugins/notify-claude/
```

```json
{
  "name": "mi-marketplace",
  "owner": { "name": "Lucas" },
  "plugins": [
    { "name": "notify-claude", "source": "./plugins/notify-claude" }
  ]
}
```

- `claude plugin marketplace add ./mi-marketplace`
- `claude plugin install notify-claude@mi-marketplace`

El `name` de la entrada tiene que coincidir con el de `plugin.json`. Instalado desde una ruta local, se carga en el sitio y basta con `/reload-plugins` para ver los cambios.

---

## 6. La mascota en el escritorio (app aparte)

**Decidido el 09-11-2026:**

- App independiente, que no forma parte de DesktopFriend.
- **Se abre cada vez que tiene que avisar y se cierra al terminar el aviso** (la opción A de la tabla de abajo).
- Se cierra cuando el programador **pulsa en la mascota** (señal de que lo ha visto) o cuando **hace lo que el agente necesitaba**.
- La mascota es **literalmente la de Claude Code**, y tanto ella como el bocadillo del mensaje van **dibujados en pixel art**. Es un requisito clave.
- **Solo avisa cuando Claude te pregunta algo (`AskUserQuestion`) o te pide permiso.** No avisa cuando termina, ni cuando un servidor MCP pide datos (`elicitation_dialog`), por ejemplo al conectarlo.
  - **Cambiado el 20-11-2026**: también avisa cuando Claude termina después de trabajar un buen rato sin ti. Ver "Aviso de terminado" más abajo.
- **Aparece aunque estés mirando el terminal**, porque así se ve que el plugin funciona. Habrá una opción para desactivarla.
- **Si llegan varios avisos, solo se muestra el último.**

### Por qué hace falta una app

Un plugin de Claude Code no puede dibujar en el escritorio: solo puede ejecutar programas o hacer peticiones HTTP. La mascota tiene que ser una app propia a la que el plugin avisa.

### La mascota: la de Claude Code, tal cual

Comprobado en el propio Claude Code (2.1.319):

- **Cómo está dibujada**: con caracteres de cuadrantes (`▐ ▛ █ ▜ ▌ ▝ ▘`…), donde cada celda del terminal equivale a 2×2 píxeles. La versión pequeña ocupa unas 9 columnas por 3 filas, unos 18×6 píxeles. La versión grande, la de la pantalla de bienvenida, usa bloques completos y medios bloques.
- **Colores del tema**:
  - Cuerpo, `clawd_body`: `rgb(215,119,87)`.
  - Fondo, `clawd_background`: `rgb(0,0,0)`.
  - En los temas ANSI, `redBright` sobre `black`.
- **Poses que ya trae Claude Code**:
  - Brazos: `down`, `up`, `one-up`.
  - Ojos: `open`, `left`, `right`, `closed`, `wink`.
  - Giro por pasos: `right-12`, `right-30`, `right-55`, `right-75`, `edge`, `back-105`, `back-125`, `back-150`. Sirve para que se dé la vuelta al cambiar de dirección.
- **Plan**: pasar esos fotogramas una sola vez a una rejilla de píxeles (un sprite) y usar la misma rejilla en la app y en el terminal, para que sea idéntica en los dos sitios. Para avisar podría saludar con un brazo (`one-up`), guiñar un ojo (`wink`) o levantar los dos brazos (`up`).

### El bocadillo: también en pixel art

- **Forma**: borde de un píxel de la mascota, esquinas escalonadas y un piquito que apunta a la mascota.
- **Colores**: los del tema de Claude Code, con borde naranja `clawd_body` y fondo negro.
- **Fuente**: bitmap y con tildes, `ñ`, `¡` y `¿`. Elegida: Terminus (ver "Detalles decididos").
- **Textos**: "¡He terminado!", "Necesito tu permiso", "Tengo una pregunta"… Podrían llevar también el nombre del proyecto (la carpeta `cwd`) para saber de qué terminal viene el aviso.

### Cómo conseguir que se vea pixelado de verdad

- **Dibujo**: en un `<canvas>`, con `imageSmoothingEnabled = false` y `image-rendering: pixelated`.
- **Escala entera**: cada píxel de la mascota ocupa N×N píxeles de pantalla, por ejemplo 8×8.
- **Escalado de Windows** (125 %, 150 %): hay que calcular con `devicePixelRatio` para que cada píxel caiga en un número entero de píxeles físicos. Si no, sale borroso o con líneas desiguales.
- **Animación**: por fotogramas, sin interpolar ni rotar suavemente, igual que en el terminal.

### Ventana

Transparente, sin bordes, siempre encima, sin icono en la barra de tareas y dejando pasar los clics fuera de la mascota, como hace DesktopFriend con `clickThrough.ts`.

### Ciclo de vida de un aviso

1. Claude te pregunta algo o te pide permiso, y el plugin lanza `app.exe --kind <tipo> --session <id> --message "..."`.
2. La app se abre, la mascota entra (por ejemplo, por la esquina inferior derecha) y muestra el bocadillo.
3. Se queda hasta que pasa una de estas dos cosas:
   - **Pulsas en la mascota**: la app se despide y se cierra sola.
   - **Haces lo que el agente necesitaba**: el plugin lanza `app.exe --dismiss --session <id>`. La instancia única (en Win32, un mutex con nombre y `WM_COPYDATA`; en Tauri, `tauri-plugin-single-instance`) le pasa esa orden a la app que ya está abierta, y esta se despide y se cierra.
4. No hay tiempo límite: si no haces nada, la mascota sigue ahí. La excepción es el aviso de terminado, que se va solo (ver "Aviso de terminado").

### Qué cuenta como "hacer lo que necesitaba"

| Aviso | Qué lo abre | Qué lo cierra | Fiabilidad |
| --- | --- | --- | --- |
| Claude te hace una pregunta (`AskUserQuestion`) | `tool.call` con `tool: 'AskUserQuestion'`, antes de `next(e)` | Que `next(e)` resuelva, porque ya has respondido | ✅ preciso |
| Necesita permiso | `classic.PermissionRequest`, que salta justo al enseñar el diálogo | Lo primero que llegue: `tool.run` de esa llamada (concedido, antes de ejecutarse), el aviso "ctrl+b to run in background" de Bash (`ui.render` sobre `ToolProgress`: ya corre) o la fila con su resultado (`session.append` por la puerta `tool-result`: concedido y terminado, o rechazado) | ✅ preciso, salvo con Bash y PowerShell (ver abajo) |
| Claude ha terminado | `turn.complete` del bucle principal con `reason: 'answer'`, tras 30 s o más trabajando sin ti | Nada que hacer: la app se despide sola 10 s después de enseñar el bocadillo | ✅ |
| Cualquiera | — | Cierras la sesión (`session.end`), escribes otro prompt (`prompt.submit`) o termina el turno (`turn.complete`, también cuando lo interrumpes) sin dar pie a un aviso de terminado | ✅ |

**Resuelto el 20-11-2026** (Claude Code 2.1.329). `tool.run` salta cuando una llamada ya tiene permiso y va a ejecutarse, así que es la señal exacta de "has pulsado Permitir". Pero un hook de `tool.run` o `tool.call` le quita a la herramienta que oye la opción de mandarse a segundo plano (ctrl+b). Por eso el mod solo escucha `tool.run` en las herramientas que no se pueden mandar: Edit, Write, MultiEdit, NotebookEdit, WebFetch, WebSearch y las de MCP. Con Bash y PowerShell, el gato se va cuando aparece su aviso de segundo plano o cuando llega el resultado.

`classic.PermissionRequest` no trae el id de la llamada, así que el mod apunta en `tool.check` las llamadas que el motor va a preguntar (veredicto `ask`) y empareja cada diálogo con la primera pendiente que tenga la misma herramienta y la misma entrada.

### Aviso de terminado (añadido el 20-11-2026)

Lucas pidió que el gato avisara también cuando termina una tarea, y eligió:

- **Cuándo**: solo tras un trabajo largo, cuando Claude lleva **30 s o más trabajando sin ti** (`MIN_TRABAJO_MS` en el mod). Así no salta con cada respuesta corta mientras chateas.
- **Cómo se cuentan esos 30 s**: desde lo último que hiciste tú. Eso es el inicio del turno (`turn.start`), un mensaje escrito mientras trabaja (`prompt.submit`) o la respuesta a una pregunta o a un permiso del gato. El rato que Claude pasa esperándote no cuenta: si tardas cinco minutos en contestar y Claude acaba 3 s después, no avisa. Si el módulo se ha recargado durante el turno, se usa `durationMs` (el turno entero).
- **Cuándo no avisa**: si interrumpes el turno (`aborted`), si acaba en error o rechazo, o si quien termina es un subagente (`agentId`).
- **Texto**: "¡He terminado!", con la carpeta del proyecto debajo, como los demás.
- **Animación**: una nueva, `terminado`. Cierra los dos ojos sonriendo (^^, con `happyL` además del `happyR` del guiño), da dos saltitos con la cola en alto y balancea la cabeza. Se aprobó viendo la vista previa.
- **Cuándo se va**: sola, 10 s después de enseñar el bocadillo (`LEAVE_AFTER` en `scene.rs`). También al pulsarla, al escribir otro prompt o al cerrar la sesión. Si encima llega otro aviso de terminado, vuelve a contar los 10 s. Si llega una pregunta o un permiso, el gato ya no se va solo.
- Sale aunque estés mirando el terminal, como los demás avisos.

### Varios avisos a la vez: solo el último

- Si llega un aviso con la app ya abierta, la instancia única lo recibe y el bocadillo se **sustituye** por el nuevo, aunque venga de otro terminal.
- `--dismiss --session <id>` solo cierra la app si el aviso que se ve es de esa sesión. Si es de otra, la orden se ignora.
- Un aviso sustituido no vuelve a aparecer, aunque siga pendiente en su terminal.

### Dos formas de recibir el aviso (elegida: A)

| | **A. Se lanza con cada aviso** (elegida) | B. Residente en la bandeja |
| --- | --- | --- |
| Cómo le llega el aviso | El plugin ejecuta `app.exe --kind done --message "..."` | La app escucha en `127.0.0.1:<puerto>` y el plugin le hace un POST (hook `type: "http"` o `$.http.fetch` desde el mod) |
| En segundo plano | Nada | La app abierta y oculta todo el rato |
| Retraso | Lo que tarde en arrancar la app (sin medir); la animación de entrada lo disimula | Inmediato |
| Si la app está cerrada | No aplica: se abre sola | El aviso se pierde |
| Varios avisos seguidos | Instancia única: el segundo lanzamiento reenvía sus argumentos a la ventana abierta | Cola dentro de la app |
| Puerto abierto | No | Sí, solo en local |

### Tecnología: cuál es la más rápida para esto

Lo que pide este caso:

- Arrancar desde cero en cada aviso y verse cuanto antes.
- Una ventana pequeña, transparente y siempre encima.
- Pixel art nítido.
- Cerrarse al pulsarla.

Es justo el tipo de ventana para el que Microsoft desaconseja WebView2 (el motor de Tauri): *"To render splash screens or simple dialogs, use lightweight XAML or Win32 screens instead of WebView2 […] due to startup costs and resource contention"*. Cada arranque en frío de WebView2 tiene que levantar los procesos del navegador y sus cachés de disco.

| Opción | Arranque en cada aviso | Memoria | Pixel art | En tu equipo |
| --- | --- | --- | --- | --- |
| **Rust + Win32 nativo** (crate `windows`, ventana *layered* con `UpdateLayeredWindow`) | El más rápido: sin navegador ni runtime que cargar | Pocos MB | Ideal: se escribe cada píxel en un buffer BGRA a escala entera, sin ningún suavizado de por medio | Rust 1.99 y las Build Tools de C++ ya instalados |
| Rust + winit + softbuffer | Muy rápido, parecido al anterior | Pocos MB | Bueno, pero falta comprobar que softbuffer admita transparencia por píxel en Windows | Igual |
| C# WinForms (.NET Framework 4.8) | Rápido, aunque carga el CLR y compila al vuelo | Sin medir | Bien con color clave (`TransparencyKey`), porque el pixel art no tiene semitransparencias | `csc.exe` viene con Windows; no hay SDK de .NET |
| Tauri 2 (WebView2) | Lento para esto: levanta los procesos de Edge en cada aviso. Los blogs dan entre 120 y 800 ms en frío, con métodos poco fiables | DesktopFriend: 204 MB de memoria privada con sus procesos de WebView2 | Bien en un `<canvas>`, pero peleando con el escalado del navegador | Instalado |
| PowerShell + WPF | El más lento: arranca PowerShell y luego carga WPF | Sin medir | Difícil de conseguir nítido y animado | Viene con Windows |

**Decidido el 09-11-2026: Rust + Win32 nativo.**

- **Velocidad**: es lo más rápido que hay en Windows para esto, porque no carga navegador ni runtime.
- **Pixel art**: encaja con el requisito, porque se controla cada píxel. Basta con que la app declare soporte de DPI por monitor para elegir una escala entera en cada pantalla.
- **Clics**: con `UpdateLayeredWindow`, los píxeles con alfa 0 dejan pasar el ratón, así que los clics atraviesan la ventana salvo sobre la mascota.
- **Instancia única**: sencilla. Un mutex con nombre detecta la copia abierta y se le pasan los argumentos con `WM_COPYDATA`.
- **Tamaño**: el `.exe` es pequeño (estimado por debajo de 1 MB), así que puede viajar dentro del plugin.
- **Herramientas**: ya tienes Rust por DesktopFriend.
- **Contras**: es más código de bajo nivel (ventana y texto a mano). El texto tiene que ir en una fuente bitmap, pero eso ya lo pide el estilo pixel.

**Sin medir**: no hay datos fiables de arranque para ninguna opción en Windows. Para confirmarlo habría que hacer dos prototipos mínimos (Rust nativo y Tauri) y medir desde el lanzamiento hasta el primer fotograma visible. Esa prueba abre ventanas en pantalla, así que se hará solo con permiso.

### Cómo la lanza y la cierra el plugin

- **Desde el mod** (sección 7), que es lo recomendado porque ve todos los eventos de la tabla anterior:
  - Para abrir: `$.process.spawn({ argv: [rutaApp, "--kind", "permiso", "--session", id, ...] })`, o `"pregunta"`.
  - Para cerrar: el mismo comando con `--dismiss`.
  - Solo funciona en la CLI, que es donde se usa.
- **Con hooks clásicos**: `Notification`, `PermissionRequest`, `PreToolUse`/`PostToolUse` con `AskUserQuestion` y `UserPromptSubmit`, en forma exec, con `command` apuntando al `.exe` de la app. Es más engorroso que el mod.
- **Ruta del `.exe`**:
  - Si viaja dentro del plugin, el mod la conoce sin configurar nada, gracias a `$.plugin.root` (la carpeta absoluta del plugin): por ejemplo, `${root}/bin/mascota.exe`.
  - Si la app vive aparte, se configura con `userConfig` (por ejemplo, `appPath`).
- **Opción para quitarla**: `userConfig` con, por ejemplo, `desktopMascot: true | false`. Desactivada, no se lanza la app y la mascota del terminal sigue igual.

### Detalles decididos (09-11-2026)

- **Posición**:
  - Esquina inferior derecha del área de trabajo, encima de la barra de tareas.
  - En el monitor de la ventana activa (`GetForegroundWindow` + `MonitorFromWindow`). Si no se puede saber, el principal.
  - En tu equipo: principal de 2560×1440 con barra de tareas de 48 px, y secundario de 1920×1080.
  - Si la barra de tareas no está abajo o se oculta sola, se usa igualmente la esquina inferior derecha del área de trabajo.
- **Fuente pixel: Terminus.**
  - Licencia SIL OFL 1.1.
  - Cubre ISO 8859-1, que incluye á, é, í, ó, ú, ñ, ¡ y ¿. Verificado en su web por el juego de caracteres declarado, sin revisar los glifos uno a uno.
  - Tamaños bitmap: 6×12, 8×14, 8×16, 10×18, 10×20, 11×22, 12×24, 14×28 y 16×32.
  - Alternativas: Spleen (BSD 2-Clause, con todo ISO 8859-1 salvo quizá en el tamaño 5×8) y Cozette (MIT, con la cobertura de Latin-1 sin confirmar).
- **Animación de entrada**:
  1. **Asoma** desde detrás de la barra de tareas. El sprite sube recortado justo en el borde superior de la barra, que es el borde inferior del área de trabajo, así que parece que sale de detrás sin tocar el orden de ventanas de la barra.
  2. **Salta** y aterriza de pie encima de la barra.
  - Se mueve en pasos de un píxel entero de la mascota, sin interpolar, para que siga pareciendo pixel art.
- **Pose mientras avisa**:
  - Permiso: los dos brazos arriba (`up`), y parpadea (`closed`) cada pocos segundos.
  - Pregunta: un brazo arriba (`one-up`), como quien levanta la mano.
- **Salida**, al pulsarla o al hacer la acción: guiña un ojo (`wink`) y baja.
- **Texto del bocadillo**:
  - Permiso: "¿Puedo usar <herramienta>?", por ejemplo "¿Puedo usar Bash?".
  - Pregunta: el principio de la pregunta de `AskUserQuestion`.
  - Debajo, en los dos casos: un **icono de carpeta dibujado en pixel art** y el nombre de la carpeta del proyecto.

### Dónde vive el código de la app (decidido: opción 1)

**Decidido el 09-11-2026**: un repositorio nuevo y **público** en GitHub, `notify-claude`, con **todo el proyecto**. Se creará cuando se empiece a programar.

**Opción 1 (elegida): el mismo repositorio, `notify-claude`.**

```
notify-claude/                ← repositorio y marketplace
├── .claude-plugin/marketplace.json
├── plugin/                   ← el plugin (mod) que se instala
│   └── bin/mascota.exe       ← la app ya compilada
├── desktop/                  ← código Rust de la app
├── sprites/                  ← fotogramas de la mascota, para las dos partes
└── INVESTIGACION.md
```

- Ventajas:
  - Todo en un sitio. El sprite tiene una sola fuente para el terminal y la app.
  - Si cambian los argumentos (`--kind`, `--dismiss`), se cambian los dos lados en un mismo commit.
  - Se instala con un solo comando. El plugin lleva el `.exe` y lo encuentra con `$.plugin.root`, sin configurar nada.
  - Es un único proyecto escaparate completo, con TypeScript y Rust nativo.
  - Como el marketplace apunta a `./plugin`, al instalar el plugin no se copia el código de la app.
- Inconvenientes:
  - Dos lenguajes y dos formas de compilar en el mismo repositorio.
  - Hay que copiar el `.exe` compilado a `plugin/bin`, y subir un binario al repositorio o publicarlo en una release.

**Opción 2: carpeta y repositorio propios para la app.**

- Ventajas:
  - Cada proyecto tiene su vida, sus versiones y su README.
  - La app podría servir para otras herramientas, como un aviso con mascota para cualquier programa de terminal.
  - El repositorio del plugin queda ligero y sin binarios.
  - Son dos proyectos en el portfolio.
- Inconvenientes:
  - Hay que instalar dos cosas y configurar la ruta del `.exe`.
  - El sprite queda duplicado o hay que sincronizarlo entre los dos repositorios.
  - Un cambio de argumentos hay que coordinarlo entre dos versiones.

---

## 7. La mascota en el terminal (mod)

Un **mod** es un plugin con un módulo TypeScript (`hooks/register.tsx`) que se ejecuta dentro de Claude Code y puede dibujar en su interfaz. Su `hooks/hooks.json` es `{ "modules": ["./register.tsx"] }`. Ejemplo real instalado: `sub-meter` de midudev.

### Lo que ofrece la API de mods en Windows Terminal

| Pieza | Qué hace | En Windows Terminal |
| --- | --- | --- |
| `ui.render` sobre `AbovePrompt` | Franja justo encima de la caja de escribir | ✅ |
| `Raster` | Cuadrícula de celdas con carácter, color de texto y color de fondo | ✅ solo existe en el terminal |
| `Client` | Zona con estado propio y reloj de animación (`every(ms, fn)`) | ✅ |
| Panel (`$.ui.open`) | Panel lateral fijo | ✅ a partir de unas 110-144 columnas |
| `$.ui.toast(text)` | Aviso pequeño dentro de Claude Code | ✅ |
| `$.ui.notify(text)` | Notificación por el canal nativo | ⚠️ solo pitido, sin texto ni icono |
| `$.audio.play` | Reproduce un sonido | ❌ en Windows no suena |
| `Image` | Imagen real | ❌ solo kitty, Ghostty o iTerm2; en el resto muestra el texto `alt` |
| `$.process.spawn` | Lanza un proceso | ✅ sirve para abrir la app del escritorio |
| `turn.start`, `turn.complete`, `classic.Notification` | Eventos de turno y de aviso | ✅ |

### Cómo se haría

- **Dónde**: un hook `ui.render` sobre `{ component: 'AbovePrompt' }`. Esa franja se dibuja en el terminal y en la app de escritorio de Claude. El mod comprueba `e.surface === 'terminal'` y, si no lo es, devuelve `next(e)` sin dibujar nada: así aparece **únicamente en el terminal**.
- **Cómo se dibuja**: con `Raster`, cuya entrada es una cadena base64 de tripletes `[carácter, color de texto, color de fondo]`. Con el medio bloque `▀`, cada celda da dos píxeles verticales: el de arriba es el color de texto y el de abajo, el de fondo. Así se consigue pixel art en color en Windows Terminal, con los mismos fotogramas y colores que usa Claude Code (sección 6).
- **Cómo se pasea**: con un `Client` que use su reloj (`every`) para mover la mascota unas 6–8 veces por segundo. `columns` le dice cuánto ancho tiene. Otra opción es repintar el `Raster` con `$.ui.blit`. Descansa a la derecha y camina de un lado a otro por encima de la caja de escribir. Al cambiar de dirección se da la vuelta con los fotogramas de giro de Claude Code.
- **Cómo reacciona**: `turn.start` o `isWorking` (Claude trabajando), `turn.complete` (ha terminado), `classic.Notification` (pide permiso). El estado se comparte con `atom`, `read` y `update`; cada `update` vuelve a dibujar la franja.
- **Ancho útil**: `bodyColumns`, que es el ancho del terminal menos las 5 celdas que se reserva Claude Code.

### Límites

- **No flota sobre el texto**: la franja ocupa sus propias filas (unas 3 o 4) encima de la caja de escribir, y la conversación sube esas filas.
- **El `[-]` de la derecha**: Claude Code pone un `[-]` en el extremo derecho de la franja para plegarla (también con `ctrl+x ctrl+a`). No se puede quitar, así que "a la derecha" es justo a su izquierda.
- **Las encuestas**: cuando Claude Code muestra una encuesta en esa franja (`hasSurvey`), la mascota tiene que apartarse.
- **Pantalla completa**: la franja solo tiene el espacio que quede (`maxRows`, como mucho la mitad de las filas).
- **Activación**: los mods se activan aceptando un aviso de Claude Code, o arrancando con `--plugin-dir`. No funcionan en `claude -p` ni en el SDK.
- **Fluidez**: no se sabe cómo de suave se verá la animación en Windows Terminal hasta probarla.

---

## 8. Sobre la imagen del bichito

> **Cambiado el 10-11-2026**: la mascota ya no es la de Claude Code, sino un **gato esmoquin propio** (blanco y negro, ojos amarillos). Donde las secciones 6 y 7 hablan de la mascota literal, de sus poses (`up`, `one-up`, `wink`…) o del naranja `clawd_body`, manda lo que se dice aquí.

- **Dibujo**: procedural. `sprites/gato.js` convierte una pose continua (respiración, cola, patas, ojos, orejas) en una rejilla de 58×59 píxeles, y define las animaciones a 30 fps: entrada, reposo, pregunta, permiso y salida. `node sprites/vista-previa.js` genera la vista previa en `sprites/salida/animaciones.html`.
- **App de escritorio**: `desktop/src/cat.rs` y `anim.rs` son un port literal a Rust. `sprites/paridad.txt` guarda un hash por fotograma y las pruebas de la app comprueban que los 858 salen idénticos, así que si se cambia un lado hay que cambiar el otro y regenerar ese archivo.
- **Bocadillo**: blanco roto con borde oscuro y texto en Terminus 6×12, a una escala menor que la del gato (al 100 %, el gato va a 3× y el bocadillo a 2×) para que el texto no ocupe media pantalla.
- **Terminal**: la franja da como mucho 8 píxeles de alto, así que el gato del terminal será un dibujo aparte, más simple. Se abandona la idea de un único sprite para las dos partes.
- **Marca**: al ser un dibujo propio, ya no hay que revisar las directrices de marca de Anthropic por la mascota.

---

## 9. Propuesta

1. **Plugin `notify-claude`** (un mod):
   - Dibuja la mascota en pixel art y la hace pasear encima de la caja de escribir, solo en el terminal.
   - Reacciona cuando Claude trabaja, termina o pide permiso.
   - Cuando Claude te pregunta algo o te pide permiso, abre la app del escritorio con `$.process.spawn`, salvo que la opción `desktopMascot` esté desactivada. También la abre cuando Claude termina tras trabajar un buen rato sin ti (añadido el 20-11-2026).
   - Cuando haces lo que necesitaba, la cierra con `--dismiss`.
2. **App de escritorio** (Rust + Win32 nativo; ver sección 6):
   - Se abre con cada aviso (opción A).
   - La mascota aparece y muestra un bocadillo en pixel art.
   - Se cierra al pulsar en la mascota o al recibir `--dismiss`.
   - Instancia única: el aviso nuevo sustituye al que se esté viendo, y solo se muestra el último.
3. Un gato esmoquin propio en pixel art. El del escritorio está hecho; el del terminal será un dibujo aparte, más pequeño (sección 8).

## 10. Decisiones

Tomadas:

- [x] Nada de notificación toast: la mascota aparece en el escritorio.
- [x] Mascota en el escritorio y en el terminal.
- [x] En el terminal: a la derecha, encima de la caja de escribir, paseando, y **solo** en el terminal.
- [x] Escritorio: app aparte, que solo aparece cuando avisa.
- [x] La app se abre con cada aviso y se cierra al terminar el aviso (opción A).
- [x] Se cierra al pulsar en la mascota (lo has visto) o al hacer lo que el agente necesitaba.
- [x] Mascota y bocadillo en pixel art. La mascota es un gato esmoquin propio (cambiado el 10-11-2026; antes, la mascota literal de Claude Code). Ver sección 8.
- [x] La mascota del escritorio solo avisa cuando Claude **pregunta** o **pide permiso**.
- [x] **Aviso de terminado** (20-11-2026): también avisa cuando Claude termina tras 30 s o más trabajando sin ti, con "¡He terminado!" y una animación propia, y se va sola a los 10 s (sección 6, "Aviso de terminado").
- [x] Aparece aunque estés mirando el terminal, con una opción para desactivarla.
- [x] Varios avisos a la vez: solo se muestra el último.
- [x] Posición, fuente (Terminus), animaciones de entrada, aviso y salida, y texto del bocadillo con icono de carpeta en pixel art (sección 6).
- [x] Tecnología de la app: Rust + Win32 nativo.
- [x] Las peticiones de datos de servidores MCP (`elicitation_dialog`) no avisan. Solo avisan las preguntas de Claude (`AskUserQuestion`) y los permisos.
- [x] Repositorio nuevo y público en GitHub, `notify-claude`, con todo el proyecto: plugin, app, sprites y documentación (opción 1 de la sección 6). Se crea al empezar a programar.

Pendientes:

- [x] ~~Marca de Anthropic~~: resuelto el 10-11-2026 al cambiar la mascota por un gato propio.
- [ ] **Gato del terminal**: dibujar la versión pequeña (8 píxeles de alto como mucho) para la franja de encima de la caja de escribir.
- [x] **Cómo viaja el `.exe`** (decidido el 20-11-2026): subido al repositorio en `plugin/bin/`. Se instala y funciona sin red; cada versión añade unos 180 KB al historial. Con una release, el mod tendría que descargar y lanzar un ejecutable. Tras cambiar la app hay que recompilarla y copiarla ahí (README, "Desarrollo").
- [x] **Publicación** (decidido el 20-11-2026): el propio repositorio es el marketplace (`.claude-plugin/marketplace.json`, con `source: ./plugin`), con licencia MIT. Por ahora no se envía al directorio oficial de Anthropic. El icono para ese directorio ya está hecho: `plugin/.claude-plugin/icon.png`, generado con `node sprites/icono.js`. Si se enviara, el validador avisa de que el nombre `notify-claude` "parece de Anthropic".

## 11. No verificado

- Página oficial de notificaciones (`/docs/en/notifications`): devuelve 404.
- Lista completa de valores de `preferredNotifChannel` y su valor por defecto.
- Matchers con `|` en `Notification`.
- Cuándo saltan `agent_needs_input` y `agent_completed`.
- Fluidez de la animación de la franja en Windows Terminal.
- Cómo se comportan en la práctica el `[-]`, el plegado y las encuestas con la mascota dibujada.
- Tiempo de arranque de la app en la opción A.
- En modo auto, si `classic.PermissionRequest` salta solo cuando el clasificador no ha resuelto la llamada y se enseña el diálogo, o también antes. Si salta antes, el gato asomaría un instante en llamadas que el clasificador aprueba solo.
- Cuánto tarda en salir el aviso de segundo plano de Bash. Si sale enseguida, el gato se va casi al pulsar "Permitir"; si no, se va al terminar el comando.
- Que `$.process.spawn` mantenga viva la app hasta que se despide. La documentación dice que el hijo muere al salir del bucle, al abortarse el hook o al recargarse el módulo, y nada más. Por eso, si interrumpes el turno mientras hay una pregunta, la app desaparece de golpe, sin guiño.
- Tiempos reales de arranque en este equipo de Rust + Win32 frente a Tauri: no hay datos fiables publicados.
- Si softbuffer admite transparencia por píxel en Windows.
- La cobertura de Latin-1 de Cozette, que es una de las alternativas.
- Los glifos de Terminus uno a uno: su web declara ISO 8859-1, pero no se han revisado los caracteres.
- Que el pixel art se vea nítido con el escalado de Windows al 125 % y al 150 %.
- Dibujo exacto de cada fotograma de la mascota: se han visto los nombres de las poses y parte de los caracteres, pero falta pasarlos a la rejilla de píxeles.

## Fuentes

- Manifiesto: https://code.claude.com/docs/en/plugins/manifest-reference.md
- Componentes: https://code.claude.com/docs/en/plugins/components.md
- Crear un plugin: https://code.claude.com/docs/en/plugins/create.md
- Hooks (referencia): https://code.claude.com/docs/en/hooks.md
- Hooks (guía): https://code.claude.com/docs/en/hooks-guide.md
- CLI de plugins: https://code.claude.com/docs/en/plugins/cli-reference.md
- Marketplace local: https://code.claude.com/docs/en/plugins/create-marketplace.md
- Terminal y notificaciones: https://code.claude.com/docs/en/terminal-config.md
- Status line: https://code.claude.com/docs/en/statusline.md
- Mods: https://code.claude.com/docs/en/plugins/mods/overview.md
- Tipos de la API de mods (`AbovePrompt`, `Raster`, `Client`, `$.clock`, `$.process`, `$.http`, `$.ui`): generados localmente por la skill `plugin-authoring` (Claude Code 2.1.319).
- DesktopFriend (proyecto propio): referencia de ventana transparente y click-through con Tauri, y su consumo medido (`docs/bitacora.md`, paso 140).
- Rendimiento de WebView2 (Microsoft): https://learn.microsoft.com/microsoft-edge/webview2/concepts/performance
- Cifras de arranque de Tauri en blogs (poco fiables): https://dev.to/johalputt/comparison-tauri-20-vs-flutter-40-desktop-for-cross-platform-app-performance-5cng y https://dev.to/tinyforge/tauri-cold-launch-was-1s-heres-how-i-got-it-to-feel-instant-32lh
- Ventanas *layered* con alfa por píxel y clics que atraviesan: https://learn.microsoft.com/en-us/answers/questions/1291809/how-to-use-updatelayeredwindow-for-drawing-a-bitma
- softbuffer: https://github.com/rust-windowing/softbuffer
- Terminus Font (licencia, juegos de caracteres y tamaños): https://terminus-font.sourceforge.net/
- Spleen (cobertura y licencia): https://github.com/fcambus/spleen
- Claude Code 2.1.319 instalado (`~/.local/share/claude/versions/2.1.319`): colores del tema (`clawd_body`, `clawd_background`) y nombres de las poses de la mascota.
