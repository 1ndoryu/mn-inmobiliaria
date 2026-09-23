// Mini-backend local de mejora (127.0.0.1:3122).
// - Custodia GEMINI_PSID/GEMINI_PSIDTS: nunca llegan al navegador.
// - Cola secuencial (concurrencia 1) con intervalo+jitter+tope diario+backoff.
// - F19: reintentos automáticos por trabajo (backoff 1-2-4-8-16m, tope 5);
//   sin credenciales o con tope diario la cola espera, no falla trabajos.
// - F18: GET /api/estado (snapshot), GET /api/eventos (últimos eventos),
//   POST /api/mejora/:jobId/reintentar; logs diarios en logs/ del proyecto.
// - F21: POST /api/probar (sonda real de cookies sin gastar foto),
//   POST /api/mejora/:jobId/cancelar, POST /api/cola/reiniciar (empieza de
//   nuevo sin tocar lo lista); POST /api/mejora no duplica por foto.
// - F22: `npm run renovar-cookies` reescribe .env.local desde tu Chrome y el
//   backend recarga GEMINI_PSID/PSIDTS en caliente (sin reiniciar).
// - Reutiliza un worker Python persistente (una sola sesión de Gemini para
//   todas las fotos; si muere o se cuelga, se re-arranca en el siguiente trabajo).
//
// Arranque: `npm run server:mejora` (los temporales van a C:\tmp, nunca al repo).
// Sin dependencias: solo node:http/fs/crypto/child_process.

import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';
import { promises as fs, createWriteStream, watch } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const AQUI = path.dirname(fileURLToPath(import.meta.url));
const RAIZ = path.dirname(AQUI);
const TMP = path.join('C:\\tmp', 'inmobiliaria-mejora');
/* Logs en logs/ del proyecto (gitignored): en C:\tmp los borraría la purga
   horaria. Los temporales del worker (PNG entrada/salida, se borran solos)
   sí se quedan en C:\tmp. */
const LOG_DIR = path.join(RAIZ, 'logs');

/* .env.local mínimo: KEY=valor, sin comillas raras, sin exponer nada en logs. */
async function leerMapaEnv() {
  const mapa = new Map();
  try {
    const texto = await fs.readFile(path.join(RAIZ, '.env.local'), 'utf8');
    for (const linea of texto.split('\n')) {
      const t = linea.trim();
      if (!t || t.startsWith('#') || !t.includes('=')) continue;
      const i = t.indexOf('=');
      const k = t.slice(0, i).trim();
      let v = t.slice(i + 1).trim();
      if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) v = v.slice(1, -1);
      if (k) mapa.set(k, v);
    }
  } catch {
    // Sin .env.local: /salud dirá que faltan credenciales. No se cae.
  }
  return mapa;
}

async function cargarEnvLocal() {
  const mapa = await leerMapaEnv();
  for (const [k, v] of mapa) {
    if (!(k in process.env)) process.env[k] = v;
  }
}

/* F22: recarga en caliente de credenciales (npm run renovar-cookies reescribe
 * .env.local y el watcher avisa). Solo toca GEMINI_PSID/PSIDTS; si cambian,
 * la sesión del worker queda obsoleta: se descarta en cuanto no haya trabajo
 * en vuelo para que el siguiente login use las nuevas. Sin reiniciar nada. */
let recargaPendiente = false;

async function recargarCredenciales() {
  const mapa = await leerMapaEnv();
  let cambio = false;
  for (const k of ['GEMINI_PSID', 'GEMINI_PSIDTS']) {
    const nuevo = mapa.get(k) ?? '';
    if (nuevo && nuevo !== (process.env[k] ?? '')) {
      process.env[k] = nuevo;
      cambio = true;
    }
    // Sin valor en el fichero: no se borra el actual (podría venir de entorno).
  }
  if (!cambio) return;
  fallosSeguidos = 0; // Credenciales nuevas: el backoff global no las arrastra.
  ultimoAvisoCreds = 0;
  evento('credenciales-recargadas', null, null, 'Cookies renovadas desde .env.local: la cola continúa sin reiniciar.');
  if (jobEnVuelo) {
    recargaPendiente = true; // El worker se recicla al terminar el vuelo.
  } else {
    matarWorker();
  }
  void bombear();
}

