import { useCallback, useState } from 'react';
import type { RecetaPublicidad } from '@/domain/plantilla-publicidad';
import type { Inmueble } from '@/domain/inmueble';
import { actualizarRemoto } from '@/data/inmuebles/api';

// Recetas guardadas por inmueble (qué 3 fotos por ÍNDICE + formato usa cada
// imagen publicitaria). Fuente real: el servidor (`inmuebles.receta`, vía
// PUT); localStorage queda como reserva y se escribe primero para no perder
// nada si la red falla. Las recetas viejas con URLs se migran al leer (ver
// `recetaVigenteDe`); la v1 se ignora por forma.

const CLAVE = 'publicidad:recetas:v2';
const CLAVE_VIEJA = 'publicidad:composiciones:v1';

type Mapa = Record<string, unknown>;

function leerMapa(clave: string): Mapa {
  try {
    const crudo = localStorage.getItem(clave);
    if (!crudo) return {};
    const datos = JSON.parse(crudo) as unknown;
    if (typeof datos !== 'object' || datos === null) return {};
    return datos as Mapa;
  } catch {
    return {};
  }
}

/* Lee la v2 y, si está vacía, rescata la v1 (recetas con URLs): la
 * migración a índices ocurre al resolver con el inmueble (`recetaVigenteDe`)
 * y se persiste al próximo guardar. */
function leer(): Mapa {
  const actual = leerMapa(CLAVE);
  if (Object.keys(actual).length > 0) return actual;
  return leerMapa(CLAVE_VIEJA);
}

export function usePublicidades() {
  const [mapa, setMapa] = useState<Mapa>(leer);

  /* Guarda la receta en local (reserva) y en el servidor (fuente del
   * frente público). Devuelve el inmueble ya persistido. Si el PUT falla,
   * lanza: la copia local queda, pero el llamador debe avisarlo. */
  const guardar = useCallback(async (inmueble: Inmueble, receta: RecetaPublicidad): Promise<Inmueble> => {
    setMapa((previo) => {
      const siguiente = { ...previo, [inmueble.id]: receta };
      try {
        localStorage.setItem(CLAVE, JSON.stringify(siguiente));
      } catch {
        // Sin almacenamiento la receta solo vive en memoria; no bloquea.
      }
      return siguiente;
    });
    return actualizarRemoto({ ...inmueble, receta });
  }, []);

  const recetaDe = useCallback(
    (inmuebleId: string): unknown => mapa[inmuebleId] ?? null,
    [mapa],
  );

  return { recetaDe, guardar };
}
