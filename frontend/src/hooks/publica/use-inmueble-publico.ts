import { useEffect, useState } from 'react';
import { ErrorApi } from '../../data/inmuebles/api';
import { detallePublico } from '../../data/inmuebles/publica';
import type { InmueblePublico } from '../../domain/inmueble';

/* Detalle público por slug. `noEncontrado` = la API responde 404 (no existe
 * o no está publicado): la página lo muestra en vez de un error genérico. */
export function useInmueblePublico(slug: string) {
  const [inmueble, setInmueble] = useState<InmueblePublico | null>(null);
  const [cargando, setCargando] = useState(true);
  const [noEncontrado, setNoEncontrado] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let vivo = true;
    detallePublico(slug)
      .then((i) => {
        if (vivo) setInmueble(i);
      })
      .catch((e: unknown) => {
        if (!vivo) return;
        if (e instanceof ErrorApi && e.estado === 404) setNoEncontrado(true);
        else setError(e instanceof Error ? e.message : 'No se pudo cargar el inmueble.');
      })
      .finally(() => {
        if (vivo) setCargando(false);
      });
    return () => {
      vivo = false;
    };
  }, [slug]);

  return { inmueble, cargando, noEncontrado, error };
}
