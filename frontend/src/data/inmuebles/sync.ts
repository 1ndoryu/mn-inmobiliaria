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

/* Resultado de publicar o traer: lo escrito y, si la operación quedó a medias, el fallo. */
export interface InformeSync {
  slug: string;
  hecho: string[];
  fallo: string | null;
}

/* [16AA-2] Resultado de aplicar el plan: lo escrito en cada lado, lo que quedó
 * pendiente (fotos, fase F3) y el residuo — lo que sigue distinto tras releer. */
export interface InformeAplicarSync {
  slug: string;
  plan_id: string;
  en_local: string[];
  en_servidor: string[];
  pendientes: string[];
  fallo: string | null;
  residuo: string[];
}

/* Escribe el plan en los dos lados. `planId` es el del plan mostrado: si algo
 * cambió desde entonces, el backend responde 409 y no escribe nada. */
export function aplicarResolucionSync(slug: string, planId: string, conservarViejo: string[]): Promise<InformeAplicarSync> {
  return apiFetch<InformeAplicarSync>(`/api/admin/sync/resolver/${encodeURIComponent(slug)}/aplicar`, {
    method: 'POST',
    body: JSON.stringify({ plan_id: planId, conservar_viejo: conservarViejo }),
  });
}

/* Servidor con el que se compara; el diálogo lo muestra antes de escribir en él. */
export interface DestinoSync {
  host: string;
}

export function obtenerDestinoSync(): Promise<DestinoSync> {
  return apiFetch<DestinoSync>('/api/admin/sync/destino');
}

/* Crea en el servidor el inmueble local `slug`: alta, receta, ficha, fotos y visibilidad. */
export function publicarSync(slug: string): Promise<InformeSync> {
  return apiFetch<InformeSync>(`/api/admin/sync/inmuebles/${encodeURIComponent(slug)}/publicar`, { method: 'POST' });
}

/* Crea en el local el inmueble `slug` del servidor, con los mismos pasos. */
export function traerSync(slug: string): Promise<InformeSync> {
  return apiFetch<InformeSync>(`/api/admin/sync/inmuebles/${encodeURIComponent(slug)}/traer`, { method: 'POST' });
}
