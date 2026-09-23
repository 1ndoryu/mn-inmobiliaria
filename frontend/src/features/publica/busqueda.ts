import {
  ETIQUETAS_TIPO,
  type InmueblePublico,
  type Operacion,
  type TipoInmueble,
} from '../../domain/inmueble';

/* Normalización para búsqueda inteligente: minúsculas, sin tildes y con
 * espacios colapsados, así "busqueda  villa" casa con "Búsqueda en Villa". */
export function normalizar(texto: string): string {
  return texto
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

export type RegistroBusqueda = { i: InmueblePublico; texto: string };

/* Índice normalizado: se calcula una vez por carga, no por tecla. */
export function construirIndice(inmuebles: InmueblePublico[]): RegistroBusqueda[] {
  return inmuebles.map((i) => ({
    i,
    texto: normalizar(
      [i.titulo, i.ubicacion, i.residencia ?? '', i.descripcion, ETIQUETAS_TIPO[i.tipo] ?? '', i.operacion].join(' '),
    ),
  }));
}

export function extraerTerminos(busqueda: string): string[] {
  return normalizar(busqueda).split(' ').filter(Boolean);
}

/* Filtros avanzados del modal: operación, rango de precio, rango de metros
 * (m² de propiedad, el que muestran las tarjetas), ubicación y habitaciones
 * mínimas. Los numéricos viajan como texto (igual que en el modal publicar)
 * y lo no numérico se ignora al filtrar. */
export interface FiltrosAvanzados {
  operacion: Operacion | 'todas';
  precioMin: string;
  precioMax: string;
  metrosMin: string;
  metrosMax: string;
  ubicacion: string;
  habitacionesMin: string;
}

export const FILTROS_AVANZADOS_VACIOS: FiltrosAvanzados = {
  operacion: 'todas',
  precioMin: '',
  precioMax: '',
  metrosMin: '',
  metrosMax: '',
  ubicacion: '',
  habitacionesMin: '',
};

export function hayFiltrosAvanzados(f: FiltrosAvanzados): boolean {
  return (
    f.operacion !== 'todas' ||
    f.precioMin.trim() !== '' ||
    f.precioMax.trim() !== '' ||
    f.metrosMin.trim() !== '' ||
    f.metrosMax.trim() !== '' ||
    f.ubicacion.trim() !== '' ||
    f.habitacionesMin.trim() !== ''
  );
}

/* Cada término debe aparecer en algún campo (título, ubicación,
 * descripción, tipo u operación); el filtro de tipo combina con AND. */
export function filtrarIndice(
  indice: RegistroBusqueda[],
  filtro: TipoInmueble | null,
  terminos: string[],
  avanzados: FiltrosAvanzados = FILTROS_AVANZADOS_VACIOS,
): InmueblePublico[] {
  const min = Number(avanzados.precioMin);
  const max = Number(avanzados.precioMax);
  const mMin = Number(avanzados.metrosMin);
  const mMax = Number(avanzados.metrosMax);
  const hab = Number(avanzados.habitacionesMin);
  const ubi = normalizar(avanzados.ubicacion);
  return indice
    .filter(
      (r) =>
        (filtro === null || r.i.tipo === filtro) &&
        terminos.every((t) => r.texto.includes(t)) &&
        (avanzados.operacion === 'todas' || r.i.operacion === avanzados.operacion) &&
        (!Number.isFinite(min) || min <= 0 || r.i.precio >= min) &&
        (!Number.isFinite(max) || max <= 0 || r.i.precio <= max) &&
        (!Number.isFinite(mMin) || mMin <= 0 || r.i.metros >= mMin) &&
        (!Number.isFinite(mMax) || mMax <= 0 || r.i.metros <= mMax) &&
        (ubi === '' || normalizar(r.i.ubicacion).includes(ubi)) &&
        (!Number.isFinite(hab) || hab <= 0 || r.i.habitaciones >= hab),
    )
    .map((r) => r.i);
}
