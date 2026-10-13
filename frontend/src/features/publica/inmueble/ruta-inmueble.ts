import { origenActual } from '../../../platform/ventana';

/* Rutas públicas por inmueble: `/inmueble/<slug>`. Pura salvo el origen,
 * que sale de la plataforma. */
const PREFIJO = '/inmueble/';

export function rutaInmueble(slug: string): string {
  return `${PREFIJO}${encodeURIComponent(slug)}`;
}

/* Enlace absoluto para compartir (origen del navegador + ruta). */
export function enlaceInmueble(slug: string): string {
  return `${origenActual()}${rutaInmueble(slug)}`;
}

/* Slug de una ruta `/inmueble/<slug>`; `null` si la ruta no es de inmueble
 * (sin slug o con codificación inválida). */
export function slugDeRuta(ruta: string): string | null {
  if (!ruta.startsWith(PREFIJO)) return null;
  const resto = ruta.slice(PREFIJO.length).replace(/\/+$/, '');
  if (resto === '') return null;
  try {
    return decodeURIComponent(resto);
  } catch {
    return null;
  }
}
