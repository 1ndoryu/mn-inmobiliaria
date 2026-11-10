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
  /* [30AA-1] Prompt de las FOTOS VARIANTE (v2): se le pasa a la imagen ya
   * mejorada para que la repita desde otro ángulo sin cambiar la realidad.
   * Es independiente de `prompt` para poder ajustarlo sin rehacer las
   * mejoradas normales. */
  promptVariante: string;
}

/* [15AA-11] Texto por defecto retirado: el "corrige luz y color de forma
 * natural" le daba pie a Gemini a tocar el balance de blancos y devolvia las
 * fotos con un velo amarillento. */
const PROMPT_ANTERIOR =
  'Mejora esta foto inmobiliaria a fotografia profesional: sube nitidez y resolucion, corrige luz y color de forma natural, limpia suciedad leve en paredes, telas u objetos y retira personas y distracciones menores. Haz la imagen ultra HD, mas grande, con mas definicion: maxima resolucion y detalle que puedas devolver. No cambies la estructura, geometria, muebles ni elementos fijos; no inventes nada que no exista en la foto original.';

/* [28AF-1] Retirado: pedia CONSERVAR el color del original y las fotos seguian
 * saliendo amarillentas. El actual pide quitar la dominante y blanquear. */
const PROMPT_COLOR_CONSERVADO_SIN_ANGULO = [
  'Mejora esta foto inmobiliaria a fotografia profesional:',
  'sube nitidez y resolucion, arregla la exposicion (sombras y altas luces)',
  'sin tocar el color, limpia suciedad leve en paredes, telas u objetos y',
  'retira personas y distracciones menores.',
  'Haz la imagen ultra HD, mas grande, con mas definicion:',
  'maxima resolucion y detalle que puedas devolver.',
  'CONSERVA LOS COLORES DEL ORIGINAL: no cambies el balance de blancos ni la',
  'temperatura de color; no apliques filtros ni tonos calidos, ambar, dorados',
  'o amarillentos, ni vinetas ni degradados. Los blancos y los grises siguen',
  'siendo neutros y las paredes, del mismo color que en la original.',
  'No cambies la estructura, geometria, muebles ni elementos fijos;',
  'no inventes nada que no exista en la foto original.',
].join(' ');

/* [15AA-11] Base del prompt actual: quita la dominante amarilla y deja los
 * blancos neutros, sin filtros ni tonos calidos. Mismo texto (a mano) en
 * `scripts/datos/inmueble/mejorar.mjs`, que no puede importar TypeScript. */
const PROMPT_SIN_ANGULO = [
  'Mejora esta foto inmobiliaria a fotografia profesional:',
  'sube nitidez y resolucion, arregla la exposicion (sombras y altas luces)',
  'sin tocar la forma, limpia suciedad leve en paredes, telas u objetos y',
  'retira personas y distracciones menores.',
  'Haz la imagen ultra HD, mas grande, con mas definicion:',
  'maxima resolucion y detalle que puedas devolver.',
  'QUITA todo tono amarillento, ambar o calido de la imagen y lleva el balance',
  'de blancos a un blanco neutro: los blancos y los grises deben verse blancos',
  'y grises neutros, sin velo ni dominante de color. No apliques filtros calidos,',
  'vinetas ni degradados. Conserva los colores propios de los objetos y de las',
  'paredes: solo se retira la dominante de la luz.',
  'No cambies la estructura, geometria, muebles ni elementos fijos;',
  'no inventes nada que no exista en la foto original.',
].join(' ');

/* [16AA-5] Pedido de la usuaria: muchas fotos se suben con la camara torcida
 * y la mejora no enderezaba la toma. Enderezar la perspectiva mueve lineas
 * respecto al original, asi que se pide aqui y no en la parte que prohibe
 * tocar la geometria. */
const CORRECCION_ANGULO =
  'Corrige el angulo si la camara no esta muy bien alineada: endereza la perspectiva (verticales y horizontales) sin deformar la estancia.';

export const PROMPT_DEFECTO = `${PROMPT_SIN_ANGULO} ${CORRECCION_ANGULO}`;

