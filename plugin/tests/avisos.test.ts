// Pruebas del mod: qué lanzaría (argumentos de mascota.exe) ante cada evento. El
// `process.spawn` de la prueba solo apunta la orden, así que no se abre ninguna ventana.

import type { On, SessionAppendInput, TurnCompleteInput } from 'claude-code'
import { expect, mock, test } from 'claude-code/testing'

type Orden = string[]

/** Apunta cada lanzamiento sin ejecutar nada (sin la ruta del .exe). Los lanzamientos
 *  salen del hook sin esperarlos, así que `llegan(n)` espera a que haya `n`. */
function capturar(on: On) {
  const ordenes: Orden[] = []
  const esperas: { n: number; listo: () => void }[] = []
  on('process.spawn', async function* (_, e) {
    expect(e.argv[0]).toMatch(/[\\/]bin[\\/]mascota\.exe$/)
    ordenes.push(e.argv.slice(1))
    for (const w of esperas.filter(w => ordenes.length >= w.n)) w.listo()
    return { value: { code: 0, signal: null } }
  })
  const llegan = (n: number) =>
    ordenes.length >= n ? Promise.resolve() : new Promise<void>(listo => esperas.push({ n, listo }))
  // lo que en una sesión responde el motor
  on('session.id', () => ({ value: 'sesion-prueba' }))
  on('session.cwd', () => ({ value: 'C:\\proyectos\\notify-claude' }))
  on('prompt.submit', (_, e) => ({ text: e.text }))
  on('turn.start', (_, e) => ({ turnId: e.turnId }))
  on('turn.complete', (_, e) => ({ text: e.answer }))
  const reloj = mock.clock(on)
  return { ordenes, llegan, reloj }
}

const sesion = (o: Orden | undefined) => o?.[o.indexOf('--session') + 1]

const prompt = (text: string) => ({ text, wait: false, origin: { kind: 'composer' } }) as const

function filaDeResultado(id: string): SessionAppendInput {
  return {
    door: 'tool-result',
    origin: { kind: 'tool', tool: 'Bash' },
    uuid: `fila-${id}`,
    message: { type: 'user', role: 'user', content: [{ type: 'tool_result', tool_use_id: id, content: 'ok' }] },
  }
}

/** El final de un turno del bucle principal que ha durado `ms`. */
const fin = (ms: number, extra: Partial<TurnCompleteInput> = {}) =>
  ({ answer: 'listo', durationMs: ms, isAborted: false, turnId: 't1', reason: 'answer', ...extra }) as TurnCompleteInput

const PREGUNTA = {
  tool: 'AskUserQuestion' as const,
  questions: [
    {
      question: '¿Qué base de datos usamos?',
      header: 'BD',
      multiSelect: false,
      options: [
        { label: 'SQLite', description: '' },
        { label: 'Postgres', description: '' },
      ],
    },
  ],
}

const BASH_OK = { result: { stdout: '', stderr: '', interrupted: false } }

test('un permiso abre el gato y solo el resultado de esa llamada lo cierra', async ($, on) => {
  const { ordenes, llegan } = capturar(on)
  const ids: string[] = []
  on('tool.check', (_, e) => {
    if (e.tool_use_id !== undefined) ids.push(e.tool_use_id)
    return { decision: 'ask' }
  })
  on('classic.PermissionRequest', () => ({ decision: { behavior: 'allow' } }))
  on('tool.run', () => BASH_OK)

  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await llegan(1)
  expect(ordenes[0]?.slice(0, 2)).toEqual(['--kind', 'permiso'])
  expect(ordenes[0]).toContain('¿Puedo usar Bash?')
  expect(ordenes[0]).toContain('--cwd')

  await $.session.append(filaDeResultado('otra'))
  await $.session.append(filaDeResultado(ids[0] ?? ''))
  await llegan(2)
  expect(ordenes).toHaveLength(2)
  expect(ordenes[1]).toEqual(['--dismiss', '--session', sesion(ordenes[0])])
})

test('al conceder un permiso de Edit se despide antes de que la herramienta termine', async ($, on) => {
  const { ordenes, llegan } = capturar(on)
  on('tool.check', () => ({ decision: 'ask' }))
  on('classic.PermissionRequest', () => ({ decision: { behavior: 'allow' } }))
  on('tool.run', async () => {
    // la herramienta "corre": para entonces el gato ya se tiene que estar despidiendo
    await llegan(2)
    return { result: { filePath: 'a.txt' } }
  })

  await $.tool.call({ tool: 'Edit', file_path: 'a.txt', old_string: 'a', new_string: 'b' })
  expect(ordenes[0]).toContain('¿Puedo usar Edit?')
  expect(ordenes[1]?.[0]).toBe('--dismiss')
})

