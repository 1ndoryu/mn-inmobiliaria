import { ETIQUETAS_TIPO, formatearPrecio, type Inmueble } from '@/domain/inmueble';
import type { ConfigCopy } from '@/domain/copy';
import { completarIA } from './cliente-ia';

/* Generación del Copy para redes vía el centro de IA del backend [199A-2].
 * Puro salvo la red: los componentes solo llaman a `generarCopyConIA`.
 * No confundir con `organizarConIA` (esa ordena la ficha al subir el
 * inmueble; esta escribe el texto publicable y se guarda en `inmueble.copy`). */

export interface CopyGenerado {
  corta: string;
  larga: string;
}

export type ResultadoCopy =
  | { ok: true; copy: CopyGenerado; modelo: string }
  | { ok: false; motivo: string };

function fichaResumen(i: Inmueble): string {
  const partes = [
    `Título: ${i.titulo || 'Sin título'}`,
    `Descripción: ${i.descripcion || '—'}`,
    `Ubicación: ${i.ubicacion || '—'}`,
    i.residencia ? `Residencia: ${i.residencia}` : 'Residencia: —',
    i.precio > 0 ? `Precio: ${formatearPrecio(i.precio)}` : 'Precio: a consultar',
    `Tipo: ${ETIQUETAS_TIPO[i.tipo]} · Operación: ${i.operacion} · Estado: ${i.estado}`,
  ];
  const extras: string[] = [];
  if (i.habitaciones > 0) extras.push(`${i.habitaciones} hab`);
  if (i.banos > 0) extras.push(`${i.banos} baños`);
  if (i.metros > 0) extras.push(`${i.metros} m²`);
  if (i.metrosTerreno > 0) extras.push(`${i.metrosTerreno} m² terreno`);
  if (i.puestos > 0) extras.push(`${i.puestos} puestos`);
  if (extras.length > 0) partes.push(`Detalles: ${extras.join(', ')}`);
  return partes.join('\n');
}

function aTexto(v: unknown, max: number): string {
  return typeof v === 'string' ? v.trim().slice(0, max) : '';
}

interface EntradaCopy {
  inmueble: Inmueble;
  config: ConfigCopy;
  signal?: AbortSignal;
}

export async function generarCopyConIA({ inmueble, config, signal }: EntradaCopy): Promise<ResultadoCopy> {
  const sistema = [
    'Eres un redactor inmobiliario para redes sociales. Escribes en español, tono cercano y comercial, sin inventar datos.',
    'Devuelve SOLO un objeto JSON, sin markdown ni explicaciones, con estas claves exactas:',
    '{"corta": string, "larga": string}',
    'Reglas: "corta" = 1-2 líneas con gancho para ir SOBRE la imagen (máx 140 caracteres).',
    `"larga" = 3-6 frases para el pie del post, con emojis moderados, y termina SIEMPRE con este llamado a la acción: "${config.cta}"`,
    'Usa solo los datos de la ficha; si un dato falta, no lo inventes.',
  ].join('\n');
  const usuario = `${config.prompt}\n\nFicha del inmueble:\n${fichaResumen(inmueble)}`;
  if (signal?.aborted) return { ok: false, motivo: 'Petición cancelada.' };
  const r = await completarIA(sistema, usuario, [], signal);
  if (!r.ok || !r.texto?.trim()) {
    return { ok: false, motivo: r.motivos.join(' · ') || 'La IA devolvió una respuesta vacía. Reinténtalo.' };
  }
  const datos = extraerJson(r.texto);
  if (!datos) return { ok: false, motivo: 'La IA no devolvió un JSON válido. Reinténtalo.' };
  const corta = aTexto(datos.corta, 280);
  const larga = aTexto(datos.larga, 2000);
  if (!corta || !larga) return { ok: false, motivo: 'La IA devolvió un copy incompleto (falta corta o larga). Reinténtalo.' };
  return { ok: true, copy: { corta, larga }, modelo: `${r.proveedor ?? '?'}:${r.modelo ?? '?'}` };
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

function intentarParse(texto: string): Record<string, unknown> | null {
  try {
    const v: unknown = JSON.parse(texto.trim().replace(/^```(?:json)?/i, '').replace(/```$/, '').trim());
    return typeof v === 'object' && v !== null ? (v as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}
