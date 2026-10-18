// Utilidades compartidas por los subcomandos de inmueble (argumentos, datos y listados).
import { readFileSync } from 'node:fs';
import { api, nucleo } from '../../dev/lib-api.mjs';

export const REQUERIDAS = ['PROD_BASE', 'PROD_EMAIL', 'PROD_PASSWORD', 'LOCAL_EMAIL', 'LOCAL_PASSWORD'];

export function args() {
  const cmd = process.argv[2];
  const o = {};
  const resto = process.argv.slice(3);
  for (let i = 0; i < resto.length; i++) {
    const a = resto[i];
    if (!a.startsWith('--')) continue;
    const k = a.slice(2);
    const v = resto[i + 1] !== undefined && !resto[i + 1].startsWith('--') ? resto[++i] : true;
    o[k] = v;
  }
  return { cmd, o };
}

export function leerDatos(v) {
  if (v === undefined) {
    console.error('FALLO preflight: falta --datos (JSON o @ruta.json).');
    process.exit(2);
  }
  const texto = String(v).startsWith('@') ? readFileSync(String(v).slice(1), 'utf8') : String(v);
  try {
    return JSON.parse(texto);
  } catch {
    console.error('FALLO preflight: --datos no es JSON válido.');
    process.exit(2);
  }
}

export async function listar(base, token) {
  return (await api(base, '/api/admin/inmuebles?page=1&per_page=200', { token })).items;
}

export function porTitulo(items, titulo) {
  const t = String(titulo ?? '').trim().toLowerCase();
  return items.find((i) => String(i.titulo ?? '').trim().toLowerCase() === t) ?? null;
}

export function diffNucleo(a, b) {
  const na = nucleo(a);
  const nb = nucleo(b);
  return Object.keys(na).filter((k) => JSON.stringify(na[k]) !== JSON.stringify(nb[k]));
}
