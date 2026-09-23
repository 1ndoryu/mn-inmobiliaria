// [169A-2] Cliente de solicitudes públicas (MN-Inmobiliaria, sin JWT).
// Sin token: el alta y la subida de fotos son abiertas. Los fallos se
// lanzan como `ErrorApi` con el mensaje del servidor (nunca silenciosos).

import type { BorradorSolicitud } from '../../domain/solicitud';
import { precioAEnviar, puestosAEnviar } from '../../domain/solicitud';
import { API_URL, ErrorApi } from '../inmuebles/api';

export interface FotoSolicitudSubida {
  storage_key: string;
  url: string;
}

interface CuerpoError {
  error?: unknown;
  message?: unknown;
}

function mensajeDeError(cuerpo: CuerpoError | null, estado: number): string {
  return (
    (cuerpo && typeof cuerpo.message === 'string' && cuerpo.message) ||
    (cuerpo && typeof cuerpo.error === 'string' && cuerpo.error) ||
    `El servidor devolvió ${estado}.`
  );
}

async function publica<T>(ruta: string, opciones: RequestInit = {}): Promise<T> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${API_URL}${ruta}`, opciones);
  } catch {
    throw new ErrorApi('No se pudo contactar con el servidor (¿arrancado en ' + API_URL + '?).', 0);
  }
  if (respuesta.status === 204) return undefined as T;
  let cuerpo: CuerpoError | null = null;
  try {
    cuerpo = (await respuesta.json()) as CuerpoError;
  } catch {
    cuerpo = null;
  }
  if (!respuesta.ok) throw new ErrorApi(mensajeDeError(cuerpo, respuesta.status), respuesta.status);
  return cuerpo as T;
}

/* Sube una foto y devuelve su clave (para adjuntarla al alta). El cuerpo
 * son los bytes crudos; el servidor impone tipo (jpg/png/webp) y 10 MiB. */
export async function subirFotoSolicitud(archivo: File | Blob, filename: string): Promise<FotoSolicitudSubida> {
  let respuesta: Response;
  const ruta = `/api/public/solicitudes/fotos?filename=${encodeURIComponent(filename)}`;
  try {
    respuesta = await fetch(`${API_URL}${ruta}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: archivo,
    });
  } catch {
    throw new ErrorApi('No se pudo subir la foto (¿servidor arrancado?).', 0);
  }
  let cuerpo: (FotoSolicitudSubida & CuerpoError) | null = null;
  try {
    cuerpo = (await respuesta.json()) as FotoSolicitudSubida & CuerpoError;
  } catch {
    cuerpo = null;
  }
  if (!respuesta.ok || !cuerpo || typeof cuerpo.storage_key !== 'string')
    throw new ErrorApi(mensajeDeError(cuerpo, respuesta.status), respuesta.status);
  return { storage_key: cuerpo.storage_key, url: cuerpo.url };
}

/* Alta de la solicitud: entra en `pendiente` para revisión del admin. */
export async function crearSolicitud(b: BorradorSolicitud): Promise<unknown> {
  return publica('/api/public/solicitudes', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      nombre: b.nombre.trim(),
      telefono: b.telefono.trim(),
      email: b.email.trim() === '' ? null : b.email.trim(),
      descripcion: b.descripcion.trim(),
      ubicacion: b.ubicacion.trim(),
      residencia: b.residencia.trim(),
      puestos: puestosAEnviar(b.puestos),
      precio_estimado: precioAEnviar(b.precioEstimado),
      operacion: b.operacion,
      fotos: b.fotosClaves,
      origen_contacto: 'web',
    }),
  });
}

export function urlFotoSolicitud(url: string): string {
  return url.startsWith('/') ? `${API_URL}${url}` : url;
}
