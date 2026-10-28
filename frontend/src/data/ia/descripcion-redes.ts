import type { Inmueble } from '@/domain/inmueble';
import { completarIA } from './cliente-ia';
import { fichaResumen } from './ficha-texto';

/* [28AA-4] Descripción y etiquetas para el post de redes, vía el centro de IA
 * del backend. Como `generarCopyConIA`, es puro salvo la red: el calendario
 * llama aquí y guarda el resultado con PUT, así que el texto siempre es editable
 * antes de publicar. Sin datos inventados: solo lo que dice la ficha. */

export interface DescripcionRedes {
  descripcion: string;
  tags: string[];
}

/* [28AA-4] `prompt` es lo que se envió a la IA (sistema + ficha): el calendario
 * lo guarda como `prompt_usado` junto con `modelo`, para rehacerlo o auditarlo. */
export type ResultadoDescripcion =
  | { ok: true; datos: DescripcionRedes; modelo: string; prompt: string }
  | { ok: false; motivo: string };

const MAX_DESCRIPCION = 2200;
const MAX_TAGS = 30;
const MAX_TAG_LEN = 60;

const SISTEMA = [
  'Eres un redactor inmobiliario para redes sociales. Escribes en español, tono cercano y comercial, sin inventar datos.',
  'Devuelve SOLO un objeto JSON, sin markdown ni explicaciones, con estas claves exactas:',
  '{"descripcion": string, "tags": string[]}',
  'Reglas: "descripcion" = 3-6 frases para el pie del post, con emojis moderados, y termina con un llamado a la acción.',
  '"tags" = entre 5 y 12 etiquetas en minúsculas, sin el símbolo #, sin espacios (escribe las palabras juntas).',
  'Usa solo los datos de la ficha; si un dato falta, no lo inventes.',
].join('\n');

export async function generarDescripcionRedes(
  inmueble: Inmueble,
  signal?: AbortSignal,
): Promise<ResultadoDescripcion> {
  if (signal?.aborted) return { ok: false, motivo: 'Petición cancelada.' };
  const usuario = `Ficha del inmueble:\n${fichaResumen(inmueble)}`;
  const r = await completarIA(SISTEMA, usuario, [], signal);
  if (!r.ok || !r.texto?.trim()) {
    // La IA de texto a veces devuelve vacío: se avisa y el botón reintenta
    return { ok: false, motivo: r.motivos.join(' · ') || 'La IA devolvió una respuesta vacía. Reinténtalo.' };
  }
  const datos = extraerJson(r.texto);
  if (!datos) return { ok: false, motivo: 'La IA no devolvió un JSON válido. Reinténtalo.' };
  const descripcion = aTexto(datos.descripcion, MAX_DESCRIPCION);
  const tags = normalizarTags(datos.tags);
  if (!descripcion) return { ok: false, motivo: 'La IA devolvió una descripción vacía. Reinténtalo.' };
  return {
    ok: true,
    datos: { descripcion, tags },
    modelo: `${r.proveedor ?? '?'}:${r.modelo ?? '?'}`,
    prompt: `${SISTEMA}\n\n${usuario}`,
  };
}

/* Mismas reglas que el backend (`normalizar_tags`): quita la almohadilla inicial,
 * quita TODOS los espacios, recorta a 60 caracteres y pasa a minúsculas. Así lo
 * que se muestra es lo que se guarda, y el servidor no corrige nada en silencio. */
export function normalizarTags(entrada: unknown): string[] {
  if (!Array.isArray(entrada)) return [];
  const vistos = new Set<string>();
  for (const bruto of entrada) {
    if (typeof bruto !== 'string') continue;
    const sinAlmohadilla = bruto.trim().replace(/^#+/, '');
    const tag = [...sinAlmohadilla.replace(/\s/g, '')].slice(0, MAX_TAG_LEN).join('').toLowerCase();
    if (tag) vistos.add(tag);
    if (vistos.size >= MAX_TAGS) break;
  }
  return [...vistos];
}

function aTexto(v: unknown, max: number): string {
  return typeof v === 'string' ? v.trim().slice(0, max) : '';
}

function extraerJson(texto: string): Record<string, unknown> | null {
  const directo = intentarParse(texto);
  if (directo) return directo;
  const candidatos = texto.match(/\{[\s\S]*\}/g) ?? [];
  candidatos.sort((a, b) => b.length - a.length);
  for (const c of candidatos) {
    const parsed = intentarParse(c);
    if (parsed) return parsed;
  }
  return null;
}

function intentarParse(s: string): Record<string, unknown> | null {
  try {
    const v = JSON.parse(s);
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}
