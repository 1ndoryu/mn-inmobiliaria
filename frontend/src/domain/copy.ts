// Dominio puro del Copy para redes: sin DOM, sin localStorage, sin React.
// El copy son 2 textos por inmueble (corta para la imagen, larga para el pie)
// que se generan una vez con IA y se guardan en `inmueble.copy`.

export interface ConfigCopy {
  /** Generar el copy automáticamente al guardar un inmueble nuevo. */
  generarAlCrear: boolean;
  /** Plantilla del prompt (se combina con la ficha del inmueble). */
  prompt: string;
  /** Llamado a la acción obligatorio al final de la descripción larga. */
  cta: string;
}

export const CTA_DEFECTO = 'Contáctanos para más información.';

export const PROMPT_COPY_DEFECTO = [
  'Genera 2 descripciones para este inmueble: una corta que iría en la imagen,',
  'y otra para la descripción de la imagen (el texto que iría debajo en la descripción de la red social).',
].join(' ');

export const CONFIG_COPY_DEFECTO: ConfigCopy = {
  generarAlCrear: true,
  prompt: PROMPT_COPY_DEFECTO,
  cta: CTA_DEFECTO,
};

export const CLAVE_CONFIG_COPY = 'inmobiliaria:copy-config:v1';

function saneaLinea(v: unknown, defecto: string, max: number): string {
  const t = typeof v === 'string' ? v.trim() : '';
  if (!t) return defecto;
  return t.slice(0, max);
}

/* Normaliza la configuración guardada: ante la duda, valores seguros. */
export function normalizarConfigCopy(datos: unknown): ConfigCopy {
  const d = (typeof datos === 'object' && datos !== null ? datos : {}) as Partial<ConfigCopy>;
  return {
    generarAlCrear: d.generarAlCrear !== false,
    prompt: saneaLinea(d.prompt, PROMPT_COPY_DEFECTO, 2000),
    cta: saneaLinea(d.cta, CTA_DEFECTO, 200),
  };
}
