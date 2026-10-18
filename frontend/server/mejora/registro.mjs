// Logging a fichero y buffer de eventos de la cola. Todo best-effort: si el
// disco falla, el servidor sigue funcionando.
import path from 'node:path';
import { createWriteStream } from 'node:fs';
import { LOG_DIR, hoy } from './estado.mjs';

/* F18: ficheros diarios en logs/: consola-AAAA-MM-DD.log (todo lo que sale
 * por consola, incluido [worker]) y eventos-AAAA-MM-DD.log (JSONL). */
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

export function aArchivo(nombre, linea) {
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

/* Salida informativa (arranque): mismo destino que el espejo de consola. */
export function informar(texto) {
  consolaOrig.log(texto);
  aArchivo('consola', texto);
}

/* Eventos de cola: buffer en memoria (últimos 500) + JSONL en disco. */
export const eventos = [];
export function evento(tipo, jobId, fotoId, detalle) {
  const e = { t: new Date().toISOString(), tipo, jobId: jobId ?? null, fotoId: fotoId ?? null, detalle: detalle ?? null };
  eventos.push(e);
  if (eventos.length > 500) eventos.splice(0, eventos.length - 500);
  aArchivo('eventos', JSON.stringify(e));
}
