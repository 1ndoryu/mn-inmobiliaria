import { ETIQUETAS_TIPO, fotosVisiblesDe, type Inmueble } from './inmueble';

// Plantilla de la imagen publicitaria: medidas y textos derivados del Figma
// "Post I" (197x262, fondo foto + degradado #EFEFEF arriba + pie curvo).
// Puro: sin DOM ni Canvas, solo constantes y texto.

export const PLANTILLA_ANCHO = 197;
export const PLANTILLA_ALTO = 262;

export const COLOR_FONDO_CLARO = '#EFEFEF';
export const COLOR_TINTA = '#050200';
export const COLOR_ACENTO = '#FEA429';

export const TELEFONO_PUBLICIDAD = '+58424-9208855';
export const WEB_PUBLICIDAD = 'MN-INMOBILIARIA.COM';

export type FormatoPublicidad = 'post-3-4' | 'post-4-5' | 'cuadrado-1-1';

export interface PresetExportacion {
  formato: FormatoPublicidad;
  etiqueta: string;
  anchoPx: number;
  altoPx: number;
}

export const PRESETS_EXPORTACION: PresetExportacion[] = [
  { formato: 'post-3-4', etiqueta: 'Post 3:4', anchoPx: 2160, altoPx: 2873 },
  { formato: 'post-4-5', etiqueta: 'Post 4:5', anchoPx: 2160, altoPx: 2700 },
  { formato: 'cuadrado-1-1', etiqueta: 'Cuadrado 1:1', anchoPx: 2160, altoPx: 2160 },
];

export interface ComposicionPublicidad {
  /** URL resuelta de la foto de fondo (full-bleed con degradado encima). */
  fondo: string;
  /** URL resuelta de la foto del círculo grande (abajo derecha). */
  circularGrande: string;
  /** URL resuelta de la foto del círculo mediano (abajo izquierda). */
  circularMediano: string;
  formato: FormatoPublicidad;
  /** Muestra la pastilla con el precio bajo el título. */
  conPrecio: boolean;
  /** Líneas personalizadas del título; vacío = automático. */
  titulo1?: string;
  titulo2?: string;
}

/* Receta guardada: la composición por ÍNDICES sobre las fotos visibles, no
 * por URL. Las URLs mueren cada vez que se suben/quitan fotos (el servidor
 * regenera todas) y la receta colapsaba a la portada; el índice sobrevive
 * porque el orden se conserva. `resolverReceta` la convierte a URLs. */
export interface RecetaPublicidad {
  fondoIdx: number;
  circularGrandeIdx: number;
  circularMedianoIdx: number;
  formato: FormatoPublicidad;
  conPrecio: boolean;
  titulo1?: string;
  titulo2?: string;
}

/** Primera parte de la ubicación ("Villa Tocoma, ..." -> "Villa Tocoma"). */
export function zonaCortaDe(ubicacion: string): string {
  return ubicacion.split(',')[0]?.trim() || 'la zona';
}

/* Zona del título: la residencia (= urbanización) cuando existe; si no, la
 * zona corta de la ubicación. Una sola fuente para que el título automático,
 * los placeholders y la tarjeta digan siempre lo mismo. */
export function tituloZonaDe(inmueble: Inmueble): string {
  return inmueble.residencia.trim() || zonaCortaDe(inmueble.ubicacion);
}

/** "Town House en venta" (línea 1 del título). */
export function lineaTitulo1De(inmueble: Inmueble): string {
  const tipo = ETIQUETAS_TIPO[inmueble.tipo] ?? inmueble.tipo;
  const operacion = inmueble.operacion === 'alquiler' ? 'alquiler' : 'venta';
  return `${tipo} en ${operacion}`;
}

export interface LineasTitulo {
  linea1: string;
  /** Inicio de la línea 2 en tinta ("en " en automático). */
  prefijo2: string;
  /** Zona de la línea 2 en naranja (vacío en personalizado). */
  zona2: string;
}

/** Líneas del título: personalizadas si hay, si no las automáticas.
 * En automático la línea 2 es la residencia (= urbanización) cuando el
 * inmueble la trae; si no, la zona corta de la ubicación. */
