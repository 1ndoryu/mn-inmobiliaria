// Red hacia el mini-backend local de mejora (127.0.0.1:3122 vía proxy /api).
// Puro: sin React ni DOM. El backend custodia las cookies de Gemini.

export interface SaludMejora {
  ok: boolean;
  listo: boolean;
  cola: number;
  procesadosHoy: number;
  /** Ritmo efectivo que aplica el backend (lo fija la UI vía /api/config). */
  intervaloSeg?: number;
  jitterPct?: number;
  maxPorDia?: number;
  detalle?: string;
}

export interface ResultadoEnvio {
  ok: boolean;
  jobId?: string;
  motivo?: string;
}

export interface EstadoTrabajo {
  estado: 'encolado' | 'procesando' | 'lista' | 'error' | 'cancelado' | 'desconocido';
  imagen?: string;
  error?: string;
  /** Último fallo aunque el trabajo siga vivo (reintento programado). */
  motivo?: string;
  fotoId?: string;
  intentos?: number;
  /** Segundos hasta el próximo reintento (null = sin espera programada). */
  proximoReintentoEnSeg?: number | null;
}

/* F18: snapshot de cola para diagnóstico. */
export interface TrabajoCola {
  jobId: string;
  fotoId: string;
  estado: 'encolado' | 'procesando';
  encoladoHaceSeg: number;
  intentos: number;
  proximoReintentoEnSeg: number | null;
  motivo: string | null;
}

export interface EstadoCola {
  ok: boolean;
  listo: boolean;
  detalle: string | null;
  cola: TrabajoCola[];
  activo: TrabajoCola | null;
  worker: { vivo: boolean; trabajos: number; pid: number | null };
  procesadosHoy: number;
  maxPorDia: number;
  maxIntentos: number;
  intervaloSeg?: number;
  jitterPct?: number;
  fallosSeguidos: number;
  proximoArranqueEnSeg: number | null;
  reintentosPendientes?: number;
}

export interface EventoCola {
  t: string;
  tipo: string;
  jobId: string | null;
  fotoId: string | null;
  detalle: string | null;
}

import { urlADataUrl } from '../../platform/red';

const BASE = '/api';

export async function leerSalud(signal?: AbortSignal): Promise<SaludMejora> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/salud`, { signal });
  } catch {
    return { ok: false, listo: false, cola: 0, procesadosHoy: 0, detalle: 'Backend no disponible. Arranca `npm run server:mejora`.' };
  }
  if (!respuesta.ok) return { ok: false, listo: false, cola: 0, procesadosHoy: 0, detalle: `Backend devolvió ${respuesta.status}.` };
  try {
    const cuerpo = (await respuesta.json()) as Partial<SaludMejora>;
    return {
      ok: true,
      listo: cuerpo.listo === true,
      cola: typeof cuerpo.cola === 'number' ? cuerpo.cola : 0,
      procesadosHoy: typeof cuerpo.procesadosHoy === 'number' ? cuerpo.procesadosHoy : 0,
      intervaloSeg: typeof cuerpo.intervaloSeg === 'number' ? cuerpo.intervaloSeg : undefined,
      jitterPct: typeof cuerpo.jitterPct === 'number' ? cuerpo.jitterPct : undefined,
      maxPorDia: typeof cuerpo.maxPorDia === 'number' ? cuerpo.maxPorDia : undefined,
    };
  } catch {
    return { ok: false, listo: false, cola: 0, procesadosHoy: 0, detalle: 'Respuesta no JSON del backend.' };
  }
}

export async function enviarFoto(fotoId: string, original: string, prompt: string, signal?: AbortSignal): Promise<ResultadoEnvio> {
  /* El backend exige dataURL: las fotos del servidor viajan como URL y se
   * convierten aquí, bajo demanda (sin descargar MBs al importar). */
  let cuerpo: string = original;
  if (original.startsWith('http')) {
    try {
      cuerpo = await urlADataUrl(original);
    } catch {
      return { ok: false, motivo: 'No se pudo descargar la foto del servidor para mejorarla.' };
    }
  }
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/mejora`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ fotoId, original: cuerpo, prompt }),
      signal,
    });
  } catch {
    return { ok: false, motivo: 'No se pudo contactar con el backend local.' };
  }
  if (!respuesta.ok) {
    const detalle = (await respuesta.text()).slice(0, 200);
    return { ok: false, motivo: `Backend devolvió ${respuesta.status}. ${detalle}` };
  }
  try {
    const cuerpo = (await respuesta.json()) as { jobId?: unknown };
    if (typeof cuerpo.jobId !== 'string' || !cuerpo.jobId) return { ok: false, motivo: 'Backend sin jobId.' };
    return { ok: true, jobId: cuerpo.jobId };
  } catch {
    return { ok: false, motivo: 'Respuesta no JSON del backend.' };
  }
}

