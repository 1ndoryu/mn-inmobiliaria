// [10AA-24] Escucha los cambios del panel de chats por long-poll y avisa para
// refrescar. Pausa con la pestaña oculta (al volver, la versión guardada ya
// quedó vieja y el backend responde al instante) y espacia los reintentos si la
// API falla, para no martillearla.

import { useEffect, useRef } from 'react';
import { esperarCambiosChats } from '../../../data/chat/marketplace-chats';
import { ErrorApi } from '../../../data/inmuebles/api';

/* Ráfagas (releer + guardar caché de un mismo mensaje) se funden en un refresco. */
const AGRUPAR_MS = 400;
const REINTENTO_BASE_MS = 2_000;
const REINTENTO_MAX_MS = 30_000;
/* Respuesta sin cambio antes de este plazo = algo intermedio la cortó: no se
 * vuelve a preguntar en bucle cerrado. */
const RESPUESTA_RAPIDA_MS = 1_000;

function pausa(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolver) => {
    const t = setTimeout(resolver, ms);
    signal.addEventListener('abort', () => {
      clearTimeout(t);
      resolver();
    }, { once: true });
  });
}

function hastaVisible(signal: AbortSignal): Promise<void> {
  return new Promise((resolver) => {
    const listo = () => {
      document.removeEventListener('visibilitychange', alVer);
      resolver();
    };
    const alVer = () => {
      if (!document.hidden) listo();
    };
    document.addEventListener('visibilitychange', alVer);
    signal.addEventListener('abort', listo, { once: true });
  });
}

export function useCambiosChats(alCambiar: () => void): void {
  /* La función cambia en cada render del consumidor: el bucle lee la última. */
  const ultima = useRef(alCambiar);
  useEffect(() => {
    ultima.current = alCambiar;
  }, [alCambiar]);

  useEffect(() => {
    const ctl = new AbortController();
    let temporizador: ReturnType<typeof setTimeout> | undefined;
    const avisar = () => {
      clearTimeout(temporizador);
      temporizador = setTimeout(() => ultima.current(), AGRUPAR_MS);
    };

    void (async () => {
      let version: number | undefined;
      let fallos = 0;
      while (!ctl.signal.aborted) {
        if (document.hidden) {
          await hastaVisible(ctl.signal);
          continue;
        }
        const inicio = Date.now();
        try {
          const r = await esperarCambiosChats(version, ctl.signal);
          fallos = 0;
          const cambio = version !== undefined && r.version !== version;
          version = r.version;
          if (cambio) avisar();
          else if (Date.now() - inicio < RESPUESTA_RAPIDA_MS) await pausa(RESPUESTA_RAPIDA_MS, ctl.signal);
        } catch (e: unknown) {
          if (ctl.signal.aborted) return;
          /* Sin sesión no hay nada que escuchar: el login vuelve a montar el panel. */
          if (e instanceof ErrorApi && e.estado === 401) return;
          fallos += 1;
          await pausa(Math.min(REINTENTO_MAX_MS, REINTENTO_BASE_MS * 2 ** (fallos - 1)), ctl.signal);
        }
      }
    })();

    return () => {
      ctl.abort();
      clearTimeout(temporizador);
    };
  }, []);
}