export function lineasTituloDe(
  inmueble: Inmueble,
  comp: Pick<ComposicionPublicidad, 'titulo1' | 'titulo2'>,
): LineasTitulo {
  const t1 = comp.titulo1?.trim();
  const t2 = comp.titulo2?.trim();
  if (t1 || t2) {
    return {
      linea1: t1 || lineaTitulo1De(inmueble),
      prefijo2: t2 || `en ${tituloZonaDe(inmueble)}`,
      zona2: '',
    };
  }
  const zona = tituloZonaDe(inmueble);
  return { linea1: lineaTitulo1De(inmueble), prefijo2: 'en ', zona2: zona };
}

/** "135.000$" (precio en dólares, $ al final pegado al número, como el Figma). */
export function precioCortoDe(precio: number): string {
  const miles = new Intl.NumberFormat('es-VE', { maximumFractionDigits: 0 }).format(precio);
  return `${miles}$`;
}

/** Leyenda bajo el precio dentro de la pastilla ("Listo para firmar"). */
export const LEYENDA_PRECIO = 'Listo para firmar';

/** Rellena los campos nuevos en recetas guardadas antes de que existieran. */
export function normalizarReceta(receta: RecetaPublicidad): RecetaPublicidad {
  return { ...receta, conPrecio: receta.conPrecio ?? true };
}

/* Misma foto con distinta base (absoluta/relativa): se compara por ruta. */
function mismaFoto(a: string, b: string): boolean {
  if (a === b) return true;
  try {
    return new URL(a, 'http://x').pathname === new URL(b, 'http://x').pathname;
  } catch {
    return false;
  }
}

function indiceSano(idx: number, total: number, reserva: number): number {
  if (!Number.isInteger(idx) || idx < 0 || idx >= total) return reserva;
  return idx;
}

/** Receta automática: fondo = portada (1ª), círculo grande = 2ª foto y
 * mediano = 3ª; con menos de 3 fotos se reutiliza la última disponible. */
export function recetaPorDefecto(inmueble: Inmueble): RecetaPublicidad | null {
  const total = fotosVisiblesDe(inmueble).length;
  if (total === 0) return null;
  return {
    fondoIdx: 0,
    circularGrandeIdx: total > 1 ? 1 : 0,
    circularMedianoIdx: total > 2 ? 2 : total > 1 ? 1 : 0,
    formato: 'post-3-4',
    conPrecio: true,
  };
}

/* Receta guardada con el formato viejo (URLs en `fondo`/`circularGrande`/
 * `circularMediano`): se migra a índices emparejando cada URL con la foto
 * visible actual; la que ya no existe cae al valor por defecto. */
function migrarRecetaLegacy(inmueble: Inmueble, guardada: {
  fondo?: unknown; circularGrande?: unknown; circularMediano?: unknown;
  formato?: unknown; conPrecio?: unknown; titulo1?: unknown; titulo2?: unknown;
}): RecetaPublicidad | null {
  if (typeof guardada.fondo !== 'string') return null;
  const visibles = fotosVisiblesDe(inmueble);
  if (visibles.length === 0) return null;
  const defecto = recetaPorDefecto(inmueble)!;
  const aIndice = (url: unknown, reserva: number): number => {
    if (typeof url !== 'string') return reserva;
    const idx = visibles.findIndex((f) => mismaFoto(f, url));
    return idx < 0 ? reserva : idx;
  };
  const formato = PRESETS_EXPORTACION.some((p) => p.formato === guardada.formato)
    ? (guardada.formato as RecetaPublicidad['formato'])
    : defecto.formato;
  const texto = (v: unknown): string | undefined => (typeof v === 'string' ? v : undefined);
  return normalizarReceta({
    fondoIdx: aIndice(guardada.fondo, defecto.fondoIdx),
    circularGrandeIdx: aIndice(guardada.circularGrande, defecto.circularGrandeIdx),
    circularMedianoIdx: aIndice(guardada.circularMediano, defecto.circularMedianoIdx),
    formato,
    conPrecio: typeof guardada.conPrecio === 'boolean' ? guardada.conPrecio : true,
    titulo1: texto(guardada.titulo1),
    titulo2: texto(guardada.titulo2),
  });
}

