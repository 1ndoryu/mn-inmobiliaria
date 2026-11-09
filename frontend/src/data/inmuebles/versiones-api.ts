/* [30AA-1] Cliente de cuentas de Facebook, versiones de texto y fotos
 * variante. Reutiliza `apiFetch` (sesión + JWT + 401), salvo la subida de
 * bytes, que necesita `Content-Type: application/octet-stream` como las
 * fotos normales. Puro: sin React. */

import { API_URL, ErrorApi, apiFetch, avisarSesionExpirada, leerToken } from './api';
import type {
  CuentaFacebook,
  EstadoVersion,
  FotoVariante,
  SalidaGeneracion,
  VersionesInmueble,
  VinculoFacebook,
} from '../../domain/versiones';

/* ---------- Cuentas de Facebook ---------- */

export function listarCuentas(): Promise<CuentaFacebook[]> {
  return apiFetch<CuentaFacebook[]>('/api/admin/facebook');
}

export function crearCuenta(datos: {
  nombre: string;
  url?: string;
  notas?: string;
  principal?: boolean;
}): Promise<CuentaFacebook> {
  return apiFetch<CuentaFacebook>('/api/admin/facebook', {
    method: 'POST',
    body: JSON.stringify({ url: '', notas: '', principal: false, ...datos }),
  });
}

export function actualizarCuenta(
  id: string,
  datos: { nombre: string; url?: string; notas?: string; principal?: boolean },
): Promise<CuentaFacebook> {
  return apiFetch<CuentaFacebook>(`/api/admin/facebook/${encodeURIComponent(id)}`, {
    method: 'PUT',
    body: JSON.stringify({ url: '', notas: '', principal: false, ...datos }),
  });
}

export function borrarCuenta(id: string): Promise<void> {
  return apiFetch<void>(`/api/admin/facebook/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/* ---------- Vínculo inmueble ↔ cuenta ---------- */

export function vinculosDe(inmuebleId: string): Promise<VinculoFacebook[]> {
  return apiFetch<VinculoFacebook[]>(`/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/facebook`);
}

export function vincularCuenta(
  inmuebleId: string,
  cuentaId: string,
  marketplaceId: string | null,
): Promise<VinculoFacebook[]> {
  return apiFetch<VinculoFacebook[]>(
    `/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/facebook/${encodeURIComponent(cuentaId)}`,
    { method: 'PUT', body: JSON.stringify({ marketplace_id: marketplaceId }) },
  );
}

export function desvincularCuenta(inmuebleId: string, cuentaId: string): Promise<VinculoFacebook[]> {
  return apiFetch<VinculoFacebook[]>(
    `/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/facebook/${encodeURIComponent(cuentaId)}`,
    { method: 'DELETE' },
  );
}

/* ---------- Versiones de texto ---------- */

export function leerVersiones(inmuebleId: string): Promise<VersionesInmueble> {
  return apiFetch<VersionesInmueble>(`/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/versiones`);
}

export function generarVersion(
  inmuebleId: string,
  version: number,
  instruccion = '',
): Promise<SalidaGeneracion> {
  return apiFetch<SalidaGeneracion>(`/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/versiones`, {
    method: 'POST',
    body: JSON.stringify({ version, instruccion }),
  });
}

export function editarVersion(
  inmuebleId: string,
  version: number,
  cambios: { titulo?: string; descripcion?: string; estado?: EstadoVersion },
): Promise<unknown> {
  return apiFetch<unknown>(
    `/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/versiones/${version}`,
    { method: 'PUT', body: JSON.stringify(cambios) },
  );
}

export function borrarVersion(inmuebleId: string, version: number): Promise<void> {
  return apiFetch<void>(`/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/versiones/${version}`, {
    method: 'DELETE',
  });
}

/* ---------- Fotos variante ---------- */

/* Sube la variante IA de la foto que está en `orden`. Va por `fetch` directo
 * porque el cuerpo son bytes de imagen, no JSON. */
export async function subirFotoVariante(
  inmuebleId: string,
  orden: number,
  bytes: ArrayBuffer,
  extension: string,
  modelo: string,
  promptUsado: string,
): Promise<FotoVariante> {
  const consulta = new URLSearchParams({
    orden: String(orden),
    filename: `variante-${orden + 1}.${extension}`,
    modelo,
    prompt_usado: promptUsado.slice(0, 2000),
  });
  const token = leerToken();
  let respuesta: Response;
  try {
    respuesta = await fetch(
      `${API_URL}/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/fotos/variante?${consulta.toString()}`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/octet-stream',
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: bytes,
      },
    );
  } catch {
    throw new ErrorApi('No se pudo contactar con la API al subir la variante.', 0);
  }
  if (respuesta.status === 401) {
    avisarSesionExpirada();
    throw new ErrorApi('Sesión caducada o inválida: vuelve a entrar.', 401);
  }
  if (!respuesta.ok) {
    throw new ErrorApi(`La subida de la variante devolvió ${respuesta.status}.`, respuesta.status);
  }
  return (await respuesta.json()) as FotoVariante;
}

export function borrarFotoVariante(inmuebleId: string, orden: number): Promise<void> {
  return apiFetch<void>(
    `/api/admin/inmuebles/${encodeURIComponent(inmuebleId)}/fotos/variante/${orden}`,
    { method: 'DELETE' },
  );
}

/* ---------- Configuración ---------- */

export function leerConfigVersiones(): Promise<{ versiones: number }> {
  return apiFetch<{ versiones: number }>('/api/admin/configuracion/versiones');
}

export function guardarConfigVersiones(versiones: number): Promise<{ versiones: number }> {
  return apiFetch<{ versiones: number }>('/api/admin/configuracion/versiones', {
    method: 'PUT',
    body: JSON.stringify({ versiones }),
  });
}
