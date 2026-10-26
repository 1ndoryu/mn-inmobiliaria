/* [25AA-2] Las fotos del plan de sync, contadas como un solo cambio: puro (sin React ni red).
 * El backend compara por posición (`foto:<origen>:<orden>`), así que mover la
 * principal cambia todas las posiciones desplazadas. Aquí se agrupan esas filas
 * y se cuentan en palabras; la elección por clave sigue siendo la del backend. */
import type { FilaPlanSync } from '@/data/inmuebles/sync';

export type EleccionGrupo = 'ultimo' | 'viejo' | 'mixto';

export function esFilaFoto(clave: string): boolean {
  return clave.startsWith('foto:');
}

/* Posición (desde 0) de `foto:<origen>:<orden>`; `null` si la clave no es de foto. */
function posicion(clave: string): number | null {
  const n = Number(clave.split(':')[2]);
  return esFilaFoto(clave) && Number.isInteger(n) ? n : null;
}

/* Lo elegido para todo el grupo: «viejo» si todas están en `conservarViejo`, «ultimo» si ninguna. */
export function eleccionDeGrupo(claves: string[], conservarViejo: string[]): EleccionGrupo {
  const viejas = claves.filter((c) => conservarViejo.includes(c)).length;
  if (viejas === 0) return 'ultimo';
  return viejas === claves.length ? 'viejo' : 'mixto';
}

/* Posiciones distintas (1, 2, 3…) de un conjunto de filas: una foto y su mejorada cuentan una vez. */
function posiciones(filas: FilaPlanSync[]): number[] {
  const orden = filas.map((f) => posicion(f.clave)).filter((n): n is number => n !== null);
  return [...new Set(orden)].sort((a, b) => a - b);
}

function plural(n: number, uno: string, varios: string): string {
  return `${n} ${n === 1 ? uno : varios}`;
}

/* Frases de lo que hará el plan en cada lado; el orden es el de lectura: primero el hecho, luego el efecto. */
export function resumirFotos(filas: FilaPlanSync[]): string[] {
  const fotos = filas.filter((f) => esFilaFoto(f.clave));
  if (fotos.length === 0) return [];
  const lineas: string[] = [];

  /* Misma posición con otra foto en cada lado = reorden o reemplazo, no fotos nuevas. */
  const distintas = fotos.filter((f) => f.local !== null && f.servidor !== null);
  const principal = distintas.some((f) => posicion(f.clave) === 0);
  if (principal) lineas.push('La foto principal (la 1.ª) es distinta en cada lado.');
  const sueltas = posiciones(distintas).filter((n) => n !== 0);
  if (sueltas.length > 0) {
    lineas.push(
      `${plural(sueltas.length, 'posición tiene', 'posiciones tienen')} otra foto en cada lado` +
        `${distintas.length > 1 ? ' (suele pasar al mover la principal o cambiar el orden: todo lo que se desplaza cambia de posición)' : ''}.`,
    );
  }

  /* Efecto por lado de destino: lo que se escribe, lo que se añade y lo que se quita. */
  for (const destino of ['local', 'servidor'] as const) {
    const aqui = fotos.filter((f) => (f.elegido === 'local' ? 'servidor' : 'local') === destino);
    if (aqui.length === 0) continue;
    const reemplazan = aqui.filter((f) => f.accion === 'escribir' && f.local !== null && f.servidor !== null);
    const anaden = aqui.filter((f) => f.accion === 'escribir' && (destino === 'local' ? f.local : f.servidor) === null);
    const quitan = aqui.filter((f) => f.accion === 'quitar');
    const partes = [
      reemplazan.length > 0 && `se cambian ${plural(posiciones(reemplazan).length, 'posición', 'posiciones')}`,
      anaden.length > 0 && `se ${anaden.length === 1 ? 'añade' : 'añaden'} ${plural(posiciones(anaden).length, 'foto', 'fotos')}`,
      quitan.length > 0 && `se ${quitan.length === 1 ? 'quita' : 'quitan'} ${plural(posiciones(quitan).length, 'foto', 'fotos')}`,
    ].filter((p): p is string => typeof p === 'string');
    if (partes.length > 0) lineas.push(`En ${destino === 'local' ? 'Local' : 'el Servidor'}: ${partes.join(', ')}.`);
  }
  return lineas;
}
