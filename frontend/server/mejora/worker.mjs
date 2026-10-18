// Worker Python persistente (gemini_worker.py --daemon): una sola sesión de
// Gemini para todas las fotos. Si muere o se cuelga, se descarta y el siguiente
// trabajo lo re-arranca (nuevo login solo entonces).
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { PYTHON, SCRIPT_WORKER, TMP } from './estado.mjs';
import { aArchivo } from './registro.mjs';

let hijoWorker = null;
let hechos = 0; // fotos procesadas con la sesión actual
const pendientes = new Map(); // id -> { resolve, reject, temporizador }
let buffer = '';

export function workerVivo() {
  return hijoWorker !== null && hijoWorker.exitCode === null;
}

export function workerPid() {
  return hijoWorker?.pid ?? null;
}

export function workerTrabajos() {
  return hechos;
}

export function matarWorker() {
  for (const { reject, temporizador } of pendientes.values()) {
    clearTimeout(temporizador);
    reject(new Error('El worker se cerró a mitad del trabajo.'));
  }
  pendientes.clear();
  if (hijoWorker) {
    try {
      hijoWorker.kill();
    } catch {
      // Ya estaba muerto: nada que hacer.
    }
    hijoWorker = null;
  }
  buffer = '';
}

/* Reparte las líneas JSON del stdout del worker entre las peticiones pendientes. */
function repartirLineas(trozo) {
  buffer += trozo.toString();
  const lineas = buffer.split('\n');
  buffer = lineas.pop() ?? '';
  for (const linea of lineas) {
    const t = linea.trim();
    if (!t) continue;
    let msg = null;
    try {
      msg = JSON.parse(t);
    } catch {
      continue; // No es protocolo: se ignora.
    }
    if (msg && typeof msg.id === 'string' && pendientes.has(msg.id)) {
      const { resolve, reject, temporizador } = pendientes.get(msg.id);
      pendientes.delete(msg.id);
      clearTimeout(temporizador);
      if (msg.ok) {
        hechos += 1;
        resolve(msg.estado ?? null); // F26: la sonda informa el estado (AVAILABLE o degradada).
      } else {
        reject(new Error(String(msg.error || 'El worker no pudo mejorarla.').slice(0, 300)));
      }
    }
    // {"listo":true} del arranque lo espera asegurarWorker; aquí se ignora.
  }
}

async function asegurarWorker() {
  if (hijoWorker && hijoWorker.exitCode === null) return;
  matarWorker();
  await fs.mkdir(TMP, { recursive: true });
  const hijo = spawn(PYTHON, [SCRIPT_WORKER, '--daemon'], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  hijoWorker = hijo;
  hechos = 0;
  hijo.stderr.on('data', (c) => {
    process.stderr.write(`[worker] ${c}`);
    try {
      aArchivo('consola', `[worker] ${c.toString().slice(0, 2000)}`);
    } catch {
      // Logging best-effort.
    }
  });
  hijo.stdout.on('data', repartirLineas);
  hijo.on('error', () => {
    if (hijoWorker === hijo) matarWorker();
  });
  hijo.on('exit', () => {
    if (hijoWorker === hijo) matarWorker();
  });
  // Espera el {"listo":true} (login hecho) hasta 60s.
  await new Promise((resolve, reject) => {
    const temporizador = setTimeout(() => reject(new Error('El worker no inició sesión.')), 60 * 1000);
    const alSalir = () => {
      clearTimeout(temporizador);
      reject(new Error('El worker murió al arrancar.'));
    };
    const alTrozo = (c) => {
      if (c.toString().includes('"listo"')) {
        clearTimeout(temporizador);
        hijo.off('exit', alSalir);
        hijo.stdout.off('data', alTrozo);
        resolve();
      }
    };
    hijo.once('exit', alSalir);
    hijo.stdout.on('data', alTrozo);
    hijo.once('error', (e) => {
      clearTimeout(temporizador);
      reject(e);
    });
  });
}

/* Envía una petición al worker y espera su respuesta o el plazo. Si el plazo
 * vence o la escritura falla, la sesión se descarta (posiblemente colgada). */
async function pedirAlWorker(id, carga, plazoMs, msgPlazo) {
  await asegurarWorker();
  const hijo = hijoWorker;
  if (!hijo || !hijo.stdin || hijo.stdin.destroyed) throw new Error('El worker no acepta peticiones.');
  return new Promise((resolve, reject) => {
    const temporizador = setTimeout(() => {
      pendientes.delete(id);
      matarWorker(); // Sesión posiblemente colgada: empezar de cero.
      reject(new Error(msgPlazo));
    }, plazoMs);
    pendientes.set(id, { resolve, reject, temporizador });
    hijo.stdin.write(`${JSON.stringify({ id, ...carga })}\n`, (e) => {
      if (e) {
        pendientes.delete(id);
        clearTimeout(temporizador);
        matarWorker();
        reject(e);
      }
    });
  });
}

/* Mismo contrato que la foto de entrada: data URL base64 de imagen. */
function dataUrlABytes(dataUrl) {
  const m = /^data:(image\/[a-zA-Z0-9.+-]+);base64,(.+)$/.exec(dataUrl || '');
  if (!m) return null;
  return { mime: m[1], bytes: Buffer.from(m[2], 'base64') };
}

function extensionDe(mime) {
  if (mime.includes('png')) return '.png';
  if (mime.includes('webp')) return '.webp';
  return '.jpg';
}

/* Mejora una foto y devuelve su resultado como data URL PNG. */
export async function ejecutarWorker(original, prompt) {
  await fs.mkdir(TMP, { recursive: true });
  const id = randomUUID();
  const datos = dataUrlABytes(original);
  if (!datos) throw new Error('Original no es un dataURL de imagen válido.');
  const entrada = path.join(TMP, `${id}-in${extensionDe(datos.mime)}`);
  const salida = path.join(TMP, `${id}-out.png`);
  await fs.writeFile(entrada, datos.bytes);
  try {
    await pedirAlWorker(id, { input: entrada, output: salida, prompt }, 5 * 60 * 1000, 'El worker tardó más de 5 minutos.');
    const bytes = await fs.readFile(salida);
    return `data:image/png;base64,${bytes.toString('base64')}`;
  } finally {
    await fs.rm(entrada, { force: true }).catch(() => {});
    await fs.rm(salida, { force: true }).catch(() => {});
  }
}

/* F21/F26: sonda real de credenciales sin gastar una foto (texto temporal).
 * Devuelve el estado que informa el worker: AVAILABLE o degradada. */
export function sondearSesion() {
  const id = `probar-${randomUUID()}`;
  return pedirAlWorker(id, { probar: true }, 90 * 1000, 'Sin respuesta del worker en 90 segundos.');
}