export async function leerTrabajo(jobId: string, signal?: AbortSignal): Promise<EstadoTrabajo | null> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/mejora/${encodeURIComponent(jobId)}`, { signal });
  } catch {
    return null;
  }
  /* 404 con "Trabajo no encontrado" no es fallo de red: el backend se
   * reinició y perdió la cola en memoria. Se distingue para re-enviar
   * solo en vez de sentenciar la foto (F19). */
  if (respuesta.status === 404) {
    try {
      const cuerpo = (await respuesta.json()) as { error?: unknown };
      if (cuerpo.error === 'Trabajo no encontrado.') return { estado: 'desconocido' };
    } catch {
      // Cuerpo no JSON: cae a null.
    }
    return null;
  }
  if (!respuesta.ok) return null;
  try {
    return (await respuesta.json()) as EstadoTrabajo;
  } catch {
    return null;
  }
}

/* Ritmo y topes al backend en caliente (lo que la UI guarda manda; sin
 * backend disponible no falla: devuelve ok:false y la UI avisa). */
export async function aplicarConfigRemota(config: {
  intervaloSeg: number;
  jitterPct: number;
  maxPorDia: number;
}): Promise<{ ok: boolean; intervaloSeg?: number; jitterPct?: number; maxPorDia?: number; detalle?: string }> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/config`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        intervaloSeg: config.intervaloSeg,
        jitterPct: config.jitterPct,
        maxPorDia: config.maxPorDia,
      }),
    });
  } catch {
    return { ok: false, detalle: 'Backend no disponible: el ritmo nuevo se aplicará al arrancar el servidor.' };
  }
  if (!respuesta.ok) return { ok: false, detalle: `Backend devolvió ${respuesta.status}.` };
  try {
    const cuerpo = (await respuesta.json()) as Partial<SaludMejora>;
    return {
      ok: true,
      intervaloSeg: typeof cuerpo.intervaloSeg === 'number' ? cuerpo.intervaloSeg : undefined,
      jitterPct: typeof cuerpo.jitterPct === 'number' ? cuerpo.jitterPct : undefined,
      maxPorDia: typeof cuerpo.maxPorDia === 'number' ? cuerpo.maxPorDia : undefined,
    };
  } catch {
    return { ok: false, detalle: 'Respuesta no JSON del backend.' };
  }
}

/* F21: reintentar un error final con el original retenido (sin re-subir
 * MBs). false = el trabajo ya no existe o no es error final (entonces toca
 * re-enviar la foto entera). */
export async function reintentarTrabajo(jobId: string): Promise<boolean> {
  try {
    const respuesta = await fetch(`${BASE}/mejora/${encodeURIComponent(jobId)}/reintentar`, { method: 'POST' });
    return respuesta.ok;
  } catch {
    return false;
  }
}

/* F21: sacar un trabajo de la cola o abortar el que está en proceso.
 * false = no se pudo contactar o ya terminó. */
export async function cancelarTrabajo(jobId: string): Promise<boolean> {
  try {
    const respuesta = await fetch(`${BASE}/mejora/${encodeURIComponent(jobId)}/cancelar`, { method: 'POST' });
    return respuesta.ok;
  } catch {
    return false;
  }
}

export interface ResultadoReinicio {
  ok: boolean;
  reencolados: number;
  descartados: number;
}

/* F21: empezar la cola de nuevo sin tocar lo lista. Los encolados, en
 * proceso (salvo el que se genera ahora) y en error final vuelven a
 * encolado con intentos a cero; los duplicados por foto se descartan. */
export async function reiniciarCola(): Promise<ResultadoReinicio> {
  try {
    const respuesta = await fetch(`${BASE}/cola/reiniciar`, { method: 'POST' });
    if (!respuesta.ok) return { ok: false, reencolados: 0, descartados: 0 };
    const cuerpo = (await respuesta.json()) as { reencolados?: unknown; descartados?: unknown };
    return {
      ok: true,
      reencolados: typeof cuerpo.reencolados === 'number' ? cuerpo.reencolados : 0,
      descartados: typeof cuerpo.descartados === 'number' ? cuerpo.descartados : 0,
    };
  } catch {
    return { ok: false, reencolados: 0, descartados: 0 };
  }
}

/* F21: sonda real de las cookies sin gastar una foto. null = backend caído. */
export async function probarConexion(): Promise<{ ok: boolean; error?: string } | null> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/probar`, { method: 'POST' });
  } catch {
    return null;
  }
  if (!respuesta.ok) return null;
  try {
    const cuerpo = (await respuesta.json()) as { ok?: unknown; error?: unknown };
    if (cuerpo.ok === true) return { ok: true };
    return { ok: false, error: typeof cuerpo.error === 'string' ? cuerpo.error : 'Falló la prueba.' };
  } catch {
    return null;
  }
}

/* F18: snapshot de cola. null = backend no disponible. */
export async function leerEstado(signal?: AbortSignal): Promise<EstadoCola | null> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/estado`, { signal });
  } catch {
    return null;
  }
  if (!respuesta.ok) return null;
  try {
    return (await respuesta.json()) as EstadoCola;
  } catch {
    return null;
  }
}

/* F18: últimos eventos del backend. Vacío si no disponible. */
export async function leerEventos(ultimos = 60, signal?: AbortSignal): Promise<EventoCola[]> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${BASE}/eventos?ultimos=${ultimos}`, { signal });
  } catch {
    return [];
  }
  if (!respuesta.ok) return [];
  try {
    const cuerpo = (await respuesta.json()) as { eventos?: unknown };
    return Array.isArray(cuerpo.eventos) ? (cuerpo.eventos as EventoCola[]) : [];
  } catch {
    return [];
  }
}
