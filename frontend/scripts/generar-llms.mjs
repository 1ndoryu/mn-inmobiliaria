#!/usr/bin/env node

/* Genera public/llms.txt (protocolo llmstxt.org) desde la API pública del
 * backend: catálogo completo con descripciones para que modelos y
 * herramientas lean el sitio sin navegar el HTML. Se ejecuta en `prebuild`;
 * si el backend no responde, avisa y conserva el archivo anterior para no
 * romper el build. */

import { writeFileSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DESTINO = path.join(__dirname, '..', 'public', 'llms.txt');
const API = process.env.LLMS_API_URL ?? 'http://127.0.0.1:3000/api/public/inmuebles';

const linea = (v) => (v === null || v === undefined || v === '' ? '—' : String(v));

try {
  const respuesta = await fetch(API);
  if (!respuesta.ok) throw new Error(`API devolvió ${respuesta.status}.`);
  const cuerpo = await respuesta.json();
  const items = Array.isArray(cuerpo) ? cuerpo : (cuerpo.items ?? cuerpo.datos ?? cuerpo.data ?? []);

  const bloques = items.map((it) => {
    const specs = [
      `Tipo: ${linea(it.tipo)}`,
      `Operación: ${linea(it.operacion)}`,
      `Precio: ${linea(it.precio)}`,
      `Ubicación: ${linea(it.ubicacion)}`,
      it.residencia ? `Residencia: ${it.residencia}` : null,
      `Habitaciones: ${linea(it.habitaciones)}`,
      `Baños: ${linea(it.banos)}`,
      it.puestos ? `Puestos: ${it.puestos}` : null,
      it.metros ? `Construcción: ${it.metros} m²` : null,
      it.metros_terreno ? `Terreno: ${it.metros_terreno} m²` : null,
    ].filter(Boolean).join(' · ');
    return `## ${it.titulo ?? it.id}\n\n${specs}\n\n${linea(it.descripcion)}`;
  });

  const contenido = [
    '# MN Inmobiliaria',
    '',
    '> Compra, venta y alquiler de inmuebles en Puerto Ordaz, Venezuela.',
    '> Publica tu inmueble o contacta por la web.',
    '',
    ...bloques,
  ].join('\n');

  writeFileSync(DESTINO, contenido, 'utf8');
  console.log(`[llms] ${items.length} inmuebles → public/llms.txt`);
} catch (e) {
  console.warn(`[llms] Sin backend (${e instanceof Error ? e.message : e}); se conserva ${existsSync(DESTINO) ? 'el archivo anterior' : 'nada (no existe aún)'}.`);
}
