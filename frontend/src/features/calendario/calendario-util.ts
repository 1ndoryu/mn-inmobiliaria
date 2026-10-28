/* [28AA-6] Utilidades puras del calendario: día de cada publicación, rejilla del
 * mes (semanas que empiezan en lunes) y etiquetas de estado. Sin React. */
import type { EstadoPublicacion, Publicacion } from '@/domain/redes/publicacion-redes';
import { aIso } from '@/hooks/publicaciones/use-calendario';

export const ETIQUETA_ESTADO: Record<EstadoPublicacion, string> = {
  borrador: 'Borrador',
  lista: 'Lista',
  publicada: 'Publicada',
  saltada: 'Saltada',
};

export const CLASE_ESTADO: Record<EstadoPublicacion, string> = {
  borrador: 'border-border bg-muted text-muted-foreground',
  lista: 'border-amber-500/40 bg-amber-500/10 text-amber-700',
  publicada: 'border-emerald-500/40 bg-emerald-500/10 text-emerald-700',
  saltada: 'border-border bg-muted text-muted-foreground line-through',
};

/* Día del calendario: el previsto o, si ya salió, el real. Sin ninguno no entra en la rejilla. */
export function diaDePublicacion(p: Publicacion): string | null {
  if (p.programadaPara) return p.programadaPara;
  if (p.publicadaEn) return aIso(new Date(p.publicadaEn));
  return null;
}

/* Publicaciones por día (`YYYY-MM-DD`), en el orden en que llegan del servidor. */
export function agruparPorDia(publicaciones: Publicacion[]): Map<string, Publicacion[]> {
  const mapa = new Map<string, Publicacion[]>();
  for (const p of publicaciones) {
    const dia = diaDePublicacion(p);
    if (!dia) continue;
    mapa.set(dia, [...(mapa.get(dia) ?? []), p]);
  }
  return mapa;
}

/* Celdas completas de semanas (lunes primero) que cubren el mes: incluye los
 * días de los meses vecinos para que la última fila no quede a medias. */
export function diasDeRejilla(mes: Date): Date[] {
  const anio = mes.getFullYear();
  const numMes = mes.getMonth();
  const inicio = (new Date(anio, numMes, 1).getDay() + 6) % 7;
  const ultimo = new Date(anio, numMes + 1, 0).getDate();
  const celdas = Math.ceil((inicio + ultimo) / 7) * 7;
  /* `Date` absorbe los días fuera de rango (día 0, día 32…) sin cálculos a mano. */
  return Array.from({ length: celdas }, (_, i) => new Date(anio, numMes, i - inicio + 1));
}
