import { API_URL, ErrorApi, remotoADominio, type InmuebleRemoto } from './api';
import type { InmueblePublico } from '../../domain/inmueble';

/* Web pública de solo lectura (sin JWT): la visibilidad la decide el
 * backend (`publicado = TRUE`). La paginación se pide amplia y el
 * buscador filtra en cliente (la API pública no tiene parámetro `q`). */

function publicoADominio(r: InmuebleRemoto): InmueblePublico {
  return { ...remotoADominio(r), slug: r.slug };
}

async function publicaFetch<T>(ruta: string): Promise<T> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${API_URL}${ruta}`);
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
