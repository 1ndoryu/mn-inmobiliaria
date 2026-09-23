// Historial visible de mejoras: dominio puro, sin DOM ni storage.
// Cada transición relevante (encolada, lista, error, cancelado) deja una
// entrada con fecha para revisar qué pasó sin abrir la consola ni el backend.

export type TipoEventoMejora = 'encolada' | 'lista' | 'error' | 'cancelado';

export interface EventoMejora {
  id: string;
  fecha: string;
  fotoId: string;
  inmuebleId: string;
  tipo: TipoEventoMejora;
  /** Detalle corto: motivo del error o nº de intento. */
  detalle: string | null;
}

export const CLAVE_HISTORIAL_MEJORA = 'inmobiliaria:mejora-historial:v1';
export const MAX_HISTORIAL = 200;

export function crearEvento(fotoId: string, inmuebleId: string, tipo: TipoEventoMejora, detalle: string | null): EventoMejora {
  const id =
    typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
  return { id, fecha: new Date().toISOString(), fotoId, inmuebleId, tipo, detalle };
}

/* Normaliza lo guardado y recorta a las últimas MAX entradas. */
export function normalizarHistorial(datos: unknown): EventoMejora[] {
  if (!Array.isArray(datos)) return [];
  const validos = datos.filter(
    (e): e is EventoMejora =>
      typeof e === 'object' &&
      e !== null &&
      typeof (e as EventoMejora).id === 'string' &&
      typeof (e as EventoMejora).fecha === 'string' &&
      typeof (e as EventoMejora).fotoId === 'string' &&
      typeof (e as EventoMejora).inmuebleId === 'string' &&
      ((e as EventoMejora).tipo === 'encolada' ||
        (e as EventoMejora).tipo === 'lista' ||
        (e as EventoMejora).tipo === 'error' ||
        (e as EventoMejora).tipo === 'cancelado'),
  );
  return validos
    .sort((a, b) => a.fecha.localeCompare(b.fecha))
    .slice(-MAX_HISTORIAL);
}
