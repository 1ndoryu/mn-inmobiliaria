/* [13AA-12] Cliente del panel Sync (solo lectura). Los tipos espejan los
 * DTO del backend tal cual viajan (snake_case). */
import type { EstadoSyncInmueble, LadoSync } from '../../domain/sync/sync-inmueble';
import { apiFetch } from './api';

export interface DiferenciaSync {
  clave: string;
  lado: LadoSync;
}

export interface ResultadoSync {
  estado: EstadoSyncInmueble;
  estimado: boolean;
  diferencias: DiferenciaSync[];
}

export interface EntradaSync {
  slug: string;
  resultado: ResultadoSync;
}

export interface SoloServidorSync {
  slug: string;
  titulo: string;
}

export interface ComparacionSync {
  inmuebles: EntradaSync[];
  solo_servidor: SoloServidorSync[];
}

/* Tagged union sobre `estado` (serde `tag = "estado"`). */
export type EstadoRemotoSync =
  | { estado: 'iniciando' }
  | { estado: 'desactivado' }
  | { estado: 'sin_credenciales' }
  | { estado: 'sin_conexion'; motivo: string }
  | { estado: 'sin_soporte'; motivo: string }
  | { estado: 'activo' };

export interface EstadoSync {
  revision: number;
  generado_en: string;
  remoto: EstadoRemotoSync;
  fotos_pendientes_local: number;
  comparacion: ComparacionSync | null;
}

export interface FilaDiferenciaSync {
  clave: string;
  lado: LadoSync;
  /* `null` = el elemento no existe en ese lado. */
  local: string | null;
  servidor: string | null;
}

export interface DiferenciasSync {
  slug: string;
  resultado: ResultadoSync;
  filas: FilaDiferenciaSync[];
}

/* Long-poll: sin `desde` responde al instante; con él, espera a que cambie la revisión. */
export function esperarEstadoSync(desde: number | undefined, signal: AbortSignal): Promise<EstadoSync> {
  const q = desde === undefined ? '' : `?desde=${desde}`;
  return apiFetch<EstadoSync>(`/api/admin/sync/estado${q}`, { signal });
}

/* Pide al backend consultar al servidor ya; el resultado llega por `esperarEstadoSync`. */
export function comprobarSync(): Promise<void> {
  return apiFetch<void>('/api/admin/sync/comprobar', { method: 'POST' });
}

export function diferenciasSync(slug: string): Promise<DiferenciasSync> {
  return apiFetch<DiferenciasSync>(`/api/admin/sync/diferencias/${encodeURIComponent(slug)}`);
}