function vigilarEnvLocal() {
  try {
    let temporizador = null;
    watch(path.join(RAIZ, '.env.local'), () => {
      if (temporizador) clearTimeout(temporizador);
      temporizador = setTimeout(() => void recargarCredenciales(), 800);
    });
  } catch {
    // Sin watcher (permisos): renovar-cookies sigue funcionando con reinicio.
  }
}

function entero(env, defecto, min, max) {
  const n = Number(process.env[env]);
  if (!Number.isFinite(n)) return defecto;
  return Math.min(max, Math.max(min, Math.floor(n)));
}

const PUERTO = entero('MEJORA_PORT', 3122, 1024, 65535);
/* Ritmo y topes en caliente: el frontal los ajusta con POST /api/config al
 * guardar (lo que el usuario pone en la UI manda; los env son el defecto
 * inicial). `const` impediría aplicarlos sin reiniciar. */
let INTERVALO_SEG = entero('MEJORA_INTERVALO_SEG', 120, 30, 3600);
let JITTER_PCT = entero('MEJORA_JITTER_PCT', 30, 0, 50);
let MAX_POR_DIA = entero('MEJORA_MAX_DIA', 40, 1, 200);
/* F19: intentos máximos por trabajo antes de darlo por perdido (1-20). */
const MAX_INTENTOS = entero('MEJORA_MAX_INTENTOS', 5, 1, 20);
const PYTHON = process.env.MEJORA_PYTHON || 'python';

/* F18+F19: registro por trabajo. `estado` es encolado|procesando|lista|error.
 * `motivo` guarda el último fallo aunque el trabajo siga vivo (reintento
 * programado); `error` solo se fija en error final. `original` se libera
 * (null) al terminar para no retener MBs en memoria. */
const trabajos = new Map(); // jobId -> registro
const cola = []; // jobIds en orden de llegada
let procesando = false;
/* F21: trabajo que el worker está generando ahora mismo (si lo hay). Sirve
 * para no tocarlo al reiniciar la cola y para descartar su resultado si se
 * cancela en pleno vuelo. */
let jobEnVuelo = null;
let ultimoFin = 0;
let fallosSeguidos = 0;
let dia = new Date().toISOString().slice(0, 10);
let procesadosHoy = 0;
let avisadoTope = false;
let ultimoAvisoCreds = 0;

/* F18: ficheros diarios en logs/ del proyecto (gitignored, nunca en C:\tmp):
 * consola-AAAA-MM-DD.log (todo lo que sale por consola, incluido [worker]) y
 * eventos-AAAA-MM-DD.log (JSONL con encolado/inicio/lista/reintento/...).
 * Todo best-effort: si el disco falla, el servidor sigue funcionando. */
const flujos = new Map(); // clave -> WriteStream
function flujoDia(nombre) {
  const clave = `${nombre}-${hoy()}`;
  const actual = flujos.get(clave);
  if (actual) return actual;
  for (const f of flujos.values()) {
    try {
      f.end();
    } catch {
      // Ya cerrado: nada que hacer.
    }
  }
  flujos.clear();
  try {
    const f = createWriteStream(path.join(LOG_DIR, `${clave}.log`), { flags: 'a' });
    f.on('error', () => {
      flujos.delete(clave); // Se reintenta en la próxima escritura.
    });
    flujos.set(clave, f);
    return f;
  } catch {
    return null;
  }
}

function aArchivo(nombre, linea) {
  try {
    flujoDia(nombre)?.write(`${new Date().toISOString()} ${linea}\n`);
  } catch {
    // El logging nunca rompe el servidor.
  }
}

function textoPlano(args) {
  return args
    .map((a) => {
      if (typeof a === 'string') return a;
      try {
        return JSON.stringify(a);
      } catch {
        return '[no serializable]';
      }
    })
    .join(' ');
}

/* Espejo de consola a fichero: lo que antes solo se veía en el terminal
 * queda en consola-AAAA-MM-DD.log para leerlo después. */
const consolaOrig = { log: console.log.bind(console), error: console.error.bind(console), warn: console.warn.bind(console) };
console.log = (...args) => {
  consolaOrig.log(...args);
  aArchivo('consola', textoPlano(args));
};
console.error = (...args) => {
  consolaOrig.error(...args);
  aArchivo('consola', `[error] ${textoPlano(args)}`);
};
console.warn = (...args) => {
  consolaOrig.warn(...args);
  aArchivo('consola', `[warn] ${textoPlano(args)}`);
};

