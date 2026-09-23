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
