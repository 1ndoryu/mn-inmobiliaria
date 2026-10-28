import { useEffect, useState } from 'react';
import { listarPublicidadPublica } from '../../data/publicidad/publica';
import type { PublicidadPublicaItem } from '../../domain/redes/publicacion-redes';

/* [28AA-8] Piezas ya salidas para /publicidad: lista pública, sin sesión. */
export function usePublicidadPublica() {
  const [items, setItems] = useState<PublicidadPublicaItem[]>([]);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let vivo = true;
    listarPublicidadPublica()
      .then((lista) => {
        if (vivo) setItems(lista);
      })
      .catch((e: unknown) => {
        if (vivo) setError(e instanceof Error ? e.message : 'No se pudo cargar la publicidad.');
      })
      .finally(() => {
        if (vivo) setCargando(false);
      });
    return () => {
      vivo = false;
    };
  }, []);

  return { items, cargando, error };
}
