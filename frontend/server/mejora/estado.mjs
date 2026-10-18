// Configuración y estado vivo del mini-backend de mejora. Una sola instancia
// compartida: la leen y escriben la cola, las credenciales y las rutas.
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// frontend/server (donde vive gemini_worker.py); RAIZ es frontend/.
export const AQUI = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const RAIZ = path.dirname(AQUI);
export const SCRIPT_WORKER = path.join(AQUI, 'gemini_worker.py');
// Temporales del worker (PNG, se borran solos): en C:\tmp.
export const TMP = path.join('C:\\tmp', 'inmobiliaria-mejora');
/* Logs en logs/ del proyecto (gitignored): en C:\tmp los borraría la purga
   horaria. */
export const LOG_DIR = path.join(RAIZ, 'logs');

export function entero(env, defecto, min, max) {
  const n = Number(process.env[env]);
  if (!Number.isFinite(n)) return defecto;
  return Math.min(max, Math.max(min, Math.floor(n)));
}

/* Valor recibido por la UI: si no es numérico se conserva el actual; si se sale
 * de rango se acota. */
export function acotar(valor, actual, min, max) {
  const n = Number(valor);
  if (!Number.isFinite(n)) return actual;
  return Math.min(max, Math.max(min, Math.floor(n)));
}

/* Se leen de process.env al cargar el módulo, igual que antes: .env.local aún
 * no está cargado en ese momento. */
export const PUERTO = entero('MEJORA_PORT', 3122, 1024, 65535);
export const PYTHON = process.env.MEJORA_PYTHON || 'python';
/* F19: intentos máximos por trabajo antes de darlo por perdido (1-20). */
export const MAX_INTENTOS = entero('MEJORA_MAX_INTENTOS', 5, 1, 20);

/* Ritmo y topes en caliente: el frontal los ajusta con POST /api/config al
 * guardar (lo que el usuario pone en la UI manda; los env son el defecto
 * inicial). Objeto mutable porque un `const` suelto impediría aplicarlos sin
 * reiniciar. */
export const ritmo = {
  intervaloSeg: entero('MEJORA_INTERVALO_SEG', 120, 30, 3600),
  jitterPct: entero('MEJORA_JITTER_PCT', 30, 0, 50),
  maxPorDia: entero('MEJORA_MAX_DIA', 40, 1, 200),
};

/* F18+F19: registro por trabajo. `estado` es encolado|procesando|lista|error|cancelado.
 * `motivo` guarda el último fallo aunque el trabajo siga vivo (reintento
 * programado); `error` solo se fija en error final. `original` se libera
 * (null) al terminar para no retener MBs en memoria. */
export const trabajos = new Map(); // jobId -> registro
export const cola = []; // jobIds en orden de llegada

/* Estado de la bomba. `jobEnVuelo` es el trabajo que el worker genera ahora
 * mismo: sirve para no tocarlo al reiniciar la cola y para descartar su
 * resultado si se cancela en pleno vuelo. */
export const sesion = {
  procesando: false,
  jobEnVuelo: null,
  ultimoFin: 0,
  fallosSeguidos: 0,
  dia: hoy(),
  procesadosHoy: 0,
  avisadoTope: false,
  ultimoAvisoCreds: 0,
  recargaPendiente: false, // F22: cookies nuevas con un trabajo en vuelo
};

export function hoy() {
  return new Date().toISOString().slice(0, 10);
}
