import { ESTADOS, TIPOS, type EstadoInmueble, type TipoInmueble } from '@/domain/inmueble';
import { completarIA } from './cliente-ia';

/* Adaptador de IA para ordenar fichas de inmuebles vía el centro de IA del
 * backend [199A-2] (GloryAPI + OpenCode Go con fallback; las claves viven en
 * el `.env` del servidor). Puro salvo la red: prompts y normalización quedan
 * aquí, los componentes solo llaman a `organizarConIA`. */

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

const SISTEMA = [
  'Eres un experto inmobiliario que ordena fichas de inmuebles en español.',
  'Recibes una descripción libre (a veces desordenada) y fotos del inmueble.',
  'Devuelve SOLO un objeto JSON, sin markdown ni explicaciones, con estas claves exactas:',
  '{"titulo": string, "descripcion": string, "ubicacion": string, "residencia": string,',
  ' "nombre_corto": string, "precio": number|null,',
  ' "tipo": "apartamento"|"casa"|"local"|"terreno"|"townhouse"|"", "operacion": "venta"|"alquiler"|"ambas"|"",',
  ' "precio_alquiler": number|null,',
  ' "habitaciones": number|null, "banos": number|null, "metros": number|null, "metrosTerreno": number|null,',
  ' "puestos": number|null, "estado": "disponible"}',
  'Reglas: "metros" son m² de propiedad (construidos); "metrosTerreno" son m² de parcela/terreno (null si no se indican).',
  'Reglas: "residencia" es el nombre de la residencia/conjunto/urbanización ("Residencias Los Naranjos"); "" si no se indica.',
  /* [18AA-7] Misma regla que `sugerir_nombre_corto` en el backend (canónica
   * allí; esta es la que escribe el nombre al publicar). */
  'Reglas: "nombre_corto" es cómo se cita el inmueble al hablar de él en un chat (máx 120 caracteres):',
  'tipo abreviado ("Apart." para apartamento, "Casa", "Local", "Terreno", "Townhouse") + " en " + la residencia,',
  'o la zona/ciudad si no hay residencia; sin "Edificio"/"Edif." y con "Residencias" abreviado a "Resd."; "" si dudas.',
  'Reglas: "puestos" son los puestos de estacionamiento (null si no se indican).',
  'Reglas: titulo comercial corto (máx 80 caracteres); descripcion comercial ordenada de 2 a 4 frases',
  'basada en el texto y lo visible en las fotos; precio como número sin símbolos (null si no se indica);',
  'no inventes la dirección exacta, usa solo lo que diga el texto ("" si no hay);',
  'tipo en minúsculas de la lista exacta ("apartamento","casa","local","terreno","townhouse") o "" si dudas;',
  'operacion en minúsculas ("venta","alquiler","ambas") o "" si dudas; estado SIEMPRE el literal "disponible".',
  /* [25AA-7] «ambas» = se vende y se alquila: "precio" es la venta y "precio_alquiler" el canon mensual. */
  'Reglas: si el inmueble se vende y se alquila a la vez, operacion "ambas", precio = venta y precio_alquiler = canon mensual; si no, precio_alquiler null.',
  /* El texto pegado suele traer la firma de otra inmobiliaria: nunca debe llegar a la ficha. */
  'Reglas: quita cualquier especificación de inmobiliaria o agencia: su nombre, razón social ("C.A.", "S.A."), frases de oferta ("ofrece en venta", "ofrecemos"), contactos (teléfonos, WhatsApp, "contáctanos") y firmas o logos. Describe solo el inmueble.',
].join('\n');

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
  signal?: AbortSignal;
}

/* Ordena la ficha vía `POST /api/admin/ia/completar`: el backend elige
 * proveedor (activo + fallback) y devuelve el texto del modelo; aquí solo
 * se extrae y normaliza el JSON (ante la duda, campo vacío: nunca inventar). */
export async function organizarConIA({ texto, fotos, signal }: EntradaIA): Promise<ResultadoIA> {
  if (!texto.trim() && fotos.length === 0) {
    return { ok: false, motivo: 'Escribe una descripción o suelta al menos una foto para que la IA tenga algo que ordenar.' };
  }
  if (signal?.aborted) return { ok: false, motivo: 'Petición cancelada.' };
  const r = await completarIA(
    SISTEMA,
    texto.trim() || 'Ordena la ficha de este inmueble a partir de las fotos.',
    fotos.slice(0, 4),
    signal,
  );
  if (!r.ok || !r.texto) {
    return { ok: false, motivo: r.motivos.join(' · ') || 'La IA no pudo ordenar la ficha. Reinténtalo.' };
  }
  const datos = extraerJson(r.texto);
  if (!datos) return { ok: false, motivo: 'La IA no devolvió una ficha JSON válida. Reinténtalo con más detalle.' };
  return { ok: true, ficha: normalizarFicha(datos), modelo: `${r.proveedor ?? '?'}:${r.modelo ?? '?'}` };
}
