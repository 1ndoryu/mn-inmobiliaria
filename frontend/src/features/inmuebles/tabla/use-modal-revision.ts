/* [30AA-14] Estado del modal «Revisar versiones» que abre la fila. Vive en un
 * hook y no en `TablaInmuebles` por el límite de `useState` del gate: allí ya
 * hay cuatro y un quinto lo supera. La tabla solo monta el modal. */
import { useCallback, useState } from 'react';
import type { Inmueble } from '@/domain/inmueble';

export function useModalRevision() {
  const [inmueble, setInmueble] = useState<Inmueble | null>(null);
  const abrir = useCallback((i: Inmueble) => setInmueble(i), []);
  const cerrar = useCallback(() => setInmueble(null), []);
  return { inmueble, abrir, cerrar };
}