/* Eventos de cola: buffer en memoria (últimos 500) + JSONL en disco. */
const eventos = [];
function evento(tipo, jobId, fotoId, detalle) {
  const e = { t: new Date().toISOString(), tipo, jobId: jobId ?? null, fotoId: fotoId ?? null, detalle: detalle ?? null };
  eventos.push(e);
  if (eventos.length > 500) eventos.splice(0, eventos.length - 500);
  aArchivo('eventos', JSON.stringify(e));
}

function hoy() {
  return new Date().toISOString().slice(0, 10);
}

function esperaMs() {
  // Intervalo base + jitter + backoff exponencial (2m,4m,8m... techo 60m).
  const base = INTERVALO_SEG * 1000;
  const jitter = base * (JITTER_PCT / 100) * Math.random();
  const backoff = fallosSeguidos <= 0 ? 0 : Math.min(60 * 60 * 1000, 2 * 60 * 1000 * 2 ** (fallosSeguidos - 1));
  return Math.floor(base + jitter + backoff);
}

function listo() {
  return Boolean(process.env.GEMINI_PSID);
}

/* F19: backoff por trabajo 1-2-4-8-16m (techo 30m). `fallos` = intentos ya consumidos. */
function backoffReintentoMs(fallos) {
  return Math.min(30 * 60 * 1000, 60 * 1000 * 2 ** Math.max(0, fallos - 1));
}

/* Fallos que ningún reintento arregla: van directos a error final sin
 * quemar intentos ni esperar backoff. */
function esFinal(mensaje) {
  return /dataURL|20 ?MB|GEMINI_PSID/i.test(mensaje || '');
}

/* Sin credenciales o con tope diario la cola ESPERA (no falla trabajos).
 * Avisos con throttle para no llenar el log de eventos. */
function avisarSinCreds() {
  if (Date.now() - ultimoAvisoCreds < 3600 * 1000) return;
  ultimoAvisoCreds = Date.now();
  evento('sin-credenciales', null, null, 'Falta GEMINI_PSID en .env.local: la cola espera sin fallar trabajos. Renueva con npm run renovar-cookies.');
}

function avisarTope() {
  if (avisadoTope) return;
  avisadoTope = true;
  evento('tope-diario', null, null, `Tope ${procesadosHoy}/${MAX_POR_DIA}: la cola sigue y continúa tras medianoche.`);
}

/* El mapa de trabajos es memoria viva: se conservan los 200 más recientes. */
function podarTrabajos() {
  if (trabajos.size <= 200) return;
  for (const [id, t] of trabajos) {
    if (trabajos.size <= 200) break;
    if (t.estado === 'lista' || t.estado === 'error') trabajos.delete(id);
  }
}

