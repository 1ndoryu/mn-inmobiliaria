// Rutas de trabajos de mejora: encolar una foto, consultar su estado, reintentar
// un error final, cancelar y reiniciar la cola. Devuelven false si la petición no es suya.
import { randomUUID } from 'node:crypto';
import { cola, sesion, trabajos } from './estado.mjs';
import { evento } from './registro.mjs';
import { json, leerCuerpo } from './http.mjs';
import { bombear } from './cola.mjs';
import { matarWorker } from './worker.mjs';

const NO_ENCONTRADO = { error: 'Trabajo no encontrado.' };

export async function atenderTrabajos(req, res, url) {
  if (req.method === 'POST' && url.pathname === '/api/mejora') {
    await encolar(req, res);
    return true;
  }
  const m = /^\/api\/mejora\/([A-Za-z0-9-]+)$/.exec(url.pathname);
  if (req.method === 'GET' && m) {
    consultar(res, m[1]);
    return true;
  }
  const r = /^\/api\/mejora\/([A-Za-z0-9-]+)\/reintentar$/.exec(url.pathname);
  if (req.method === 'POST' && r) {
    reintentar(res, r[1]);
    return true;
  }
  const c = /^\/api\/mejora\/([A-Za-z0-9-]+)\/cancelar$/.exec(url.pathname);
  if (req.method === 'POST' && c) {
    cancelar(res, c[1]);
    return true;
  }
  if (req.method === 'POST' && url.pathname === '/api/cola/reiniciar') {
    reiniciarCola(res);
    return true;
  }
  return false;
}

/* Trabajo de la foto que aún está vivo (encolado, en proceso o listo), si lo hay. */
function vivoDe(fotoId) {
  return [...trabajos.values()].find(
    (t) => t.fotoId === fotoId && (t.estado === 'encolado' || t.estado === 'procesando' || t.estado === 'lista'),
  );
}

/* F21: una foto = un trabajo vivo. Si ya hay uno, se devuelve ese en vez de duplicar la cola
 * (Reintentar + automático + reenvíos creaban N trabajos por foto). */
async function encolar(req, res) {
  try {
    const cuerpo = JSON.parse(await leerCuerpo(req));
    const { fotoId, original, prompt } = cuerpo || {};
    if (typeof fotoId !== 'string' || !fotoId || typeof original !== 'string' || !original.startsWith('data:image/')) {
      json(res, 400, { error: 'fotoId y original (dataURL de imagen) son obligatorios.' });
      return;
    }
    const vivo = vivoDe(fotoId);
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
}

function consultar(res, id) {
  const t = trabajos.get(id);
  if (!t) {
    json(res, 404, NO_ENCONTRADO);
    return;
  }
  json(res, 200, {
    estado: t.estado,
    ...(t.imagen ? { imagen: t.imagen } : {}),
    ...(t.error ? { error: t.error } : {}),
    ...(t.motivo ? { motivo: t.motivo } : {}),
    fotoId: t.fotoId,
    intentos: t.intentos,
    proximoReintentoEnSeg: t.proximoReintento ? Math.max(0, Math.round((t.proximoReintento - Date.now()) / 1000)) : null,
  });
}

/* F19+MCP: reintentar un error final con el original retenido (sin re-subir MBs). */
function reintentar(res, id) {
  const t = trabajos.get(id);
  if (!t) {
    json(res, 404, NO_ENCONTRADO);
    return;
  }
  if (t.estado !== 'error') {
    json(res, 409, { error: 'Solo se reintentan errores finales.' });
    return;
  }
  t.estado = 'encolado';
  t.error = null;
  t.motivo = null;
  t.intentos = 0;
  t.proximoReintento = null;
  t.encoladoEn = Date.now();
  cola.push(id);
  evento('reintento-manual', id, t.fotoId, 'Reintento manual: intentos a cero.');
  void bombear();
  json(res, 200, { jobId: id });
}

/* F21: cancelar un trabajo en cola o en proceso. En proceso se descarta el worker
 * (su resultado se ignora al volver) y la bomba sigue con el siguiente. */
function cancelar(res, id) {
  const t = trabajos.get(id);
  if (!t) {
    json(res, 404, NO_ENCONTRADO);
    return;
  }
  if (t.estado !== 'encolado' && t.estado !== 'procesando') {
    json(res, 409, { error: 'Solo se cancelan trabajos en cola o en proceso.' });
    return;
  }
  const eraActivo = id === sesion.jobEnVuelo;
  const idx = cola.indexOf(id);
  if (idx !== -1) cola.splice(idx, 1);
  t.estado = 'cancelado';
  t.motivo = null;
  t.proximoReintento = null;
  t.finEn = Date.now();
  evento('cancelado', id, t.fotoId, eraActivo ? 'Cancelado en proceso: su resultado se descarta.' : 'Sacado de la cola por el usuario.');
  if (eraActivo) matarWorker();
  void bombear();
  json(res, 200, { cancelado: true });
}

/* F21: reiniciar la cola sin borrar lo bien. Encolados, en proceso (salvo el que el worker
 * genera ahora mismo) y en error final vuelven a encolado con intentos a cero; los duplicados
 * por foto se descartan (el primero gana); lo `lista` y lo `cancelado` no se toca. */
function reiniciarCola(res) {
  let reencolados = 0;
  let descartados = 0;
  const vistos = new Set();
  const nuevaCola = [];
  for (const [id, t] of trabajos) {
    if (id === sesion.jobEnVuelo) continue; // Termina su curso sin tocarlo.
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
}
