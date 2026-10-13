import { useEffect, useState } from 'react';
import type { InmueblePublico } from '../../domain/inmueble';
import { rutaInmueble, slugDeRuta } from '../../features/publica/inmueble/ruta-inmueble';
import { fijarRuta, rutaActual, suscribirEvento, volverAtras } from '../../platform/ventana';

/* Selección del inmueble cuyo detalle público está abierto (null = cerrado).
 * Mientras el modal está abierto, la URL es el enlace del inmueble
 * (`/inmueble/<slug>`) para copiarla; cerrarlo vuelve a la portada y el
 * botón atrás también cierra el modal. Vive en hook propio para no sumar un
 * cuarto `useState` a la página. */
export function useDetallePublico() {
  const [seleccionado, setSeleccionado] = useState<InmueblePublico | null>(null);

  useEffect(
    () =>
      suscribirEvento('popstate', () => {
        if (slugDeRuta(rutaActual()) === null) setSeleccionado(null);
      }),
    [],
  );

  return {
    seleccionado,
    elegir: (i: InmueblePublico) => {
      setSeleccionado(i);
      fijarRuta(rutaInmueble(i.slug));
    },
    cerrar: () => {
      setSeleccionado(null);
      // [por que] Solo se deshace la entrada que añadió `elegir`; si la URL ya no es de inmueble, no se retrocede.
      if (slugDeRuta(rutaActual()) !== null) volverAtras();
    },
  };
}
