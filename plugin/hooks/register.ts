// notify-claude: abre el gato del escritorio (bin/mascota.exe) cuando Claude Code te hace
// una pregunta o te pide permiso, y lo despide en cuanto has respondido. También avisa
// cuando Claude termina después de trabajar un buen rato sin ti.
//
// Cuándo se abre:
//   - Pregunta: `tool.call` de AskUserQuestion, antes de que salga el diálogo.
//   - Permiso: `classic.PermissionRequest`, que salta justo cuando se enseña el diálogo
//     (en modo auto, solo si el clasificador no lo ha resuelto).
//   - Terminado: `turn.complete` del bucle principal con respuesta, si Claude llevaba al menos
//     MIN_TRABAJO_MS trabajando solo: desde que empezó el turno o desde lo último que
//     respondiste (una pregunta, un permiso o un mensaje mientras trabajaba).
//
// Cuándo se cierra, lo primero que llegue:
//   - Pregunta: el `next(e)` de su `tool.call` resuelve, porque ya has respondido.
//   - Permiso concedido: `tool.run` de esa llamada, o el aviso "ctrl+b to run in background"
//     de Bash, que solo sale cuando el comando ya corre.
//   - Permiso concedido o rechazado: la fila con el resultado de esa llamada (`session.append`).
//   - Terminado: la app se despide sola al rato.
//   - Cualquiera: escribes otro prompt, termina el turno (sin aviso de terminado) o se
//     cierra la sesión.
//
// Para saber de qué llamada es cada diálogo, `tool.check` apunta las que el motor va a
// preguntar, con su id: `classic.PermissionRequest` no lo trae.

import type { EngineInterface, Register } from 'claude-code'

type Tipo = 'pregunta' | 'permiso' | 'terminado'

/** Un hook de `tool.run` deja sin "mandar a segundo plano" a las herramientas que oye, así
 *  que solo se escuchan las que no se pueden mandar. Bash y PowerShell se cierran con su
 *  aviso de segundo plano o con su resultado. */
const SIN_SEGUNDO_PLANO = /^(Edit|Write|MultiEdit|NotebookEdit|WebFetch|WebSearch|mcp__.+)$/

/** La app recorta el bocadillo a 3 líneas; esto solo evita mandar textos enormes. */
const MAX_MENSAJE = 300

/** Lo que tiene que trabajar Claude sin ti para avisar de que ha terminado. */
const MIN_TRABAJO_MS = 30_000

/** El aviso que esta sesión tiene en pantalla, y la llamada que lo cierra (si se sabe).
 *  Una recarga del módulo lo pierde, pero también mata la app que hubiera lanzado. */
let visible: { session: string; id: string | undefined } | undefined

/** Llamadas a las que el motor va a pedir permiso, por orden de llegada. */
const pendientes = new Map<string, { tool: string; input: string }>()

/** Desde cuándo trabaja Claude sin ti en el turno principal (`$.clock.now()`). */
let desde: number | undefined

export const register: Register = (on, options) => {
  if (options.desktopMascot === false) return

  on('tool.check', async ($, e, next) => {
    const veredicto = await next(e)
    if (veredicto.decision === 'ask' && e.tool_use_id !== undefined && e.tool !== 'AskUserQuestion') {
      pendientes.set(e.tool_use_id, { tool: e.tool, input: claveDe(e.input) })
    }
    return veredicto
  })

  on('classic.PermissionRequest', ($, e, next) => {
    // AskUserQuestion también pasa por aquí: su diálogo es el de la pregunta.
    if (e.tool_name !== 'AskUserQuestion') {
      mostrar($, 'permiso', textoPermiso(e.tool_name), e.session_id, e.cwd, emparejar(e.tool_name, e.tool_input))
    }
    return next(e)
  })

  on('tool.call', { tool: 'AskUserQuestion' }, async ($, e, next) => {
    const [session, cwd] = await Promise.all([$.session.id(), $.session.cwd()])
    mostrar($, 'pregunta', e.questions[0]?.question ?? '', session, cwd, e.tool_use_id)
    try {
      return await next(e)
    } finally {
      resuelta($, e.tool_use_id)
    }
  })

  on('tool.run', { tool: SIN_SEGUNDO_PLANO }, ($, e, next) => {
    resuelta($, e.tool_use_id)
    return next(e)
  })

  on('ui.render', { component: 'ToolProgress' }, ($, e, next) => {
    resuelta($, e.props.tool_use_id)
    return next(e)
  })

  on('session.append', { door: 'tool-result' }, ($, e, next) => {
    for (const id of idsDeResultados(e.message.content)) resuelta($, id)
    return next(e)
  })

  on('prompt.submit', async ($, e, next) => {
    despedir($)
    desde = await $.clock.now()
    return next(e)
  })

  on('turn.start', async ($, e, next) => {
    if (e.agentId === undefined) desde = await $.clock.now()
    return next(e)
  })

  on('turn.complete', async ($, e, next) => {
    if (e.agentId === undefined) {
      // tras una recarga del módulo no se sabe cuándo respondiste: cuenta el turno entero
      const trabajo = desde === undefined ? e.durationMs : (await $.clock.now()) - desde
      desde = undefined
      if (e.reason === 'answer' && trabajo >= MIN_TRABAJO_MS) {
        const [session, cwd] = await Promise.all([$.session.id(), $.session.cwd()])
        mostrar($, 'terminado', '¡He terminado!', session, cwd, undefined)
      } else {
        despedir($)
      }
    }
    return next(e)
  })

  on('session.end', ($, e, next) => {
    despedir($)
    return next(e)
  })
}

