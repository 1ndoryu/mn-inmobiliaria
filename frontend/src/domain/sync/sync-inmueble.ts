/* [13AA-12] Vocabulario y etiquetas del panel Sync: puro (sin React ni red).
 * Los valores espejan `services/sync/comparar.rs` del backend (snake_case). */

export type EstadoSyncInmueble =
  | 'igual'
  | 'local_adelante'
  | 'servidor_adelante'
  | 'ambos'
  | 'diferente'
  | 'solo_local'
  | 'calculando';

/* Quién cambió un elemento: `indeterminado` = difiere y no hay base previa. */
export type LadoSync = 'local' | 'servidor' | 'ambos' | 'indeterminado';

export interface EtiquetaSync {
  texto: string;
  clase: string;
}

/* Colores como el resto de badges de la tabla (fondo suave + texto oscuro). */
const ETIQUETAS_ESTADO: Record<EstadoSyncInmueble, EtiquetaSync> = {
  igual: { texto: 'Igual', clase: 'border-transparent bg-emerald-100 text-emerald-900' },
  local_adelante: { texto: 'Local adelante', clase: 'border-transparent bg-sky-100 text-sky-900' },
  servidor_adelante: { texto: 'Servidor adelante', clase: 'border-transparent bg-violet-100 text-violet-900' },
  ambos: { texto: 'Ambos cambiaron', clase: 'border-transparent bg-red-100 text-red-900' },
  diferente: { texto: 'Diferente (sin base)', clase: 'border-transparent bg-amber-100 text-amber-900' },
  solo_local: { texto: 'Solo local', clase: 'border-transparent bg-slate-200 text-slate-900' },
  calculando: { texto: 'Calculando…', clase: 'text-muted-foreground' },
};

/* `estimado`: la dirección sale de `updated_at` porque no hay base previa. */
export function etiquetaEstado(estado: EstadoSyncInmueble, estimado: boolean): EtiquetaSync {
  const base = ETIQUETAS_ESTADO[estado];
  return estimado ? { ...base, texto: `${base.texto} (estimado)` } : base;
}

const ETIQUETAS_LADO: Record<LadoSync, string> = {
  local: 'Local',
  servidor: 'Servidor',
  ambos: 'Ambos',
  indeterminado: 'Sin base',
};

export function etiquetaLado(lado: LadoSync): string {
  return ETIQUETAS_LADO[lado];
}

const NOMBRES_CAMPO: Record<string, string> = {
  titulo: 'Título',
  descripcion: 'Descripción',
  ubicacion: 'Ubicación',
  puestos: 'Puestos de estacionamiento',
  residencia: 'Residencia',
  precio: 'Precio',
  tipo: 'Tipo',
  operacion: 'Operación',
  habitaciones: 'Habitaciones',
  banos: 'Baños',
  metros: 'M² construidos',
  metros_terreno: 'M² terreno',
  estado: 'Estado',
  publicado: 'Publicado',
  copy_corta: 'Copy corta',
  copy_larga: 'Copy larga',
  copy_modelo: 'Copy modelo',
  receta: 'Receta de publicidad',
  extras: 'Ficha (extras)',
  precio_minimo: 'Precio mínimo',
};

/* Clave de una diferencia: nombre de campo o `foto:<origen>:<orden>` (orden desde 0). */
export function nombreClave(clave: string): string {
  const partes = clave.split(':');
  if (partes[0] === 'foto' && partes.length === 3) {
    const numero = Number(partes[2]) + 1;
    return partes[1] === 'mejorada' ? `Foto ${numero} (mejorada)` : `Foto ${numero}`;
  }
  return NOMBRES_CAMPO[clave] ?? clave;
}
