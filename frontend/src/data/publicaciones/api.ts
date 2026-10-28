/* [28AA-6] Cliente admin del calendario de redes. `apiFetch` gestiona sesión,
 * JWT y 401. Las piezas viajan como bytes crudos, igual que las fotos. */
import {
  DESTINOS_PUBLICACION,
  type Destinos,
  type EstadoPublicacion,
  type FormatoPieza,
  type Pieza,
  type Propuesta,
  type Publicacion,
} from '../../domain/redes/publicacion-redes';
import { apiFetch, ErrorApi } from '../inmuebles/api';

interface DestinosApi {
  ig_post: boolean;
  fanpage: boolean;
  ig_historia: boolean;
  fb_perfil: boolean;
}

interface PiezaApi {
  id: string;
  formato: string;
  url: string;
  ancho: number;
  alto: number;
  bytes: number;
}

interface PublicacionApi {
  id: string;
  inmueble_id: string;
  titulo_inmueble: string;
  estado: string;
  descripcion: string;
  tags: string[];
  modelo: string;
  destinos: DestinosApi;
  programada_para: string | null;
  publicada_en: string | null;
  piezas: PiezaApi[];
  creada_en: string;
}

interface PropuestaApi {
  inmueble_id: string;
  titulo: string;
}

/* Cambios parciales: lo que no se pasa no se toca en el servidor. */
export interface CambiosPublicacion {
  descripcion?: string;
  tags?: string[];
  /* El servidor trata la fecha como valor completo: enviar siempre la actual al editar. */
  programadaPara?: string | null;
  destinos?: Destinos;
  promptUsado?: string;
  modelo?: string;
}

/* Mapeo de destinos camelCase (dominio) ↔ snake_case (API), en el orden del dominio. */
const CLAVE_API: Record<keyof Destinos, keyof DestinosApi> = {
  igPost: 'ig_post',
  fanpage: 'fanpage',
  igHistoria: 'ig_historia',
  fbPerfil: 'fb_perfil',
};

function aDestinosApi(d: Destinos): DestinosApi {
  const salida = {} as DestinosApi;
  for (const k of DESTINOS_PUBLICACION) salida[CLAVE_API[k]] = d[k];
  return salida;
}

function aDestinos(d: DestinosApi): Destinos {
  const salida = {} as Destinos;
  for (const k of DESTINOS_PUBLICACION) salida[k] = d[CLAVE_API[k]];
  return salida;
}

function aPieza(p: PiezaApi): Pieza {
  return {
    id: p.id,
    // El servidor solo guarda los formatos del CHECK de BD: el cast es seguro.
    formato: p.formato as FormatoPieza,
    url: p.url,
    ancho: p.ancho,
    alto: p.alto,
    bytes: p.bytes,
  };
}

function aPublicacion(p: PublicacionApi): Publicacion {
  return {
    id: p.id,
    inmuebleId: p.inmueble_id,
    tituloInmueble: p.titulo_inmueble,
    estado: p.estado as EstadoPublicacion,
    descripcion: p.descripcion,
    tags: p.tags,
    modelo: p.modelo,
    destinos: aDestinos(p.destinos),
    programadaPara: p.programada_para,
    publicadaEn: p.publicada_en,
    piezas: p.piezas.map(aPieza),
    creadaEn: p.creada_en,
  };
}

/* Publicaciones que caen en el rango (días `YYYY-MM-DD`). El servidor limita el rango. */
export async function listarPublicaciones(desde: string, hasta: string): Promise<Publicacion[]> {
  const query = new URLSearchParams({ desde, hasta });
  const lista = await apiFetch<PublicacionApi[]>(`/api/admin/publicaciones?${query.toString()}`);
  return lista.map(aPublicacion);
}

/* Siguiente inmueble de la cola. Devuelve `null` si ya no queda ninguno publicable (404 del servidor). */
export async function proponerSiguiente(excluir?: string): Promise<Propuesta | null> {
  try {
    const p = await apiFetch<PropuestaApi>('/api/admin/publicaciones/siguiente', {
      method: 'POST',
      body: JSON.stringify({ excluir: excluir ?? null }),
    });
    return { inmuebleId: p.inmueble_id, titulo: p.titulo };
  } catch (e) {
    if (e instanceof ErrorApi && e.estado === 404) return null;
    throw e;
  }
}

export async function crearPublicacion(inmuebleId: string, programadaPara?: string | null): Promise<Publicacion> {
  const r = await apiFetch<PublicacionApi>('/api/admin/publicaciones', {
    method: 'POST',
    body: JSON.stringify({ inmueble_id: inmuebleId, programada_para: programadaPara ?? null }),
  });
  return aPublicacion(r);
}

/* Saltar: queda registrado como visto, no cuenta como publicado. */
export async function saltarPropuesta(inmuebleId: string): Promise<void> {
  await apiFetch<void>('/api/admin/publicaciones/saltar', {
    method: 'POST',
    body: JSON.stringify({ inmueble_id: inmuebleId }),
  });
}

export async function actualizarPublicacion(id: string, cambios: CambiosPublicacion): Promise<Publicacion> {
  const cuerpo: Record<string, unknown> = {};
  if (cambios.descripcion !== undefined) cuerpo.descripcion = cambios.descripcion;
  if (cambios.tags !== undefined) cuerpo.tags = cambios.tags;
  if (cambios.programadaPara !== undefined) cuerpo.programada_para = cambios.programadaPara;
  if (cambios.destinos !== undefined) cuerpo.destinos = aDestinosApi(cambios.destinos);
  if (cambios.promptUsado !== undefined) cuerpo.prompt_usado = cambios.promptUsado;
  if (cambios.modelo !== undefined) cuerpo.modelo = cambios.modelo;
  const r = await apiFetch<PublicacionApi>(`/api/admin/publicaciones/${encodeURIComponent(id)}`, {
    method: 'PUT',
    body: JSON.stringify(cuerpo),
  });
  return aPublicacion(r);
}

/* Borra un borrador. El servidor rechaza (409) las ya publicadas. */
export async function borrarPublicacion(id: string): Promise<void> {
  await apiFetch<void>(`/api/admin/publicaciones/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/* Historial: cada vez que el inmueble salió publicado, la más reciente primero. */
export async function historialDe(inmuebleId: string): Promise<Publicacion[]> {
  const lista = await apiFetch<PublicacionApi[]>(
    `/api/admin/publicaciones/historial/${encodeURIComponent(inmuebleId)}`,
  );
  return lista.map(aPublicacion);
}

/* Sube una pieza ya recortada al formato. El servidor valida formato y bytes. */
export async function subirPieza(publicacionId: string, archivo: Blob, filename: string): Promise<Pieza> {
  const query = new URLSearchParams({ filename });
  const r = await apiFetch<PiezaApi>(
    `/api/admin/publicaciones/${encodeURIComponent(publicacionId)}/piezas?${query.toString()}`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: archivo,
    },
  );
  return aPieza(r);
}

export async function borrarPieza(piezaId: string): Promise<void> {
  await apiFetch<void>(`/api/admin/publicaciones/piezas/${encodeURIComponent(piezaId)}`, { method: 'DELETE' });
}
