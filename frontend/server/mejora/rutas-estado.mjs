// Rutas de diagnóstico y ajustes: salud, snapshot de la cola, eventos, ritmo
// desde la UI y sonda de credenciales. Devuelven false si la petición no es suya.
import { MAX_INTENTOS, acotar, ritmo, cola, hoy, sesion, trabajos } from './estado.mjs';
import { eventos, evento } from './registro.mjs';
import { json, leerCuerpo } from './http.mjs';
import { bombear, esperaMs, listo } from './cola.mjs';
import { sondearSesion, workerPid, workerTrabajos, workerVivo } from './worker.mjs';

const MSG_SIN_CREDENCIALES = 'Falta GEMINI_PSID en .env.local del backend (renueva con npm run renovar-cookies).';

export async function atenderEstado(req, res, url) {
  if (req.method === 'GET' && url.pathname === '/api/salud') {
    json(res, 200, {
      ok: true,
      listo: listo(),
      detalle: listo() ? null : MSG_SIN_CREDENCIALES,
      cola: cola.length + (sesion.procesando ? 1 : 0),
      procesadosHoy: sesion.procesadosHoy,
      intervaloSeg: ritmo.intervaloSeg,
      jitterPct: ritmo.jitterPct,
      maxPorDia: ritmo.maxPorDia,
      reintentosPendientes: [...trabajos.values()].filter((t) => t.estado === 'encolado' && t.motivo).length,
      fallosSeguidos: sesion.fallosSeguidos,
      worker: { vivo: workerVivo(), trabajos: workerTrabajos() },
    });
    return true;
  }
  /* F18: snapshot completo para el panel de diagnóstico y el MCP. */
  if (req.method === 'GET' && url.pathname === '/api/estado') {
    if (hoy() !== sesion.dia) {
      sesion.dia = hoy();
      sesion.procesadosHoy = 0;
      sesion.avisadoTope = false;
    }
    json(res, 200, estadoCompleto());
    return true;
  }
  /* F18: últimos eventos de la cola (también en eventos-AAAA-MM-DD.log). */
  if (req.method === 'GET' && url.pathname === '/api/eventos') {
    const n = Math.min(500, Math.max(1, Number(url.searchParams.get('ultimos')) || 200));
    json(res, 200, { eventos: eventos.slice(-n) });
    return true;
  }
  /* Ritmo y topes desde la UI (lo que el usuario pone en Configuración manda en
   * caliente). Rangos iguales a los del frontal: intervalo 30-3600 s, jitter 0-50 %,
   * tope 1-200/día. */
  if (req.method === 'POST' && url.pathname === '/api/config') {
    try {
      const cuerpo = JSON.parse(await leerCuerpo(req));
      ritmo.intervaloSeg = acotar(cuerpo?.intervaloSeg, ritmo.intervaloSeg, 30, 3600);
      ritmo.jitterPct = acotar(cuerpo?.jitterPct, ritmo.jitterPct, 0, 50);
      ritmo.maxPorDia = acotar(cuerpo?.maxPorDia, ritmo.maxPorDia, 1, 200);
      evento('config', null, null, `Ritmo desde la UI: cada ${ritmo.intervaloSeg}s ±${ritmo.jitterPct}%, tope ${ritmo.maxPorDia}/día.`);
      void bombear();
      json(res, 200, { ok: true, intervaloSeg: ritmo.intervaloSeg, jitterPct: ritmo.jitterPct, maxPorDia: ritmo.maxPorDia });
    } catch (e) {
      json(res, 400, { error: e instanceof Error ? e.message : 'Petición inválida.' });
    }
    return true;
  }
  if (req.method === 'POST' && url.pathname === '/api/probar') {
    await probar(res);
    return true;
  }
  return false;
}

/* Snapshot de /api/estado: cola, trabajo activo, worker y ritmo. */
function estadoCompleto() {
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
  return {
    ok: true,
    listo: listo(),
    detalle: !listo()
      ? MSG_SIN_CREDENCIALES
      : sesion.procesadosHoy >= ritmo.maxPorDia
        ? `Tope diario ${sesion.procesadosHoy}/${ritmo.maxPorDia}: continúa tras medianoche.`
        : null,
    cola: enCola,
    activo,
    worker: { vivo: workerVivo(), trabajos: workerTrabajos(), pid: workerPid() },
    procesadosHoy: sesion.procesadosHoy,
    maxPorDia: ritmo.maxPorDia,
    maxIntentos: MAX_INTENTOS,
    intervaloSeg: ritmo.intervaloSeg,
    jitterPct: ritmo.jitterPct,
    fallosSeguidos: sesion.fallosSeguidos,
    /* Ojo: esperaMs() consume un sorteo de jitter; es solo orientativo. */
    proximoArranqueEnSeg:
      sesion.procesando || enCola.length === 0 ? null : Math.max(0, Math.round((sesion.ultimoFin + esperaMs() - ahora) / 1000)),
  };
}

/* F21: sonda real de credenciales. Si las cookies no autentican, falla aquí en
 * segundos con el motivo. F26: además exige estado AVAILABLE (el texto no pasa
 * por el gate de imágenes) para no dar un falso OK. */
async function probar(res) {
  if (!listo()) {
    json(res, 200, { ok: false, error: MSG_SIN_CREDENCIALES });
    return;
  }
  try {
    const estado = await sondearSesion();
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
}
