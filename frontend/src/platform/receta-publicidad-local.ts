import type { RecetaPublicidad } from '../domain/plantilla-publicidad';

/* Recetas que se editan desde `/publicidad`, por slug. Viven solo en este
 * navegador: no salen al servidor, así que no cambian la imagen que ven los
 * demás visitantes. La receta del admin sigue en el servidor. */
const CLAVE = 'publicidad:recetas-publicas:v1';

function leerTodas(): Record<string, RecetaPublicidad> {
  try {
    const datos = JSON.parse(localStorage.getItem(CLAVE) ?? '{}') as unknown;
    return typeof datos === 'object' && datos !== null ? (datos as Record<string, RecetaPublicidad>) : {};
  } catch {
    return {};
  }
}

export function recetaLocalDe(slug: string): RecetaPublicidad | null {
  return leerTodas()[slug] ?? null;
}

export function guardarRecetaLocal(slug: string, receta: RecetaPublicidad): void {
  try {
    localStorage.setItem(CLAVE, JSON.stringify({ ...leerTodas(), [slug]: receta }));
  } catch {
    // Sin almacenamiento (modo privado, cuota llena) la vista sigue con la receta en memoria.
  }
}
