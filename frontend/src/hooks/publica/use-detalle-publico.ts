import { useState } from 'react';
import type { InmueblePublico } from '../../domain/inmueble';

/* Selección del inmueble cuyo detalle público está abierto (null = cerrado).
 * Vive en hook propio para no sumar un cuarto `useState` a la página. */
export function useDetallePublico() {
  const [seleccionado, setSeleccionado] = useState<InmueblePublico | null>(null);
  return {
    seleccionado,
    elegir: (i: InmueblePublico) => setSeleccionado(i),
    cerrar: () => setSeleccionado(null),
  };
}
