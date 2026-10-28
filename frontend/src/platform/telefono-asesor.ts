import { TELEFONO_PUBLICIDAD } from '../domain/plantilla-publicidad';

/* [28AA-9] Teléfono que sale en las piezas de publicidad. Cada asesor lo
 * guarda en el navegador que usa (sin servidor); sin valor, va el de la
 * empresa. Todo acceso a localStorage vive aquí (Sentinel). */

const CLAVE = 'mn.telefono-publicidad';

export function telefonoAsesor(): string {
  try {
    return window.localStorage.getItem(CLAVE)?.trim() || TELEFONO_PUBLICIDAD;
  } catch {
    // Sin almacenamiento (modo privado): vale el de la empresa.
    return TELEFONO_PUBLICIDAD;
  }
}

/* Vacío = quitar lo guardado y volver al de la empresa. `false` = el
 * navegador no dejó guardarlo; el llamador lo dice. */
export function guardarTelefonoAsesor(telefono: string): boolean {
  const limpio = telefono.trim();
  try {
    if (limpio) window.localStorage.setItem(CLAVE, limpio);
    else window.localStorage.removeItem(CLAVE);
    return true;
  } catch {
    return false;
  }
}
