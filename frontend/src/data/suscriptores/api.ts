import { API_URL, ErrorApi } from '../inmuebles/api';

/* Suscripción del pie público (sin JWT): alta idempotente en el backend
 * (re-suscribir no duplica). Los fallos se lanzan como `ErrorApi` con el
 * mensaje del servidor, igual que el resto de clientes públicos. */

export interface Suscriptor {
  id: string;
  email: string;
}

export async function suscribir(correo: string): Promise<Suscriptor> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${API_URL}/api/public/suscriptores`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: correo }),
    });
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
  return respuesta.json() as Promise<Suscriptor>;
}