function json(res, codigo, cuerpo) {
  const texto = JSON.stringify(cuerpo);
  res.writeHead(codigo, {
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': 'http://127.0.0.1:5199',
    'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  });
  res.end(texto);
}

function leerCuerpo(req, limiteBytes = 20 * 1024 * 1024) {
  return new Promise((resolve, reject) => {
    const partes = [];
    let total = 0;
    req.on('data', (c) => {
      total += c.length;
      if (total > limiteBytes) {
        reject(new Error('La foto supera los 20MB.'));
        req.destroy();
        return;
      }
      partes.push(c);
    });
    req.on('end', () => resolve(Buffer.concat(partes).toString('utf8')));
    req.on('error', reject);
  });
}

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

/* Worker daemon persistente: una sola sesión de Gemini para todas las fotos.
 * El hijo vive mientras responda; si muere o se cuelga, se descarta y el
 * siguiente trabajo lo re-arranca (nuevo login solo entonces). */
let hijoWorker = null;
let workerTrabajos = 0; // fotos procesadas con la sesión actual
const workerPendientes = new Map(); // id -> { resolve, reject, temporizador }
let buferWorker = '';

function matarWorker() {
  for (const { reject, temporizador } of workerPendientes.values()) {
    clearTimeout(temporizador);
    reject(new Error('El worker se cerró a mitad del trabajo.'));
  }
  workerPendientes.clear();
  if (hijoWorker) {
    try {
      hijoWorker.kill();
    } catch {
      // Ya estaba muerto: nada que hacer.
    }
    hijoWorker = null;
  }
  buferWorker = '';
}

function repartirLineasWorker(trozo) {
  buferWorker += trozo.toString();
  const lineas = buferWorker.split('\n');
  buferWorker = lineas.pop() ?? '';
  for (const linea of lineas) {
    const t = linea.trim();
    if (!t) continue;
    let msg = null;
    try {
      msg = JSON.parse(t);
    } catch {
      continue; // No es protocolo: se ignora.
    }
    if (msg && typeof msg.id === 'string' && workerPendientes.has(msg.id)) {
      const { resolve, reject, temporizador } = workerPendientes.get(msg.id);
      workerPendientes.delete(msg.id);
      clearTimeout(temporizador);
      if (msg.ok) {
        workerTrabajos += 1;
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
  const hijo = spawn(PYTHON, [path.join(AQUI, 'gemini_worker.py'), '--daemon'], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  hijoWorker = hijo;
  workerTrabajos = 0;
  hijo.stderr.on('data', (c) => {
    process.stderr.write(`[worker] ${c}`);
    try {
      aArchivo('consola', `[worker] ${c.toString().slice(0, 2000)}`);
    } catch {
      // Logging best-effort.
    }
  });
  hijo.stdout.on('data', repartirLineasWorker);
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

async function ejecutarWorker(original, prompt) {
  await fs.mkdir(TMP, { recursive: true });
  const id = randomUUID();
  const datos = dataUrlABytes(original);
  if (!datos) throw new Error('Original no es un dataURL de imagen válido.');
  const entrada = path.join(TMP, `${id}-in${extensionDe(datos.mime)}`);
  const salida = path.join(TMP, `${id}-out.png`);
  await fs.writeFile(entrada, datos.bytes);
  try {
    await asegurarWorker();
    const hijo = hijoWorker;
    if (!hijo || !hijo.stdin || hijo.stdin.destroyed) throw new Error('El worker no acepta peticiones.');
    await new Promise((resolve, reject) => {
      const temporizador = setTimeout(() => {
        workerPendientes.delete(id);
        matarWorker(); // Sesión posiblemente colgada: empezar de cero.
        reject(new Error('El worker tardó más de 5 minutos.'));
      }, 5 * 60 * 1000);
      workerPendientes.set(id, { resolve, reject, temporizador });
      hijo.stdin.write(`${JSON.stringify({ id, input: entrada, output: salida, prompt })}\n`, (e) => {
        if (e) {
          workerPendientes.delete(id);
          clearTimeout(temporizador);
          matarWorker();
          reject(e);
        }
      });
    });
    const bytes = await fs.readFile(salida);
    return `data:image/png;base64,${bytes.toString('base64')}`;
  } finally {
    await fs.rm(entrada, { force: true }).catch(() => {});
    await fs.rm(salida, { force: true }).catch(() => {});
  }
}

/* Bomba secuencial (concurrencia 1) con reintentos por trabajo:
 * - Solo coge trabajos `encolado` con el reintento ya vencido.
 * - Fallo reintentable -> intentos+1, vuelve a `encolado` con backoff y al
 *   final de la cola (los demás pasan primero). Nada queda en error por un
 *   fallo temporal.
 * - Fallo final (causa no recuperable o intentos > MAX_INTENTOS) -> `error`.
 * - Sin credenciales o con tope diario -> la cola espera, no falla trabajos.
 * - Tras la espera del throttle se re-evalúa todo (pudo llegar medianoche). */
function trabajoVencido(id, ahora) {
  const t = trabajos.get(id);
  return Boolean(t && t.estado === 'encolado' && (t.proximoReintento ?? 0) <= ahora);
}

async function bombear() {
  // F22: cookies renovadas en pleno vuelo: recicla la sesión obsoleta en
  // cuanto queda libre para que el siguiente login use las nuevas.
  if (recargaPendiente && !jobEnVuelo) {
    recargaPendiente = false;
    matarWorker();
  }
  if (procesando) return;
  if (hoy() !== dia) {
    dia = hoy();
    procesadosHoy = 0;
    avisadoTope = false;
    evento('nuevo-dia', null, null, 'Contador diario a cero: la cola continúa.');
  }
  if (!cola.some((id) => trabajoVencido(id, Date.now()))) return;
  if (!listo()) {
    avisarSinCreds();
    return;
  }
  if (procesadosHoy >= MAX_POR_DIA) {
    avisarTope();
    return;
  }
  procesando = true;
  try {
    const espera = ultimoFin === 0 ? 0 : Math.max(0, ultimoFin + esperaMs() - Date.now());
    if (espera > 0) await new Promise((r) => setTimeout(r, espera));
    const ahora = Date.now();
    const idx = cola.findIndex((id) => trabajoVencido(id, ahora));
    if (idx === -1) return;
    if (!listo()) {
      avisarSinCreds();
      return;
    }
    if (procesadosHoy >= MAX_POR_DIA) {
      avisarTope();
      return;
    }
    const jobId = cola.splice(idx, 1)[0];
    const tarea = trabajos.get(jobId);
    if (!tarea) return;
    tarea.estado = 'procesando';
    tarea.motivo = null;
    evento('inicio', jobId, tarea.fotoId, `Intento ${tarea.intentos + 1}/${MAX_INTENTOS}.`);
    jobEnVuelo = jobId;
    try {
      const imagen = await ejecutarWorker(tarea.original, tarea.prompt);
      if (tarea.estado === 'cancelado') {
        // Se canceló en pleno vuelo: el resultado se descarta, no se guarda.
        evento('cancelado', jobId, tarea.fotoId, 'Resultado descartado: se canceló en proceso.');
        podarTrabajos();
      } else {
        tarea.estado = 'lista';
        tarea.imagen = imagen;
        tarea.finEn = Date.now();
        tarea.original = null; // Libera MBs: ya no hace falta.
        procesadosHoy += 1;
        fallosSeguidos = 0;
        evento('lista', jobId, tarea.fotoId, null);
        podarTrabajos();
      }
    } catch (e) {
      if (tarea.estado === 'cancelado') {
        // Cancelado en proceso (worker descartado): no quema intento ni
        // ensucia el backoff global.
        evento('cancelado', jobId, tarea.fotoId, 'Cancelado en proceso.');
        podarTrabajos();
      } else {
        fallosSeguidos += 1;
        const msg = (e instanceof Error ? e.message : 'Error desconocido del worker.').slice(0, 300);
        tarea.intentos += 1;
        if (esFinal(msg) || tarea.intentos > MAX_INTENTOS) {
          tarea.estado = 'error';
          tarea.error = msg;
          tarea.motivo = 'final';
          tarea.finEn = Date.now();
          /* Se conserva el original: permite reintentar sin re-subir MBs
           * (POST /api/mejora/:jobId/reintentar). La poda acota la memoria. */
          evento('error-final', jobId, tarea.fotoId, `${msg} (${tarea.intentos} intentos).`);
          podarTrabajos();
        } else {
          const esperaR = backoffReintentoMs(tarea.intentos);
          tarea.estado = 'encolado';
          tarea.motivo = msg;
          tarea.proximoReintento = Date.now() + esperaR;
          cola.push(jobId);
          evento(
            'reintento',
            jobId,
            tarea.fotoId,
            `Intento ${tarea.intentos}/${MAX_INTENTOS} falló: ${msg}. Próximo en ~${Math.max(1, Math.round(esperaR / 60000))} min.`,
          );
        }
      }
    } finally {
      ultimoFin = Date.now();
      if (jobEnVuelo === jobId) jobEnVuelo = null;
    }
  } finally {
    procesando = false;
    void bombear();
  }
}

const servidor = createServer(async (req, res) => {
  const url = new URL(req.url || '/', 'http://127.0.0.1');
  if (req.method === 'OPTIONS') {
    res.writeHead(204, {
      'Access-Control-Allow-Origin': 'http://127.0.0.1:5199',
      'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type',
    });
    res.end();
    return;
  }
  if (req.method === 'GET' && url.pathname === '/api/salud') {
    json(res, 200, {
      ok: true,
      listo: listo(),
      detalle: listo() ? null : 'Falta GEMINI_PSID en .env.local del backend (renueva con npm run renovar-cookies).',
      cola: cola.length + (procesando ? 1 : 0),
      procesadosHoy,
      intervaloSeg: INTERVALO_SEG,
      jitterPct: JITTER_PCT,
      maxPorDia: MAX_POR_DIA,
      reintentosPendientes: [...trabajos.values()].filter((t) => t.estado === 'encolado' && t.motivo).length,
      fallosSeguidos,
      worker: { vivo: hijoWorker !== null && hijoWorker.exitCode === null, trabajos: workerTrabajos },
    });
    return;
  }
  /* F18: snapshot completo para el panel de diagnóstico y el MCP. */
  if (req.method === 'GET' && url.pathname === '/api/estado') {
    if (hoy() !== dia) {
      dia = hoy();
      procesadosHoy = 0;
      avisadoTope = false;
    }
    const ahora = Date.now();
    const ficha = (id) => {
      const t = trabajos.get(id);
      if (!t) return null;
      return {
        jobId: id,
        fotoId: t.fotoId,
        estado: t.estado,
        encoladoHaceSeg: Math.max(0, Math.round((ahora - t.encoladoEn) / 1000)),
        intentos: t.intentos,
        proximoReintentoEnSeg: t.proximoReintento ? Math.max(0, Math.round((t.proximoReintento - ahora) / 1000)) : null,
        motivo: t.motivo,
      };
    };
    const enCola = cola.map(ficha).filter(Boolean);
    let activo = null;
    for (const [id, t] of trabajos) {
      if (t.estado === 'procesando') {
        activo = ficha(id);
        break;
      }
    }
    json(res, 200, {
      ok: true,
      listo: listo(),
      detalle: !listo()
        ? 'Falta GEMINI_PSID en .env.local del backend (renueva con npm run renovar-cookies).'
        : procesadosHoy >= MAX_POR_DIA
          ? `Tope diario ${procesadosHoy}/${MAX_POR_DIA}: continúa tras medianoche.`
          : null,
      cola: enCola,
      activo,
      worker: { vivo: hijoWorker !== null && hijoWorker.exitCode === null, trabajos: workerTrabajos, pid: hijoWorker?.pid ?? null },
      procesadosHoy,
      maxPorDia: MAX_POR_DIA,
      maxIntentos: MAX_INTENTOS,
      intervaloSeg: INTERVALO_SEG,
      jitterPct: JITTER_PCT,
      fallosSeguidos,
      /* Ojo: esperaMs() consume un sorteo de jitter; es solo orientativo. */
      proximoArranqueEnSeg:
        procesando || enCola.length === 0 ? null : Math.max(0, Math.round((ultimoFin + esperaMs() - ahora) / 1000)),
    });
    return;
  }
  /* F18: últimos eventos de la cola (también en eventos-AAAA-MM-DD.log). */
  if (req.method === 'GET' && url.pathname === '/api/eventos') {
    const n = Math.min(500, Math.max(1, Number(url.searchParams.get('ultimos')) || 200));
    json(res, 200, { eventos: eventos.slice(-n) });
    return;
  }
  /* Ritmo y topes desde la UI (lo que el usuario pone en Configuración
   * manda en caliente; los env quedan como defecto inicial). Rangos iguales
   * a los del frontal: intervalo 30-3600 s, jitter 0-50 %, tope 1-200/día. */
  if (req.method === 'POST' && url.pathname === '/api/config') {
    try {
      const cuerpo = JSON.parse(await leerCuerpo(req));
      const num = (v, defecto, min, max) => {
        const n = Number(v);
        if (!Number.isFinite(n)) return defecto;
        return Math.min(max, Math.max(min, Math.floor(n)));
      };
      INTERVALO_SEG = num(cuerpo?.intervaloSeg, INTERVALO_SEG, 30, 3600);
      JITTER_PCT = num(cuerpo?.jitterPct, JITTER_PCT, 0, 50);
      MAX_POR_DIA = num(cuerpo?.maxPorDia, MAX_POR_DIA, 1, 200);
      evento('config', null, null, `Ritmo desde la UI: cada ${INTERVALO_SEG}s ±${JITTER_PCT}%, tope ${MAX_POR_DIA}/día.`);
      void bombear();
      json(res, 200, { ok: true, intervaloSeg: INTERVALO_SEG, jitterPct: JITTER_PCT, maxPorDia: MAX_POR_DIA });
    } catch (e) {
      json(res, 400, { error: e instanceof Error ? e.message : 'Petición inválida.' });
    }
    return;
  }
  if (req.method === 'POST' && url.pathname === '/api/mejora') {
    try {
      const cuerpo = JSON.parse(await leerCuerpo(req));
      const { fotoId, original, prompt } = cuerpo || {};
      if (typeof fotoId !== 'string' || !fotoId || typeof original !== 'string' || !original.startsWith('data:image/')) {
        json(res, 400, { error: 'fotoId y original (dataURL de imagen) son obligatorios.' });
        return;
      }
      /* F21: una foto = un trabajo vivo. Si ya hay uno encolado, en proceso
       * o listo para esta foto, se devuelve ese en vez de duplicar la cola
       * (Reintentar + automático + reenvíos creaban N trabajos por foto). */
      const vivo = [...trabajos.values()].find(
        (t) => t.fotoId === fotoId && (t.estado === 'encolado' || t.estado === 'procesando' || t.estado === 'lista'),
      );
      if (vivo) {
        evento('encolado', vivo.jobId, fotoId, 'Duplicado ignorado: la foto ya tiene un trabajo vivo.');
        json(res, 200, { jobId: vivo.jobId, duplicado: true });
        return;
      }
      const jobId = randomUUID();
      trabajos.set(jobId, {
        jobId,
        fotoId,
        original,
        prompt: typeof prompt === 'string' && prompt.trim() ? prompt.trim().slice(0, 2000) : 'Mejora esta foto inmobiliaria.',
        estado: 'encolado',
        imagen: null,
        error: null,
        motivo: null,
        intentos: 0,
        proximoReintento: null,
        encoladoEn: Date.now(),
        finEn: null,
      });
      cola.push(jobId);
      evento('encolado', jobId, fotoId, null);
      void bombear();
      json(res, 200, { jobId });
    } catch (e) {
      json(res, 400, { error: e instanceof Error ? e.message : 'Petición inválida.' });
    }
    return;
  }
  const m = /^\/api\/mejora\/([A-Za-z0-9-]+)$/.exec(url.pathname);
  if (req.method === 'GET' && m) {
    const t = trabajos.get(m[1]);
    if (!t) { json(res, 404, { error: 'Trabajo no encontrado.' }); return; }
    json(res, 200, {
      estado: t.estado,
      ...(t.imagen ? { imagen: t.imagen } : {}),
      ...(t.error ? { error: t.error } : {}),
      ...(t.motivo ? { motivo: t.motivo } : {}),
      fotoId: t.fotoId,
      intentos: t.intentos,
      proximoReintentoEnSeg: t.proximoReintento ? Math.max(0, Math.round((t.proximoReintento - Date.now()) / 1000)) : null,
    });
    return;
  }
  /* F19+MCP: reintentar un error final con el original retenido (sin re-subir MBs). */
  const r = /^\/api\/mejora\/([A-Za-z0-9-]+)\/reintentar$/.exec(url.pathname);
  if (req.method === 'POST' && r) {
    const t = trabajos.get(r[1]);
    if (!t) { json(res, 404, { error: 'Trabajo no encontrado.' }); return; }
    if (t.estado !== 'error') { json(res, 409, { error: 'Solo se reintentan errores finales.' }); return; }
    t.estado = 'encolado';
    t.error = null;
    t.motivo = null;
    t.intentos = 0;
    t.proximoReintento = null;
    t.encoladoEn = Date.now();
    cola.push(r[1]);
    evento('reintento-manual', r[1], t.fotoId, 'Reintento manual: intentos a cero.');
    void bombear();
    json(res, 200, { jobId: r[1] });
    return;
  }
  /* F21: cancelar un trabajo en cola o en proceso. En proceso se descarta
   * el worker (su resultado se ignora al volver) y la bomba sigue con el
   * siguiente; nada queda a medias. */
  const c = /^\/api\/mejora\/([A-Za-z0-9-]+)\/cancelar$/.exec(url.pathname);
  if (req.method === 'POST' && c) {
    const t = trabajos.get(c[1]);
    if (!t) { json(res, 404, { error: 'Trabajo no encontrado.' }); return; }
    if (t.estado !== 'encolado' && t.estado !== 'procesando') {
      json(res, 409, { error: 'Solo se cancelan trabajos en cola o en proceso.' });
      return;
    }
    const eraActivo = c[1] === jobEnVuelo;
    const idx = cola.indexOf(c[1]);
    if (idx !== -1) cola.splice(idx, 1);
    t.estado = 'cancelado';
    t.motivo = null;
    t.proximoReintento = null;
    t.finEn = Date.now();
    evento('cancelado', c[1], t.fotoId, eraActivo ? 'Cancelado en proceso: su resultado se descarta.' : 'Sacado de la cola por el usuario.');
    if (eraActivo) matarWorker();
    void bombear();
    json(res, 200, { cancelado: true });
    return;
  }
  /* F21: reiniciar la cola sin borrar lo bien. Los trabajos encolados, en
   * proceso (salvo el que el worker genera ahora mismo) y en error final
   * vuelven a encolado con intentos a cero; los duplicados por foto se
   * descartan (el primero gana); lo `lista` y lo `cancelado` no se toca. */
  if (req.method === 'POST' && url.pathname === '/api/cola/reiniciar') {
    let reencolados = 0;
    let descartados = 0;
    const vistos = new Set();
    const nuevaCola = [];
    for (const [id, t] of trabajos) {
      if (id === jobEnVuelo) continue; // Termina su curso sin tocarlo.
      if (t.estado !== 'encolado' && t.estado !== 'procesando' && t.estado !== 'error') continue;
      if (vistos.has(t.fotoId)) {
        t.estado = 'cancelado';
        t.motivo = null;
        t.proximoReintento = null;
        t.finEn = Date.now();
        descartados += 1;
        continue;
      }
      vistos.add(t.fotoId);
      t.estado = 'encolado';
      t.error = null;
      t.motivo = null;
      t.intentos = 0;
      t.proximoReintento = null;
      t.encoladoEn = Date.now();
      t.finEn = null;
      nuevaCola.push(id);
      reencolados += 1;
    }
    cola.length = 0;
    cola.push(...nuevaCola);
    evento('reinicio-cola', null, null, `${reencolados} reencolados, ${descartados} duplicados descartados. Lo lista no se tocó.`);
    void bombear();
    json(res, 200, { reencolados, descartados });
    return;
  }
  /* F21: sonda real de credenciales sin gastar una foto. Login fresco +
   * mensaje de texto temporal: si las cookies no autentican, falla aquí en
   * segundos con el motivo (Permission denied / no autenticado).
   * F26: el texto no pasa por el gate de imágenes, así que la sonda exige
   * además estado AVAILABLE; si no, informa degradada en vez de un falso OK. */
  if (req.method === 'POST' && url.pathname === '/api/probar') {
    if (!listo()) {
      json(res, 200, { ok: false, error: 'Falta GEMINI_PSID en .env.local del backend (renueva con npm run renovar-cookies).' });
      return;
    }
    try {
      await asegurarWorker();
      const hijo = hijoWorker;
      if (!hijo || !hijo.stdin || hijo.stdin.destroyed) throw new Error('El worker no acepta peticiones.');
      const id = `probar-${randomUUID()}`;
      const estado = await new Promise((resolve, reject) => {
        const temporizador = setTimeout(() => {
          workerPendientes.delete(id);
          matarWorker();
          reject(new Error('Sin respuesta del worker en 90 segundos.'));
        }, 90 * 1000);
        workerPendientes.set(id, { resolve, reject, temporizador });
        hijo.stdin.write(`${JSON.stringify({ id, probar: true })}\n`, (e) => {
          if (e) {
            workerPendientes.delete(id);
            clearTimeout(temporizador);
            matarWorker();
            reject(e);
          }
        });
      });
      if (typeof estado === 'string' && estado !== 'AVAILABLE') {
        evento('prueba-conexion', null, null, `DEGRADADA (${estado}): el texto pasa pero las imágenes se rechazan. Renueva con npm run renovar-cookies.`);
        json(res, 200, {
          ok: false,
          error: `Sesión degradada (${estado}): Gemini responde texto pero rechaza imágenes. Renueva con npm run renovar-cookies.`,
        });
        return;
      }
      evento('prueba-conexion', null, null, 'OK: las cookies autentican (texto e imágenes).');
      json(res, 200, { ok: true });
    } catch (e) {
      const msg = (e instanceof Error ? e.message : 'Error desconocido.').slice(0, 300);
      evento('prueba-conexion', null, null, `FALLO: ${msg}`);
      json(res, 200, { ok: false, error: msg });
    }
    return;
  }
  json(res, 404, { error: 'Ruta no encontrada.' });
});

await cargarEnvLocal();
vigilarEnvLocal(); // F22: renovar-cookies reescribe .env.local y se recarga solo.
await fs.mkdir(TMP, { recursive: true }).catch(() => {});
await fs.mkdir(LOG_DIR, { recursive: true }).catch(() => {});
evento('arranque', null, null, `Puerto ${PUERTO}, intervalo ${INTERVALO_SEG}s, jitter ${JITTER_PCT}%, maxDia ${MAX_POR_DIA}, maxIntentos ${MAX_INTENTOS}.`);
servidor.listen(PUERTO, '127.0.0.1', () => {
  console.log(`[mejora] http://127.0.0.1:${PUERTO} listo=${listo()} intervalo=${INTERVALO_SEG}s jitter=${JITTER_PCT}% maxDia=${MAX_POR_DIA} maxIntentos=${MAX_INTENTOS}`);
});
/* Latido cada 30s: despierta reintentos vencidos, libera el tope tras la
 * medianoche y reanuda la cola aunque el último bombear no dejara cadena. */
setInterval(() => void bombear(), 30 * 1000).unref();
