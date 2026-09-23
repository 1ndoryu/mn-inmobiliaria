import { apiFetch, ErrorApi } from '@/data/inmuebles/api';
import type { CompletarIA, EstadoIA, EstadoProveedorIA, ProbarIA, ProveedorIA } from '@/domain/ia';

/* Cliente del centro de IA de texto del backend [199A-2]: sesion JWT + 401
 * via `apiFetch` (nada de claves en el front: viven en el `.env` del
 * servidor). Los fallos de transporte se devuelven como motivo, nunca throw
 * (salvo `guardar`, que el hook convierte en mensaje). */

/* El backend habla snake_case: se traduce una sola vez aqui. */
interface EstadoProveedorCrudo {
  id: string;
  nombre: string;
  modelo: string;
  habilitado: boolean;
  configurado: boolean;
  estado: string;
  comprobado_en: number | null;
  latencia_ms: number | null;
  ultimo_modelo: string | null;
  ultimo_error: string | null;
}

interface EstadoCrudo {
  activo: ProveedorIA;
  proveedores: EstadoProveedorCrudo[];
}

function aEstadoProveedor(c: EstadoProveedorCrudo): EstadoProveedorIA {
  return {
    id: c.id,
    nombre: c.nombre,
    modelo: c.modelo,
    habilitado: c.habilitado,
    configurado: c.configurado,
    estado: c.estado,
    comprobadoEn: c.comprobado_en,
    latenciaMs: c.latencia_ms,
    ultimoModelo: c.ultimo_modelo,
    ultimoError: c.ultimo_error,
  };
}

export async function leerEstadoIA(signal?: AbortSignal): Promise<EstadoIA> {
  const crudo = await apiFetch<EstadoCrudo>('/api/admin/ia/estado', { signal });
  return { activo: crudo.activo, proveedores: crudo.proveedores.map(aEstadoProveedor) };
}

export async function guardarConfigIA(
  activo: ProveedorIA,
  gloryapiHabilitado: boolean,
  opencodeGoHabilitado: boolean,
  signal?: AbortSignal,
): Promise<void> {
  await apiFetch<{ ok: boolean }>('/api/admin/ia/config', {
    method: 'PUT',
    body: JSON.stringify({
      activo,
      gloryapi_habilitado: gloryapiHabilitado,
      opencode_go_habilitado: opencodeGoHabilitado,
    }),
    signal,
  });
}

export async function probarProveedor(id: ProveedorIA, signal?: AbortSignal): Promise<ProbarIA> {
  try {
    const r = await apiFetch<{ ok: boolean; latencia_ms: number; modelo: string | null; error: string | null }>(
      '/api/admin/ia/probar',
      { method: 'POST', body: JSON.stringify({ proveedor: id }), signal },
    );
    return { ok: r.ok, latenciaMs: r.latencia_ms, modelo: r.modelo, error: r.error };
  } catch (e) {
    return { ok: false, latenciaMs: 0, modelo: null, error: mensajeError(e, signal) };
  }
}

interface CompletarCrudo {
  ok: boolean;
  texto: string | null;
  proveedor: string | null;
  modelo: string | null;
  motivos: string[];
}

export async function completarIA(
  system: string,
  texto: string,
  fotos: string[],
  signal?: AbortSignal,
): Promise<CompletarIA> {
  try {
    return await apiFetch<CompletarCrudo>('/api/admin/ia/completar', {
      method: 'POST',
      body: JSON.stringify({ system, texto, fotos }),
      signal,
    });
  } catch (e) {
    return { ok: false, texto: null, proveedor: null, modelo: null, motivos: [mensajeError(e, signal)] };
  }
}

function mensajeError(e: unknown, signal?: AbortSignal): string {
  if (signal?.aborted) return 'Petición cancelada.';
  return e instanceof ErrorApi ? e.message : 'Error inesperado de la IA.';
}
