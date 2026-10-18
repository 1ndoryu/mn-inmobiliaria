// Credenciales de Gemini (GEMINI_PSID/PSIDTS) desde .env.local, con recarga en
// caliente: `npm run renovar-cookies` reescribe el fichero y el watcher avisa.
import path from 'node:path';
import { promises as fs, watch } from 'node:fs';
import { RAIZ, sesion } from './estado.mjs';
import { evento } from './registro.mjs';
import { matarWorker } from './worker.mjs';
import { bombear } from './cola.mjs';

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

export async function cargarEnvLocal() {
  const mapa = await leerMapaEnv();
  for (const [k, v] of mapa) {
    if (!(k in process.env)) process.env[k] = v;
  }
}

/* F22: solo toca GEMINI_PSID/PSIDTS; si cambian, la sesión del worker queda
 * obsoleta: se descarta en cuanto no haya trabajo en vuelo para que el siguiente
 * login use las nuevas. Sin reiniciar nada. */
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
  sesion.fallosSeguidos = 0; // Credenciales nuevas: el backoff global no las arrastra.
  sesion.ultimoAvisoCreds = 0;
  evento('credenciales-recargadas', null, null, 'Cookies renovadas desde .env.local: la cola continúa sin reiniciar.');
  if (sesion.jobEnVuelo) {
    sesion.recargaPendiente = true; // El worker se recicla al terminar el vuelo.
  } else {
    matarWorker();
  }
  void bombear();
}

export function vigilarEnvLocal() {
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
