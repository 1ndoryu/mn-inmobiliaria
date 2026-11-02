import { API_URL, ErrorApi, remotoADominio, type InmuebleRemoto } from './api';
import type { InmueblePublico } from '../../domain/inmueble';

/* Web pública (sin JWT): la visibilidad la decide el backend (`publicado = TRUE`).
 * La paginación se pide amplia y el buscador filtra en cliente (la API pública
 * no tiene parámetro `q`). Solo lectura: la receta de `/publicidad` se guarda en
 * el navegador (`platform/receta-publicidad-local.ts`). */

function publicoADominio(r: InmuebleRemoto): InmueblePublico {
  return { ...remotoADominio(r), slug: r.slug };
}

/* Exportado para reusar la misma lectura pública (sin JWT) en otras páginas. */
export async function publicaFetch<T>(ruta: string, init?: RequestInit): Promise<T> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${API_URL}${ruta}`, init);
  } catch {
    throw new ErrorApi('Servidor no disponible.', 0);
  }
  if (!respuesta.ok) {
    let mensaje = `Error ${respuesta.status}`;
    try {
      const cuerpo = (await respuesta.json()) as { error?: string; message?: string };
      mensaje = cuerpo.error ?? cuerpo.message ?? mensaje;
    } catch {
      // El cuerpo no es JSON: se conserva el mensaje por estado.
    }
    throw new ErrorApi(mensaje, respuesta.status);
  }
  return respuesta.json() as Promise<T>;
}

export async function listarPublicos(): Promise<InmueblePublico[]> {
  const r = await publicaFetch<{ items: InmuebleRemoto[] }>('/api/public/inmuebles?page=1&per_page=100');
  return r.items.map(publicoADominio);
}

export async function detallePublico(slug: string): Promise<InmueblePublico> {
  const r = await publicaFetch<InmuebleRemoto>(`/api/public/inmuebles/${encodeURIComponent(slug)}`);
  return publicoADominio(r);
}
