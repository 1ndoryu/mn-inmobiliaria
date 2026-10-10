/* Tipos del centro de IA de texto del backend [199A-1/199A-2]: estado,
 * configuracion y completar con fallback entre proveedores. Puro: sin red. */

export type ProveedorIA = 'gloryapi' | 'opencode-go';

export interface EstadoProveedorIA {
  id: string;
  nombre: string;
  modelo: string;
  habilitado: boolean;
  configurado: boolean;
  estado: string;
  comprobadoEn: number | null;
  latenciaMs: number | null;
  ultimoModelo: string | null;
  ultimoError: string | null;
}

export interface EstadoIA {
  activo: ProveedorIA;
  proveedores: EstadoProveedorIA[];
}

export interface CompletarIA {
  ok: boolean;
  texto: string | null;
  proveedor: string | null;
  modelo: string | null;
  motivos: string[];
}

export interface ProbarIA {
  ok: boolean;
  latenciaMs: number;
  modelo: string | null;
  error: string | null;
}

/* Resumen de la conexión activa para la cabecera del admin [10AA-16]. Puro:
 * deriva del último diagnóstico guardado, no prueba nada por sí mismo. */
export type ConexionIA = 'cargando' | 'sin-leer' | 'sin-clave' | 'deshabilitada' | 'sin-comprobar' | 'fallo' | 'conectada';

export interface ResumenConexionIA {
  proveedor: string | null;
  conexion: ConexionIA;
  comprobadoEn: number | null;
}

export function resumirConexionIA(estado: EstadoIA | null, error: string | null): ResumenConexionIA {
  /* Un error de recarga gana al estado anterior: no mostrar como vigente un dato que ya no se pudo releer. */
  if (error) return { proveedor: null, conexion: 'sin-leer', comprobadoEn: null };
  if (!estado) return { proveedor: null, conexion: 'cargando', comprobadoEn: null };
  const activo = estado.proveedores.find((p) => p.id === estado.activo);
  if (!activo) return { proveedor: null, conexion: 'sin-clave', comprobadoEn: null };
  const base = { proveedor: activo.nombre, comprobadoEn: activo.comprobadoEn };
  if (!activo.configurado) return { ...base, conexion: 'sin-clave' };
  if (!activo.habilitado) return { ...base, conexion: 'deshabilitada' };
  if (activo.estado === 'ok') return { ...base, conexion: 'conectada' };
  if (activo.estado === 'error') return { ...base, conexion: 'fallo' };
  return { ...base, conexion: 'sin-comprobar' };
}
