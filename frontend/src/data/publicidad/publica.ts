import { publicaFetch } from '../inmuebles/publica';
import { urlAbsoluta } from '../inmuebles/api';
import type { FormatoPieza, Pieza, PublicidadPublicaItem } from '../../domain/redes/publicacion-redes';

/* [28AA-8] Publicaciones ya salidas para la página pública /publicidad (sin JWT).
 * El servidor manda snake_case; aquí se pasa a camelCase y se resuelven las
 * rutas `/uploads/...` contra la API, igual que en los inmuebles. */

interface PiezaRemota {
  id: string;
  formato: FormatoPieza;
  url: string;
  ancho: number;
  alto: number;
  bytes: number;
}

interface PublicidadRemota {
  inmueble_id: string;
  titulo: string;
  slug: string;
  fotos: string[];
  descripcion: string;
  tags: string[];
  piezas: PiezaRemota[];
  publicada_en: string;
}

function piezaADominio(p: PiezaRemota): Pieza {
  return { ...p, url: urlAbsoluta(p.url) };
}

function publicidadADominio(r: PublicidadRemota): PublicidadPublicaItem {
  return {
    inmuebleId: r.inmueble_id,
    titulo: r.titulo,
    slug: r.slug,
    fotos: r.fotos.map(urlAbsoluta),
    descripcion: r.descripcion,
    tags: r.tags,
    piezas: r.piezas.map(piezaADominio),
    publicadaEn: r.publicada_en,
  };
}

export async function listarPublicidadPublica(): Promise<PublicidadPublicaItem[]> {
  const lista = await publicaFetch<PublicidadRemota[]>('/api/public/publicidad');
  return lista.map(publicidadADominio);
}
