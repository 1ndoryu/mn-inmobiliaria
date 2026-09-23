import { useEffect, useState } from 'react';
import { listarPublicos } from '../../data/inmuebles/publica';
import type { InmueblePublico } from '../../domain/inmueble';

/* Catálogo público: lista de publicados, sin sesión. */
export function usePublica() {
  const [inmuebles, setInmuebles] = useState<InmueblePublico[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let vivo = true;
    listarPublicos()
      .then((lista) => {
        if (vivo) setInmuebles(lista);
      })
      .catch((e: unknown) => {
        if (vivo) setError(e instanceof Error ? e.message : 'No se pudo cargar el catálogo.');
      })
      .finally(() => {
        if (vivo) setCargando(false);
      });
    return () => {
      vivo = false;
    };
  }, []);

  return { inmuebles, cargando, error };
}
