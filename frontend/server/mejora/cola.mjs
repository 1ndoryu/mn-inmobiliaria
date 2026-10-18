// Bomba secuencial de la cola de mejora (concurrencia 1) con reintentos por trabajo:
// - Solo coge trabajos `encolado` con el reintento ya vencido.
// - Fallo reintentable -> intentos+1, vuelve a `encolado` con backoff y al final de
//   la cola (los demás pasan primero). Nada queda en error por un fallo temporal.
// - Fallo final (causa no recuperable o intentos > MAX_INTENTOS) -> `error`.
// - Sin credenciales o con tope diario -> la cola espera, no falla trabajos.
// - Tras la espera del throttle se re-evalúa todo (pudo llegar medianoche).
import { MAX_INTENTOS, ritmo, cola, hoy, sesion, trabajos } from './estado.mjs';
import { evento } from './registro.mjs';
import { ejecutarWorker, matarWorker } from './worker.mjs';

export function listo() {
  return Boolean(process.env.GEMINI_PSID);
}

/* Intervalo base + jitter + backoff exponencial (2m,4m,8m... techo 60m). */
export function esperaMs() {
  const base = ritmo.intervaloSeg * 1000;
  const jitter = base * (ritmo.jitterPct / 100) * Math.random();
  const backoff = sesion.fallosSeguidos <= 0 ? 0 : Math.min(60 * 60 * 1000, 2 * 60 * 1000 * 2 ** (sesion.fallosSeguidos - 1));
  return Math.floor(base + jitter + backoff);
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

/* Avisos con throttle para no llenar el log de eventos. */
function avisarSinCreds() {
  if (Date.now() - sesion.ultimoAvisoCreds < 3600 * 1000) return;
  sesion.ultimoAvisoCreds = Date.now();
  evento('sin-credenciales', null, null, 'Falta GEMINI_PSID en .env.local: la cola espera sin fallar trabajos. Renueva con npm run renovar-cookies.');
}

function avisarTope() {
  if (sesion.avisadoTope) return;
  sesion.avisadoTope = true;
  evento('tope-diario', null, null, `Tope ${sesion.procesadosHoy}/${ritmo.maxPorDia}: la cola sigue y continúa tras medianoche.`);
}

/* El mapa de trabajos es memoria viva: se conservan los 200 más recientes. */
export function podarTrabajos() {
  if (trabajos.size <= 200) return;
  for (const [id, t] of trabajos) {
    if (trabajos.size <= 200) break;
    if (t.estado === 'lista' || t.estado === 'error') trabajos.delete(id);
  }
}

function trabajoVencido(id, ahora) {
  const t = trabajos.get(id);
  return Boolean(t && t.estado === 'encolado' && (t.proximoReintento ?? 0) <= ahora);
}

/* Procesa un trabajo de la cola. Se llama en bucle por el latido y al terminar cada vuelo. */
export async function bombear() {
  // F22: cookies renovadas en pleno vuelo: recicla la sesión obsoleta en
  // cuanto queda libre para que el siguiente login use las nuevas.
  if (sesion.recargaPendiente && !sesion.jobEnVuelo) {
    sesion.recargaPendiente = false;
    matarWorker();
  }
  if (sesion.procesando) return;
  if (hoy() !== sesion.dia) {
    sesion.dia = hoy();
    sesion.procesadosHoy = 0;
    sesion.avisadoTope = false;
    evento('nuevo-dia', null, null, 'Contador diario a cero: la cola continúa.');
  }
  if (!cola.some((id) => trabajoVencido(id, Date.now()))) return;
  if (!listo()) {
    avisarSinCreds();
    return;
  }
  if (sesion.procesadosHoy >= ritmo.maxPorDia) {
    avisarTope();
    return;
  }
  sesion.procesando = true;
  try {
    const espera = sesion.ultimoFin === 0 ? 0 : Math.max(0, sesion.ultimoFin + esperaMs() - Date.now());
    if (espera > 0) await new Promise((r) => setTimeout(r, espera));
    const ahora = Date.now();
    const idx = cola.findIndex((id) => trabajoVencido(id, ahora));
    if (idx === -1) return;
    if (!listo()) {
      avisarSinCreds();
      return;
    }
    if (sesion.procesadosHoy >= ritmo.maxPorDia) {
      avisarTope();
      return;
    }
    const jobId = cola.splice(idx, 1)[0];
    const tarea = trabajos.get(jobId);
    if (!tarea) return;
    tarea.estado = 'procesando';
    tarea.motivo = null;
    evento('inicio', jobId, tarea.fotoId, `Intento ${tarea.intentos + 1}/${MAX_INTENTOS}.`);
    sesion.jobEnVuelo = jobId;
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
        sesion.procesadosHoy += 1;
        sesion.fallosSeguidos = 0;
        evento('lista', jobId, tarea.fotoId, null);
        podarTrabajos();
      }
    } catch (e) {
      tratarFallo(e, jobId, tarea);
    } finally {
      sesion.ultimoFin = Date.now();
      if (sesion.jobEnVuelo === jobId) sesion.jobEnVuelo = null;
    }
  } finally {
    sesion.procesando = false;
    void bombear();
  }
}

/* Fallo de un vuelo: cancelado (no quema intento), final (error) o reintentable. */
function tratarFallo(e, jobId, tarea) {
  if (tarea.estado === 'cancelado') {
    // Cancelado en proceso (worker descartado): no quema intento ni
    // ensucia el backoff global.
    evento('cancelado', jobId, tarea.fotoId, 'Cancelado en proceso.');
    podarTrabajos();
    return;
  }
  sesion.fallosSeguidos += 1;
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
    return;
  }
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
