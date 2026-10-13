// [13AA-4] Lectura del perfil de Facebook Marketplace para el modal «Vincular
// con Facebook». Solo lee: vincular pasa por `actualizarInmueble` (el PUT
// completo de siempre). Los nombres de campo son los del JSON del servidor.

import { apiFetch } from './api';

export interface CandidatoAviso {
  inmueble_id: string;
  titulo: string;
  precio: number;
  /** Un título contiene al otro (coincidencia fuerte). */
  directo: boolean;
  palabras_comunes: number;
}

export interface AvisoPendiente {
  aviso_id: string;
  titulo: string;
  precio: string | null;
  url: string;
  /** Foto principal del aviso en Facebook (`null` si la lab no la leyó). */
  imagen_url: string | null;
  /** Los más parecidos primero. */
  candidatos: CandidatoAviso[];
}

export interface InmuebleSinAviso {
  inmueble_id: string;
  titulo: string;
  precio: number;
}

export interface PerfilAvisos {
  /** Hora de la última lectura de la lab (`null` si nunca se ha leído). */
  ultima_lectura: string | null;
  avisos: AvisoPendiente[];
  inmuebles_sin_aviso: InmuebleSinAviso[];
}

export function cargarPerfilAvisos(): Promise<PerfilAvisos> {
  return apiFetch<PerfilAvisos>('/api/admin/marketplace/perfil/avisos');
}
