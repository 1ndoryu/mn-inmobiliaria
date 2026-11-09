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
  inmuebles: number;
  creado_en: string;
}

/** Vínculo de UN inmueble con UNA cuenta */
export interface VinculoFacebook {
  facebook_id: string;
  nombre: string;
  url: string;
  principal: boolean;
  marketplace_id: string | null;
  publicado_en: string | null;
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
