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

/* Lado cuyo valor queda; `accion` es lo que se hace en el otro lado. */
export type EleccionSync = 'local' | 'servidor';
export type AccionSync = 'escribir' | 'quitar';

export interface FilaPlanSync {
  clave: string;
  lado: LadoSync;
  elegido: EleccionSync;
  accion: AccionSync;
  /* `null` = el elemento no existe en ese lado. */
  local: string | null;
  servidor: string | null;
  /* Solo fotos: `/uploads/...` en local; URL absoluta en el servidor. */
  miniatura_local: string | null;
  miniatura_servidor: string | null;
}

export interface PlanResolucionSync {
  slug: string;
  plan_id: string;
  estado: EstadoSyncInmueble;
  estimado: boolean;
  en_local: number;
  en_servidor: number;
  filas: FilaPlanSync[];
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

/* Solo calcula el plan: no escribe. `conservarViejo` = claves donde gana el lado contrario a «lo último». */
export function planificarResolucionSync(slug: string, conservarViejo: string[]): Promise<PlanResolucionSync> {
  return apiFetch<PlanResolucionSync>(`/api/admin/sync/resolver/${encodeURIComponent(slug)}/plan`, {
    method: 'POST',
    body: JSON.stringify({ conservar_viejo: conservarViejo }),
  });
}
