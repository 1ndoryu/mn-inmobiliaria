import { ESTADOS, TIPOS, type EstadoInmueble, type TipoInmueble } from '@/domain/inmueble';
import { completarFichaIA } from './cliente-ia';

/* Adaptador de IA para ordenar fichas de inmuebles. [28AE-2] El prompt vive en
 * el backend (`config/ia.rs::SISTEMA_FICHA`, el mismo para panel, completar al
 * publicar y CLI) y el original se respalda allí antes de llamar a la IA
 * (OpenCode Go, DeepSeek Flash). Puro salvo la red: aquí solo se normaliza la
 * respuesta; los componentes llaman a `organizarConIA`. */

/* Texto comercial de la ficha. */
export interface FichaTexto {
  titulo: string;
  descripcion: string;
  ubicacion: string;
  residencia: string;
  /* [18AA-7] Cómo se cita el inmueble en el chat del Marketplace. */
  nombre_corto: string;
}

/* Números de la ficha (null = no indicado). */
export interface FichaMedidas {
  precio: number | null;
  habitaciones: number | null;
  banos: number | null;
  metros: number | null;
  metrosTerreno: number | null;
  puestos: number | null;
}

/* Clasificación de la ficha. */
export interface FichaClase {
  tipo: '' | TipoInmueble;
  operacion: '' | 'venta' | 'alquiler' | 'ambas';
  estado: EstadoInmueble;
}

/* [25AA-7] Canon mensual cuando la operación es «ambas» (null si no se indica). */
export interface FichaIA extends FichaTexto, FichaMedidas, FichaClase {
  precio_alquiler: number | null;
}

export type ResultadoIA =
  | { ok: true; ficha: FichaIA; modelo: string }
  | { ok: false; motivo: string };

function aNumero(v: unknown): number | null {
  const n = typeof v === 'string' ? Number(v.replace(/[^\d.,-]/g, '').replace(',', '.')) : Number(v);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

function aTexto(v: unknown): string {
  return typeof v === 'string' ? v.trim().slice(0, 2000) : '';
}

/* Normaliza lo que devuelva el modelo: ante la duda, campo vacío (nunca inventar). */
function normalizarFicha(datos: Record<string, unknown>): FichaIA {
  const tipo = aTexto(datos.tipo).toLowerCase();
  const operacion = aTexto(datos.operacion).toLowerCase();
  return {
    titulo: aTexto(datos.titulo).slice(0, 80),
    descripcion: aTexto(datos.descripcion),
    ubicacion: aTexto(datos.ubicacion).slice(0, 200),
    residencia: aTexto(datos.residencia).slice(0, 200),
    /* [18AA-7] El backend topa el campo a 120; se recorta aquí para no perder el PUT. */
    nombre_corto: aTexto(datos.nombre_corto).slice(0, 120),
    precio: aNumero(datos.precio),
    precio_alquiler: aNumero(datos.precio_alquiler),
    tipo: (TIPOS as readonly string[]).includes(tipo) ? (tipo as TipoInmueble) : '',
    operacion: operacion === 'venta' || operacion === 'alquiler' || operacion === 'ambas' ? operacion : '',
    habitaciones: aNumero(datos.habitaciones),
    banos: aNumero(datos.banos),
    metros: aNumero(datos.metros),
    metrosTerreno: aNumero(datos.metrosTerreno),
    puestos: aNumero(datos.puestos),
    estado: (ESTADOS as readonly string[]).includes(aTexto(datos.estado)) ? (aTexto(datos.estado) as EstadoInmueble) : 'disponible',
  };
}

function extraerJson(texto: string): Record<string, unknown> | null {
  const directo = intentarParse(texto);
  if (directo) return directo;
  // Algunos modelos envuelven el JSON en texto o markdown: buscar el mayor bloque {...}.
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

interface EntradaIA {
  texto: string;
  fotos: string[];
  /* Inmueble al que pertenece la descripción, si ya existe (liga el respaldo). */
  inmuebleId?: string;
  origen?: 'panel' | 'publicar';
  signal?: AbortSignal;
}

/* Ordena la ficha vía `POST /api/admin/ia/ficha`: el backend guarda el
 * original, aplica el prompt y devuelve el texto del modelo; aquí solo se
 * extrae y normaliza el JSON (ante la duda, campo vacío: nunca inventar). */
export async function organizarConIA({ texto, fotos, inmuebleId, origen, signal }: EntradaIA): Promise<ResultadoIA> {
  if (!texto.trim() && fotos.length === 0) {
    return { ok: false, motivo: 'Escribe una descripción o suelta al menos una foto para que la IA tenga algo que ordenar.' };
  }
  if (signal?.aborted) return { ok: false, motivo: 'Petición cancelada.' };
  const r = await completarFichaIA(texto.trim(), fotos.slice(0, 4), { inmuebleId, origen, signal });
  if (!r.ok || !r.texto) {
    return { ok: false, motivo: r.motivos.join(' · ') || 'La IA no pudo ordenar la ficha. Reinténtalo.' };
  }
  const datos = extraerJson(r.texto);
  if (!datos) return { ok: false, motivo: 'La IA no devolvió una ficha JSON válida. Reinténtalo con más detalle.' };
  return { ok: true, ficha: normalizarFicha(datos), modelo: `${r.proveedor ?? '?'}:${r.modelo ?? '?'}` };
}
