import { useEffect, useState } from 'react';
import { leerEstadoLab } from '@/data/chat/lab-estado';
import type { ResumenLab } from '@/domain/lab';

/* Estado del lab para la cabecera [10AA-23]. El lab late cada 5 s; se lee
 * cada 10 s mientras la pestaña esté visible (oculta no gasta peticiones) y
 * al volver a verla. Un fallo de lectura es «sin-leer», nunca un verde falso. */
const REFRESCO_MS = 10_000;

const INICIAL: ResumenLab = { conexion: 'cargando', ultimoOkHaceS: null };

export function useEstadoLab(): ResumenLab {
  const [resumen, setResumen] = useState<ResumenLab>(INICIAL);

  useEffect(() => {
    const ctrl = new AbortController();
    const leer = () => {
      if (document.visibilityState !== 'visible') return;
      leerEstadoLab(ctrl.signal)
        .then(setResumen)
        .catch(() => {
          if (!ctrl.signal.aborted) setResumen({ conexion: 'sin-leer', ultimoOkHaceS: null });
        });
    };
    leer();
    const id = window.setInterval(leer, REFRESCO_MS);
    document.addEventListener('visibilitychange', leer);
    return () => {
      ctrl.abort();
      window.clearInterval(id);
      document.removeEventListener('visibilitychange', leer);
    };
  }, []);

  return resumen;
}
