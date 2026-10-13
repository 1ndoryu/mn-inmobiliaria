import type { InmueblePublico } from '../../../domain/inmueble';

/* Inmuebles similares al actual, solo con la lista ya cargada (sin pedir
 * nada más). Orden: disponibles primero, mismo tipo, misma operación y, a
 * igualdad, precio más cercano. Sin precio (A convenir) va al final. */
export function inmueblesSimilares(lista: InmueblePublico[], actual: InmueblePublico, maximo = 3): InmueblePublico[] {
  const distancia = (precio: number): number =>
    precio > 0 && actual.precio > 0 ? Math.abs(precio - actual.precio) : Number.POSITIVE_INFINITY;
  const puntuados = lista
    .filter((o) => o.slug !== actual.slug)
    .map((o) => ({
      o,
      disponible: o.estado === 'disponible' ? 1 : 0,
      tipo: o.tipo === actual.tipo ? 1 : 0,
      operacion: o.operacion === actual.operacion ? 1 : 0,
      precio: distancia(o.precio),
    }));
  puntuados.sort((a, b) => b.disponible - a.disponible || b.tipo - a.tipo || b.operacion - a.operacion || comparar(a.precio, b.precio));
  return puntuados.slice(0, maximo).map((p) => p.o);
}

// [por que] Infinity - Infinity es NaN y rompe el orden del sort; se compara sin restar.
function comparar(a: number, b: number): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