function mostrar($: EngineInterface, tipo: Tipo, mensaje: string, session: string, cwd: string, id: string | undefined) {
  visible = { session, id }
  lanzar($, ['--kind', tipo, '--session', session, '--message', mensaje.slice(0, MAX_MENSAJE), '--cwd', cwd])
}

function despedir($: EngineInterface) {
  if (visible === undefined) return
  const { session } = visible
  visible = undefined
  lanzar($, ['--dismiss', '--session', session])
}

/** La llamada `id` ya no espera al programador. Si era la del aviso, has respondido tú:
 *  Claude vuelve a trabajar sin ti desde ahora. */
function resuelta($: EngineInterface, id: string) {
  pendientes.delete(id)
  if (visible?.id !== id) return
  despedir($)
  void $.clock.now().then(t => {
    if (desde !== undefined) desde = t
  })
}

/** Lanza la app con `args`. La app no escribe nada: el bucle solo la mantiene viva, porque
 *  salir de él la mataría. Termina sola al despedirse, o al instante si ya había otra abierta
 *  (le pasa el aviso y sale). */
function lanzar($: EngineInterface, args: string[]) {
  const root = $.plugin.root.replace(/[\\/]+$/, '')
  const sep = root.includes('\\') ? '\\' : '/'
  const exe = [root, 'bin', 'mascota.exe'].join(sep)
  void (async () => {
    try {
      for await (const _ of $.process.spawn({ argv: [exe, ...args] })) {
        // nada
      }
    } catch (err) {
      $.ui.log(`no se pudo lanzar ${exe}: ${err instanceof Error ? err.message : String(err)}`, { to: 'debug' })
    }
  })()
}

/** El id de la llamada que pide este permiso: la primera pendiente con la misma herramienta
 *  y la misma entrada, o, si ninguna coincide en todo, la primera con la misma herramienta. */
function emparejar(tool: string, input: unknown): string | undefined {
  const clave = claveDe(input)
  let elegida: string | undefined
  for (const [id, p] of pendientes) {
    if (p.tool !== tool) continue
    if (p.input === clave) {
      elegida = id
      break
    }
    elegida ??= id
  }
  if (elegida !== undefined) pendientes.delete(elegida)
  return elegida
}

function textoPermiso(tool: string): string {
  if (tool === 'ExitPlanMode') return '¿Te parece bien el plan?'
  // mcp__<servidor>__<herramienta>: basta con la herramienta
  const nombre = tool.startsWith('mcp__') ? (tool.split('__').at(-1) ?? tool) : tool
  return `¿Puedo usar ${nombre}?`
}

function claveDe(input: unknown): string {
  try {
    return JSON.stringify(input) ?? ''
  } catch {
    return ''
  }
}

function idsDeResultados(content: unknown): string[] {
  if (!Array.isArray(content)) return []
  const ids: string[] = []
  for (const block of content) {
    if (typeof block === 'object' && block !== null && block.type === 'tool_result' && typeof block.tool_use_id === 'string') {
      ids.push(block.tool_use_id)
    }
  }
  return ids
}
