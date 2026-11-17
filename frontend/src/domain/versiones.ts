/* [30AA-1] Versionado de inmuebles: cuentas de Facebook, variantes de texto
 * (v2 en adelante) y fotos variante. Puro: sin React ni red.
 *
 * Regla de negocio que el front debe respetar al pintar: la web pública
 * muestra SIEMPRE la v1 (la ficha); las variantes solo se ven en el panel
 * interno. Por eso ningún componente público importa este módulo. */

/** Cuenta de Facebook con su número de inmuebles vinculados */
export interface CuentaFacebook {
  id: string;
  nombre: string;
  url: string;
  notas: string | null;
  principal: boolean;
  /** [30AA-13] Versión que publica esta cuenta (v1 = Maye, v2 = Wan). */
  version: number | null;
  inmuebles: number;
  creado_en: string;
}

/** Vínculo de UNA versión de un inmueble con UNA cuenta.
 * [30AA-7] `version` es la del anuncio (1 = ficha): la v2 se publica como
 * aviso distinto y tiene su propio `marketplace_id`. */
export interface VinculoFacebook {
  facebook_id: string;
  nombre: string;
  url: string;
  principal: boolean;
  version: number;
  marketplace_id: string | null;
  publicado_en: string | null;
}

/** Versiones sin aviso en ninguna cuenta (lo calcula el backend). */
export interface FaltanAvisos {
  versiones: number[];
  total: number;
}

/** Resumen del vínculo de TODAS las versiones, para la columna de la tabla. */
export interface ResumenVinculos {
  faltan: number[];
  total: number;
  /** «al día», «falta v1» o «faltan v2, v3». */
  etiqueta: string;
  /** Cierto si alguna versión sigue sin aviso. */
  incompleto: boolean;
}

/* [30AA-7] Traduce lo que manda el backend a una frase corta. `undefined`
 * (respuesta vieja o API pública) devuelve `null`: no se sabe, no se avisa.
 * Pura. */
export function resumenVinculos(faltan: FaltanAvisos | undefined | null): ResumenVinculos | null {
  if (!faltan) return null;
  const versiones = [...faltan.versiones].sort((a, b) => a - b);
  const total = versiones.length;
  if (total === 0) return { faltan: versiones, total, etiqueta: 'al día', incompleto: false };
  const lista = versiones.map((v) => `v${v}`).join(', ');
  return {
    faltan: versiones,
    total,
    etiqueta: total === 1 ? `falta ${lista}` : `faltan ${lista}`,
    incompleto: true,
  };
}

export type EstadoVersion = 'borrador' | 'lista';

/** Texto de una versión (v2+). La v1 vive en `Inmueble`, no aquí. */
export interface VersionTexto {
  id: string;
  inmueble_id: string;
  version: number;
  titulo: string;
  descripcion: string;
  estado: EstadoVersion;
  modelo: string;
  prompt_usado: string;
  creada_en: string;
  actualizada_en: string;
}

/** Variante de foto: MISMA foto que su original, tomada desde otro ángulo */
export interface FotoVariante {
  id: string;
  orden: number;
  version: number;
  url: string;
  creada_en: string;
}

export interface EntradaHistorial {
  id: string;
  version: number;
  campo: 'titulo' | 'descripcion';
  valor_anterior: string;
  valor_nuevo: string;
  origen: 'ia' | 'manual';
  modelo: string;
  creado_en: string;
}

/** Todo lo que pinta la pestaña de versiones del modal */
export interface VersionesInmueble {
  configuradas: number;
  titulo_v1: string;
  descripcion_v1: string;
  variantes: VersionTexto[];
  fotos: FotoVariante[];
  historial: EntradaHistorial[];
}

/** Resultado de pedir una variante a la IA. `ok: false` trae el motivo. */
export interface SalidaGeneracion {
  ok: boolean;
  version: VersionTexto | null;
  modelo: string | null;
  motivos: string[];
}

/** La v2 de un inmueble, si existe (atajo para el modal) */
export function versionNumero(v: VersionesInmueble, n: number): VersionTexto | null {
  return v.variantes.find((x) => x.version === n) ?? null;
}

/** Variantes de foto de una versión, ordenadas por `orden` */
export function fotosDeVersion(v: VersionesInmueble, n: number): FotoVariante[] {
  return v.fotos.filter((f) => f.version === n).sort((a, b) => a.orden - b.orden);
}

/** ¿Hay algo que enseñar en esa pestaña? La v1 siempre; la v2 solo con texto
 * o con fotos variante. */
export function hayVersion(v: VersionesInmueble, n: number): boolean {
  if (n <= 1) return true;
  return versionNumero(v, n) !== null || fotosDeVersion(v, n).length > 0;
}

/* [30AA-7] Versiones que ESTA ficha tiene de verdad: la 1 y las generadas
 * (texto o fotos variante). La configuración global no basta: un inmueble sin
 * v2 no «debe» un aviso de v2. Pura. */
export function versionesDe(v: VersionesInmueble): number[] {
  const propias = new Set<number>([1]);
  for (const t of v.variantes) propias.add(t.version);
  for (const f of v.fotos) propias.add(f.version);
  return [...propias].sort((a, b) => a - b);
}

/* ---------- [30AA-14] Revisión de todas las versiones de las fotos ---------- */

/** Una foto de un `orden` en una versión concreta. */
export interface VersionDeFoto {
  /** `original`, `mejorada` o `variante:<n>`. */
  origen: string;
  /** 0 en original/mejorada; 2, 3… en las variantes. */
  version: number;
  url: string;
  /** % de diferencia de su histograma contra el de la original (`null` en la
   * original, o si la foto no se pudo leer). */
  diferencia_pct: number | null;
}

/** Todas las versiones de una misma foto: la original primero. */
export interface FotoComparada {
  orden: number;
  versiones: VersionDeFoto[];
}

/** Lo que devuelve `GET /inmuebles/:id/fotos/versiones`. */
export interface RevisionFotos {
  inmueble_id: string;
  titulo: string;
  fotos: FotoComparada[];
}

/** Etiqueta corta de una versión de foto: «Original», «Mejorada», «v2»… */
export function etiquetaDeVersion(v: VersionDeFoto): string {
  if (v.origen === 'original') return 'Original';
  if (v.origen === 'mejorada') return 'Mejorada';
  const n = v.origen.startsWith('variante:') ? v.origen.slice('variante:'.length) : String(v.version);
  return `v${n}`;
}
