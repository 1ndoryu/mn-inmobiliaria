// Subcomando `verificar`: compara prod y local (metadatos, fotos y bytes). Solo lectura.
import { createHash } from 'node:crypto';
import { api, leerEnv, login, nucleo, ENV_DEFECTO } from '../../dev/lib-api.mjs';
import { listar, REQUERIDAS } from './comun.mjs';

/* [08AA-35] F1: detector permanente prod↔local (solo lectura: login + GETs).
 * P1 metadatos: presencia por slug, núcleo canónico campo a campo, ficha útil
 * ({extras, precio_minimo}; inmueble_id se excluye: difiere por diseño),
 * publicado, set (origen,orden) + aserción de duplicados (sin UNIQUE en BD).
 * P2 bytes: sha256 de cada par (origen,orden) con pool acotado; las mejoradas
 * van aparte (el servidor re-codifica a JPG: mismo visual, distinto sha).
 * Exit 0 limpio | 1 con diferencias | 2 preflight/auth. Cero escrituras. */
const CAMPOS_VERIFICAR = [...Object.keys(nucleo({})), 'publicado'];

function canon(v) {
  if (v === null || v === undefined) return '';
  if (Array.isArray(v)) return v.map(canon);
  if (typeof v === 'object') return Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])]));
  if (typeof v === 'number') return String(v);
  if (typeof v === 'string') return v.trim();
  return v;
}

const shaFoto = (buf) => createHash('sha256').update(buf).digest('hex');
const relUrl = (url, base) => (url.startsWith('http') ? url.slice(base.length) : url);

async function pool(tareas, n, fn) {
  const res = new Array(tareas.length);
  let i = 0;
  await Promise.all(Array.from({ length: n }, async () => {
    while (i < tareas.length) {
      const j = i++;
      try { res[j] = await fn(tareas[j]); } catch (e) { res[j] = { __error: e.message }; }
    }
  }));
  return res;
}

export async function cmdVerificar(o) {
  const soloSlug = o.slug ?? null;
  const sinBytes = o['sin-bytes'] === true || o['sin-bytes'] === '';
  const par = Math.max(1, Number(o.par ?? 6) || 6);
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = (cfg.PROD_BASE ?? 'https://mn-inmobiliaria.com').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  const jL = (await listar(LOCAL, tL)).filter((x) => !soloSlug || x.slug === soloSlug);
  const jP = (await listar(PROD, tP)).filter((x) => !soloSlug || x.slug === soloSlug);
  const mapL = new Map(jL.map((x) => [x.slug, x]));
  const mapP = new Map(jP.map((x) => [x.slug, x]));
  const nf = (a) => a.reduce((n, x) => n + (x.fotos?.length ?? 0), 0);
  console.log(`verificar: prod=${jP.length}/${nf(jP)} local=${jL.length}/${nf(jL)}${soloSlug ? ` slug=${soloSlug}` : ''} bytes=${sinBytes ? 'no' : `sí(par=${par})`}`);
  let difs = 0;
  const marca = (m) => { console.log(m); difs++; };
  for (const s of new Set([...mapP.keys(), ...mapL.keys()])) {
    const p = mapP.get(s);
    const l = mapL.get(s);
    if (!p) { marca(`FALTA-PROD slug=${s}`); continue; }
    if (!l) { marca(`FALTA-LOCAL slug=${s}`); continue; }
    for (const c of CAMPOS_VERIFICAR) {
      if (JSON.stringify(canon(p[c])) !== JSON.stringify(canon(l[c]))) marca(`DIFIERE slug=${s} campo=${c}`);
    }
    const fichaP = await api(PROD, `/api/admin/inmuebles/${p.id}/ficha`, { token: tP }).catch(() => null);
    const fichaL = await api(LOCAL, `/api/admin/inmuebles/${l.id}/ficha`, { token: tL }).catch(() => null);
    const util = (f) => (f ? { extras: f.extras ?? {}, precio_minimo: f.precio_minimo ?? null } : null);
    if (JSON.stringify(canon(util(fichaP))) !== JSON.stringify(canon(util(fichaL)))) marca(`DIFIERE slug=${s} campo=ficha`);
    const setP = new Set((p.fotos ?? []).map((f) => `${f.origen}:${f.orden}`));
    const setL = new Set((l.fotos ?? []).map((f) => `${f.origen}:${f.orden}`));
    for (const k of setP) if (!setL.has(k)) marca(`FOTO-FALTA-LOCAL slug=${s} ${k}`);
    for (const k of setL) if (!setP.has(k)) marca(`FOTO-FALTA-PROD slug=${s} ${k}`);
    for (const [lado, arr] of [['prod', p.fotos ?? []], ['local', l.fotos ?? []]]) {
      const vistos = new Set();
      for (const f of arr) {
        const k = `${f.origen}:${f.orden}`;
        if (vistos.has(k)) marca(`DUPLICADO-${lado.toUpperCase()} slug=${s} ${k} id=${f.id}`);
        vistos.add(k);
      }
    }
  }
  if (!sinBytes) {
    const pares = [];
    for (const s of [...mapP.keys()].filter((s) => mapL.has(s))) {
      const idxL = new Map((mapL.get(s).fotos ?? []).map((f) => [`${f.origen}:${f.orden}`, f]));
      for (const f of mapP.get(s).fotos ?? []) {
        const g = idxL.get(`${f.origen}:${f.orden}`);
        if (g) pares.push({ slug: s, k: `${f.origen}:${f.orden}`, up: relUrl(f.url, PROD), ul: relUrl(g.url, LOCAL) });
      }
    }
    let okO = 0, malO = 0, okM = 0, malM = 0, errD = 0;
    await pool(pares, par, async (q) => {
      const bp = await api(PROD, q.up, { token: tP, bytes: true }).catch(() => null);
      const bl = await api(LOCAL, q.ul, { token: tL, bytes: true }).catch(() => null);
      if (!bp || !bl) { errD++; marca(`ERROR-DESCARGA slug=${q.slug} ${q.k}`); return; }
      const esOrig = q.k.startsWith('original:');
      if (shaFoto(bp) === shaFoto(bl)) { if (esOrig) okO++; else okM++; return; }
      if (esOrig) malO++; else malM++;
      marca(`BYTES-DISTINTOS slug=${q.slug} ${q.k} prod=${bp.length}B local=${bl.length}B`);
    });
    console.log(`bytes: ${pares.length} pares original=${okO}ok/${malO}mal mejorada=${okM}ok/${malM}mal errores=${errD}`);
  }
  console.log(difs ? `verificar: ${difs} diferencias (exit 1), cero escrituras` : 'verificar: LIMPIO (exit 0), cero escrituras');
  if (difs) process.exit(1);
}
