// Dominio puro de mejora de fotos: sin DOM, sin localStorage, sin React.
// Reutilizable en móvil (Capacitor) sin cambios.

export type EstadoFoto = 'pendiente' | 'procesando' | 'lista' | 'error';

/** Modo de mejora. Manual por defecto: nada se procesa sin pulsar Reintentar. */
export type ModoMejora = 'manual' | 'automatico';

/* Identidad: de qué inmueble es y en qué orden se subió (inmutable). */
export interface FotoIdentidad {
  id: string;
  inmuebleId: string;
  /** Orden de subida dentro del inmueble (0,1,2...). Inmutable. */
  orden: number;
}

/* Contenido: original intacto, resultado y estado. */
export interface FotoContenido {
  /** Copia intacta del original tal como se subió (dataURL). Nunca se muta. */
  original: string;
  /** Resultado del backend en la mejor resolución disponible (dataURL). */
  mejorada: string | null;
  estado: EstadoFoto;
}

/* Seguimiento de la cola: intentos, trabajo activo, error y auditoría. */
export interface FotoSeguimiento {
  intentos: number;
  /** Trabajo del backend que la está procesando. Permite retomar el sondeo
   * tras cerrar/recargar la pestaña en vez de quedarse colgado. */
  jobId: string | null;
  error: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface FotoMejora extends FotoIdentidad, FotoContenido, FotoSeguimiento {}

/** Campos que la cola puede escribir sobre una foto (`orden` solo lo toca la
 * purga tras el respaldo, para reparar el desplazamiento que dejó la mezcla
 * de originales y mejoradas en `inmueble.fotos`). */
export type ParcheFoto = Partial<Pick<FotoMejora, 'estado' | 'mejorada' | 'error' | 'intentos' | 'jobId' | 'orden'>>;

export interface ConfigMejora {
  /** Segundos base entre fotos. Defecto 120 para no castigar la cuenta. */
  intervaloSeg: number;
  /** Jitter porcentual 0-50 aplicado al intervalo. Defecto 30. */
  jitterPct: number;
  /** Máximo de fotos por día natural. Defecto 40. */
  maxPorDia: number;
  /** Manual = solo botón Reintentar por foto. Automático = cola sola al subir. */
  modo: ModoMejora;
  /** Plantilla de prompt enviada al worker junto a cada foto. */
  prompt: string;
}

export const PROMPT_DEFECTO = [
  'Mejora esta foto inmobiliaria a fotografia profesional:',
  'sube nitidez y resolucion, corrige luz y color de forma natural,',
  'limpia suciedad leve en paredes, telas u objetos y retira personas',
  'y distracciones menores.',
  'Haz la imagen ultra HD, mas grande, con mas definicion:',
  'maxima resolucion y detalle que puedas devolver.',
  'No cambies la estructura, geometria, muebles ni elementos fijos;',
  'no inventes nada que no exista en la foto original.',
].join(' ');

export const CONFIG_DEFECTO: ConfigMejora = {
  intervaloSeg: 120,
  jitterPct: 30,
  maxPorDia: 40,
  modo: 'manual',
  prompt: PROMPT_DEFECTO,
};

export const CLAVE_CONFIG_MEJORA = 'inmobiliaria:mejora-config:v1';
export const CLAVE_FOTOS_METADATOS = 'inmobiliaria:fotos-meta:v1';

export function nuevoId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
}

function saneaEntero(v: unknown, defecto: number, min: number, max: number): number {
  const n = typeof v === 'string' ? Number(v) : Number(v);
  if (!Number.isFinite(n)) return defecto;
  return Math.min(max, Math.max(min, Math.floor(n)));
}

/* Normaliza la configuración guardada: ante la duda, valores seguros.
 * Acepta el formato legacy `{ auto: boolean }` y lo migra a `modo`. */
export function normalizarConfig(datos: unknown): ConfigMejora {
  const d = (typeof datos === 'object' && datos !== null ? datos : {}) as Partial<ConfigMejora> & { auto?: unknown };
  const prompt = typeof d.prompt === 'string' && d.prompt.trim() ? d.prompt.trim().slice(0, 2000) : PROMPT_DEFECTO;
  const modo: ModoMejora =
    d.modo === 'automatico' || d.modo === 'manual' ? d.modo : d.auto === true ? 'automatico' : 'manual';
  return {
    intervaloSeg: saneaEntero(d.intervaloSeg, CONFIG_DEFECTO.intervaloSeg, 30, 3600),
    jitterPct: saneaEntero(d.jitterPct, CONFIG_DEFECTO.jitterPct, 0, 50),
    maxPorDia: saneaEntero(d.maxPorDia, CONFIG_DEFECTO.maxPorDia, 1, 200),
    modo,
    prompt,
  };
}

/* Calcula la espera real antes del siguiente trabajo: base + jitter aleatorio.
 * Pura y testeable; el backend es la autoridad que la aplica. */
export function proximoRetrasoMs(config: ConfigMejora, aleatorio: () => number = Math.random): number {
  const base = config.intervaloSeg * 1000;
  const jitter = base * (config.jitterPct / 100) * aleatorio();
  return Math.floor(base + jitter);
}

export function esTerminal(estado: EstadoFoto): boolean {
  return estado === 'lista';
}