test('una pregunta abre el gato con su texto y se despide al responder', async ($, on) => {
  const { ordenes, llegan } = capturar(on)
  on('tool.run', () => ({ result: { questions: [], answers: {} } }))

  await $.tool.call(PREGUNTA)
  await llegan(2)
  expect(ordenes[0]?.slice(0, 2)).toEqual(['--kind', 'pregunta'])
  expect(ordenes[0]).toContain('¿Qué base de datos usamos?')
  expect(ordenes[1]).toEqual(['--dismiss', '--session', sesion(ordenes[0])])
})

test('escribir otro prompt despide el aviso, y sin aviso no despide nada', async ($, on) => {
  const { ordenes, llegan } = capturar(on)
  on('tool.check', () => ({ decision: 'ask' }))
  on('classic.PermissionRequest', () => ({ decision: { behavior: 'deny', message: 'no' } }))

  await $.tool.call({ tool: 'mcp__lab__ping' })
  await llegan(1)
  expect(ordenes[0]).toContain('¿Puedo usar ping?')

  await $.prompt.submit(prompt('otra cosa'))
  await llegan(2)
  expect(ordenes[1]?.[0]).toBe('--dismiss')

  // sin nada en pantalla, otro prompt no lanza nada: lo siguiente que llega es el aviso nuevo
  await $.prompt.submit(prompt('y otra'))
  await $.tool.call({ tool: 'mcp__lab__ping' })
  await llegan(3)
  expect(ordenes[2]?.[0]).toBe('--kind')
})

test('con el gato desactivado no se lanza nada', { options: { desktopMascot: false } }, async ($, on) => {
  const { ordenes } = capturar(on)
  on('tool.check', () => ({ decision: 'ask' }))
  on('classic.PermissionRequest', () => ({ decision: { behavior: 'allow' } }))
  on('tool.run', () => BASH_OK)

  await $.tool.call({ tool: 'Bash', command: 'ls' })
  expect(ordenes).toEqual([])
})

test('tras un turno largo el gato avisa de que ha terminado, y otro prompt lo despide', async ($, on) => {
  const { ordenes, llegan, reloj } = capturar(on)

  await $.turn.start({ text: 'refactoriza', turnId: 't1' })
  await reloj.advance(45_000)
  await $.turn.complete(fin(45_000))
  await llegan(1)
  expect(ordenes[0]).toEqual(['--kind', 'terminado', '--session', 'sesion-prueba', '--message', '¡He terminado!', '--cwd', 'C:\\proyectos\\notify-claude'])

  await $.prompt.submit(prompt('gracias'))
  await llegan(2)
  expect(ordenes[1]).toEqual(['--dismiss', '--session', 'sesion-prueba'])
})

test('un turno corto, interrumpido, fallido o de un subagente no avisa', async ($, on) => {
  const { ordenes, reloj } = capturar(on)

  for (const extra of [{}, { reason: 'aborted', isAborted: true }, { reason: 'error' }, { agentId: 'a1' }] as const) {
    const largo = Object.keys(extra).length > 0
    await $.turn.start({ text: 'x', turnId: 't1', ...('agentId' in extra ? extra : {}) })
    await reloj.advance(largo ? 120_000 : 10_000)
    await $.turn.complete(fin(largo ? 120_000 : 10_000, extra))
  }
  await reloj.settle()
  expect(ordenes).toEqual([])
})

test('el rato que tardas en responder una pregunta no cuenta como trabajo', async ($, on) => {
  const { ordenes, llegan, reloj } = capturar(on)
  let responder = () => {}
  const respondida = new Promise<void>(r => (responder = r))
  on('tool.run', async () => {
    await respondida
    return { result: { questions: [], answers: {} } }
  })

  await $.turn.start({ text: 'x', turnId: 't1' })
  await reloj.advance(20_000)
  const llamada = $.tool.call(PREGUNTA)
  await llegan(1)
  await reloj.advance(300_000) // cinco minutos sin contestar
  responder()
  await llamada
  await llegan(2)
  expect(ordenes[1]?.[0]).toBe('--dismiss')

  // 20 s antes de preguntar y 15 s después: ninguno de los dos tramos llega al mínimo
  await reloj.advance(15_000)
  await $.turn.complete(fin(335_000))
  await reloj.settle()
  expect(ordenes).toHaveLength(2)
})