/* [30AA-1] Variante de una foto YA mejorada, para publicar el mismo inmueble
 * en otra cuenta de Facebook. El encargo es delicado: la foto tiene que
 * parecer OTRA toma (ángulo, altura, encuadre) sin que cambie nada de lo que
 * se ve (estancias, muebles, número de vanos, materiales, colores). Los
 * mismos límites que la mejora normal más el cambio de punto de vista.
 *
 * [30AA-2] El texto anterior pedía «unos pasos a un lado» y el modelo devolvía
 * la MISMA toma retocada: medido con SSIM sobre 36 pares, 8 salieron ≥ 85 % de
 * parecido y en todas ellas el desplazamiento de encuadre medido era 0. Ahora
 * el encuadre se pide de forma explícita y no negociable, con el cambio mínimo
 * en grados y la prohibición de repetir el original. */
export const PROMPT_VARIANTE = [
  'Rehaz esta fotografia inmobiliaria como si te hubieras movido de sitio:',
  'el ENCUADRE TIENE QUE SER DISTINTO al de la foto original. Situate en otra',
  'esquina de la estancia, gira la camara al menos 30 grados respecto a la toma',
  'original y sube o baja la altura del objetivo entre 40 y 60 centimetros.',
  'Se tienen que ver paredes u objetos que en el original quedaban fuera del',
  'borde, y dejar de verse parte de lo que el original encuadraba. Repetir el',
  'mismo encuadre con otro recorte NO vale: la composicion debe cambiar.',
  'Lo que NO cambia: la estancia, los muebles, los materiales, los colores y',
  'TODOS los elementos deben ser exactamente los mismos; no anadas, quites,',
  'muevas ni sustituyas nada, y no cambies el numero de ventanas, puertas,',
  'habitaciones ni su tamano. Corrige la perspectiva (verticales y',
  'horizontales). Conserva la luz natural y el balance de blancos neutro, sin',
  'tonos amarillos ni filtros; manten la nitidez y la resolucion originales.',
  'El resultado debe parecer una foto nueva hecha por otra persona en el mismo',
  'momento, nunca una imagen retocada ni una vista distinta del inmueble.',
].join(' ');

/* Prompts por defecto ya retirados: quien los tenga guardados tal cual recibe
 * el actual. Un prompt escrito a mano por la usuaria se respeta siempre. */
const PROMPTS_RETIRADOS = [
  PROMPT_ANTERIOR,
  PROMPT_COLOR_CONSERVADO_SIN_ANGULO,
  `${PROMPT_COLOR_CONSERVADO_SIN_ANGULO} ${CORRECCION_ANGULO}`,
];

export const CONFIG_DEFECTO: ConfigMejora = {
  intervaloSeg: 120,
  jitterPct: 30,
  maxPorDia: 40,
  modo: 'manual',
  prompt: PROMPT_DEFECTO,
  promptVariante: PROMPT_VARIANTE,
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
  const guardado = typeof d.prompt === 'string' ? d.prompt.trim() : '';
  const prompt = !guardado || PROMPTS_RETIRADOS.includes(guardado) ? PROMPT_DEFECTO : guardado.slice(0, 2000);
  const varianteGuardada = typeof d.promptVariante === 'string' ? d.promptVariante.trim() : '';
  const promptVariante = varianteGuardada ? varianteGuardada.slice(0, 2000) : PROMPT_VARIANTE;
  const modo: ModoMejora =
    d.modo === 'automatico' || d.modo === 'manual' ? d.modo : d.auto === true ? 'automatico' : 'manual';
  return {
    intervaloSeg: saneaEntero(d.intervaloSeg, CONFIG_DEFECTO.intervaloSeg, 30, 3600),
    jitterPct: saneaEntero(d.jitterPct, CONFIG_DEFECTO.jitterPct, 0, 50),
    maxPorDia: saneaEntero(d.maxPorDia, CONFIG_DEFECTO.maxPorDia, 1, 200),
    modo,
    prompt,
    promptVariante,
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