/** Receta vigente para el inmueble: la guardada (migrada si es legacy) o la
 * automática. Los índices se acotan al total actual para que nunca fallen
 * aunque se añadan o quiten fotos. */
export function recetaVigenteDe(inmueble: Inmueble, guardada: unknown): RecetaPublicidad | null {
  const total = fotosVisiblesDe(inmueble).length;
  if (total === 0) return null;
  const defecto = recetaPorDefecto(inmueble)!;
  if (typeof guardada !== 'object' || guardada === null) return defecto;
  const g = guardada as Record<string, unknown>;
  if (typeof g['fondoIdx'] === 'number') {
    const formato = PRESETS_EXPORTACION.some((p) => p.formato === g['formato']) ? g['formato'] as RecetaPublicidad['formato'] : defecto.formato;
    const texto = (v: unknown): string | undefined => (typeof v === 'string' ? v : undefined);
    return normalizarReceta({
      fondoIdx: indiceSano(g['fondoIdx'] as number, total, defecto.fondoIdx),
      circularGrandeIdx: indiceSano(g['circularGrandeIdx'] as number, total, defecto.circularGrandeIdx),
      circularMedianoIdx: indiceSano(g['circularMedianoIdx'] as number, total, defecto.circularMedianoIdx),
      formato,
      conPrecio: typeof g['conPrecio'] === 'boolean' ? (g['conPrecio'] as boolean) : true,
      titulo1: texto(g['titulo1']),
      titulo2: texto(g['titulo2']),
    });
  }
  return migrarRecetaLegacy(inmueble, g as never) ?? defecto;
}

/** Resuelve la receta a URLs vigentes: las fotos visibles ya traen su mejor
 * versión (mejorada del servidor cuando existe), así que la publicidad la
 * usa sola sin reeditar la receta. */
export function resolverReceta(inmueble: Inmueble, receta: RecetaPublicidad): ComposicionPublicidad {
  const visibles = fotosVisiblesDe(inmueble);
  const base = normalizarReceta(receta);
  const total = visibles.length;
  const url = (idx: number): string => (total === 0 ? '' : (visibles[indiceSano(idx, total, 0)] ?? ''));
  return {
    fondo: url(base.fondoIdx),
    circularGrande: url(base.circularGrandeIdx),
    circularMediano: url(base.circularMedianoIdx),
    formato: base.formato,
    conPrecio: base.conPrecio,
    titulo1: base.titulo1,
    titulo2: base.titulo2,
  };
}

export type IconoSpec = 'habitaciones' | 'banos' | 'metros' | 'puestos' | 'ubicacion';

export interface SpecVisible {
  icono: IconoSpec;
  texto: string;
}

/* Specs de la banda superior según el Figma "Post con Ubicacion y puestos":
 * cama | baño | m² | carro + puestos | pin + zona corta. El 0 (no indicado)
 * oculta icono y valor. Los m² son los construidos cuando hay, y el terreno
 * cuando no hay construidos (una sola spec de área: la distinción fina queda
 * en la ficha y el copy). El pin con la ubicación solo aparece cuando hay residencia
 * (= urbanización): ella va en el título y la ubicación complementa arriba;
 * sin residencia el título ya dice la zona y el pin sería redundante. */
export function specsVisiblesDe(inmueble: Inmueble): SpecVisible[] {
  const specs: SpecVisible[] = [];
  if (inmueble.habitaciones > 0) specs.push({ icono: 'habitaciones', texto: String(inmueble.habitaciones) });
  if (inmueble.banos > 0) specs.push({ icono: 'banos', texto: String(inmueble.banos) });
  const area = inmueble.metros > 0 ? inmueble.metros : inmueble.metrosTerreno;
  if (area > 0) specs.push({ icono: 'metros', texto: `${area} m²` });
  if (inmueble.puestos > 0) specs.push({ icono: 'puestos', texto: String(inmueble.puestos) });
  const residencia = inmueble.residencia.trim();
  const zona = zonaCortaDe(inmueble.ubicacion);
  if (inmueble.ubicacion.trim() !== '' && residencia !== '' && zona.toLowerCase() !== residencia.toLowerCase())
    specs.push({ icono: 'ubicacion', texto: zona });
  return specs;
}
