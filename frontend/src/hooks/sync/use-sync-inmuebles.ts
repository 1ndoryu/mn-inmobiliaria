/* [13AA-12] Estado de sincronización con el servidor en tiempo real: long-poll
 * sobre `revision` (responde al cambiar algo o a los ~25 s). Mismo patrón que
 * `use-cambios-chats`: pausa con la pestaña oculta, reintentos espaciados y
 * sin bucle cerrado si una respuesta vuelve al instante sin cambios. */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { comprobarSync, esperarEstadoSync, type EntradaSync, type EstadoSync } from '../../data/inmuebles/sync';
import { ErrorApi } from '../../data/inmuebles/api';

const REINTENTO_BASE_MS = 2_000;
const REINTENTO_MAX_MS = 30_000;
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

export interface SyncInmuebles {
  estado: EstadoSync | null;
  /* Comparación por slug (vacío sin servidor disponible). */
  entradas: Map<string, EntradaSync>;
  /* Fallo al hablar con la API local (no con el servidor: eso va en `estado.remoto`). */
  error: string | null;
  comprobarAhora: () => Promise<void>;
}

export function useSyncInmuebles(): SyncInmuebles {
  const [estado, setEstado] = useState<EstadoSync | null>(null);
  const [error, setError] = useState<string | null>(null);
  const entradas = useMemo(() => {
    const mapa = new Map<string, EntradaSync>();
    for (const e of estado?.comparacion?.inmuebles ?? []) mapa.set(e.slug, e);
    return mapa;
  }, [estado]);

  useEffect(() => {
    const ctl = new AbortController();

    void (async () => {
      let revision: number | undefined;
      let fallos = 0;
      while (!ctl.signal.aborted) {
        if (document.hidden) {
          await hastaVisible(ctl.signal);
          continue;
        }
        const inicio = Date.now();
        try {
          const e = await esperarEstadoSync(revision, ctl.signal);
          fallos = 0;
          setError(null);
          const cambio = e.revision !== revision;
          if (cambio) {
            revision = e.revision;
            setEstado(e);
          } else if (Date.now() - inicio < RESPUESTA_RAPIDA_MS) {
            await pausa(RESPUESTA_RAPIDA_MS, ctl.signal);
          }
        } catch (err: unknown) {
          if (ctl.signal.aborted) return;
          /* Sin sesión no hay nada que escuchar: el login vuelve a montar el panel. */
          if (err instanceof ErrorApi && err.estado === 401) return;
          fallos += 1;
          setError(err instanceof Error ? err.message : 'No se pudo leer el estado de sincronización.');
          await pausa(Math.min(REINTENTO_MAX_MS, REINTENTO_BASE_MS * 2 ** (fallos - 1)), ctl.signal);
        }
      }
    })();

    return () => ctl.abort();
  }, []);

  const comprobarAhora = useCallback(async () => {
    try {
      await comprobarSync();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'No se pudo pedir la comprobación.');
    }
  }, []);

  return { estado, entradas, error, comprobarAhora };
}
