/* [30AA-9] Estado del modal de vínculos que abre el badge de la columna
 * Vínculo. Vive en un hook (y no en `TablaInmuebles`) por el límite de
 * `useState` del gate: allí ya hay otros tres estados y un cuarto lo supera.
 * La tabla solo pinta `ModalVinculos` con lo que devuelve esto. */
import { useCallback, useState } from 'react';
import type { Inmueble } from '@/domain/inmueble';

export function useModalVinculos(alReponer: (inmueble: Inmueble) => void) {
  const [inmueble, setInmueble] = useState<Inmueble | null>(null);

  const abrir = useCallback((i: Inmueble) => setInmueble(i), []);
  const cerrar = useCallback(() => setInmueble(null), []);

  /* La ficha que devuelve la relectura del servidor repone la fila y, si el
   * modal sigue abierto sobre esa misma ficha, la actualiza sin cerrarlo. */
  const alActualizar = useCallback(
    (actualizado: Inmueble) => {
      alReponer(actualizado);
      setInmueble((actual) => (actual && actual.id === actualizado.id ? actualizado : actual));
    },
    [alReponer],
  );

  return { inmueble, abrir, cerrar, alActualizar };
}
