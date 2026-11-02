// [28AE-2] La descripción pasa SIEMPRE por la IA (OpenCode Go, DeepSeek Flash) con el prompt único
// del backend (`POST /api/admin/ia/ficha`), que guarda el original en `descripciones_originales`
// (privada) antes de llamar a la IA. Aquí solo se envía el texto y se aplica la respuesta.
import { api } from '../../dev/lib-api.mjs';

const TIMEOUT_IA_MS = 150000; // el backend espera hasta 120 s al modelo

/* Extrae el objeto JSON de la respuesta (el modelo a veces lo envuelve en markdown o texto). */
export function extraerJson(texto) {
  const intento = (t) => {
    try {
      const v = JSON.parse(t.trim().replace(/^```(?:json)?/i, '').replace(/```$/, '').trim());
      return v && typeof v === 'object' && !Array.isArray(v) ? v : null;
    } catch {
      return null;
    }
  };
  const directo = intento(texto);
  if (directo) return directo;
  const candidatos = (texto.match(/\{[\s\S]*\}/g) ?? []).sort((a, b) => b.length - a.length);
  for (const c of candidatos) {
    const v = intento(c);
    if (v) return v;
  }
  return null;
}

const texto = (v) => (typeof v === 'string' ? v.trim() : '');

/* Devuelve `datos` con la descripción ya ordenada por la IA. Solo rellena residencia y
 * ubicación si el usuario no las dio; titulo, precio y clasificación siempre mandan los suyos. */
export async function ordenarDescripcion(datos, { base, token, dry, sinIa }) {
  const original = texto(datos.descripcion);
  if (!original) return datos;
  if (dry) {
    console.log(`[dry-run] la descripción (${original.length} car.) ${sinIa ? 'solo se respaldaría' : 'se respaldaría y pasaría por la IA'}`);
    return datos;
  }
  const r = await api(base, '/api/admin/ia/ficha', {
    metodo: 'POST', token, timeoutMs: TIMEOUT_IA_MS,
    json: { texto: original, fotos: [], origen: 'cli', solo_respaldo: !!sinIa },
  });
  console.log(`OK respaldo privado de la descripción original (${r.respaldo_id})`);
  if (sinIa) return datos;
  if (!r.ok || !r.texto) {
    console.error(`FALLO IA: ${(r.motivos ?? []).join(' · ') || 'sin respuesta'}. El original quedó respaldado; reintenta o usa --sin-ia.`);
    process.exit(3);
  }
  const ficha = extraerJson(r.texto);
  const descripcion = texto(ficha?.descripcion);
  if (!descripcion) {
    console.error('FALLO IA: la respuesta no trae una ficha JSON con descripcion. Reintenta o usa --sin-ia.');
    process.exit(3);
  }
  console.log(`OK descripción ordenada por ${r.proveedor}:${r.modelo}`);
  return {
    ...datos,
    descripcion,
    residencia: texto(datos.residencia) || texto(ficha.residencia),
    ubicacion: texto(datos.ubicacion) || texto(ficha.ubicacion),
  };
}

/* Respaldo del original en OTRA base (prod) sin llamar a la IA. Si ese backend aún no tiene la
 * ruta (404) se avisa y se sigue: la IA no corrió allí, el respaldo que cuenta es el local. */
export async function respaldarOriginal(base, token, original, quien) {
  if (!texto(original)) return;
  try {
    const r = await api(base, '/api/admin/ia/ficha', {
      metodo: 'POST', token, json: { texto: original, fotos: [], origen: 'cli', solo_respaldo: true },
    });
    console.log(`OK respaldo privado en ${quien} (${r.respaldo_id})`);
  } catch (e) {
    console.warn(`AVISO ${quien}: no se pudo respaldar el original (${e.message}); sigue el local.`);
  }
}
