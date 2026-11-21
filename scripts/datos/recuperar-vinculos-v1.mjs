#!/usr/bin/env node
/* [GW-14] Recupera en la 89 los vínculos con los avisos de Marketplace que se
 * perdieron al borrar la cuenta de Facebook «Maye» (v1). La fuente es
 * PRODUCCIÓN, que conserva los 20 `marketplace_id`; aquí solo se lee de allí.
 *
 * Por qué por API y no con un INSERT a mano: vincular recalcula el espejo
 * `inmuebles.marketplace_id` (lo único que lee el borrador de glory-web) y
 * valida el aviso único por cuenta. Un INSERT dejaría el espejo desfasado.
 *
 * Reglas:
 *  - Empareja por SLUG (los UUID difieren entre bases), nunca por título.
 *  - Solo los slugs que existen en la 89: los 6 que solo viven allí se omiten.
 *  - No toca los vínculos actuales: si prod trae un aviso que aquí ya está
 *    reclamado, se salta y se informa.
 *  - `--dry-run` es el modo por defecto.
 *
 * Uso: node scripts/datos/recuperar-vinculos-v1.mjs [--aplicar] [--cuenta "Maye"] [--version 1]
 */
import { api, leerEnv, login, ENV_DEFECTO } from '../dev/lib-api.mjs';

const args = process.argv.slice(2);
const opt = (n, d) => {
  const i = args.indexOf(n);
  return i >= 0 && args[i + 1] && !args[i + 1].startsWith('--') ? args[i + 1] : d;
};
const aplicar = args.includes('--aplicar');
const nombreCuenta = opt('--cuenta', 'Maye');
const versionVinculo = Number(opt('--version', '1'));

const env = leerEnv(ENV_DEFECTO, ['PROD_BASE', 'PROD_EMAIL', 'PROD_PASSWORD', 'LOCAL_API', 'LOCAL_EMAIL', 'LOCAL_PASSWORD']);

/* Volcado completo: el tope de `per_page` es 100 y hoy hay 20-26 fichas. */
const LOTE = 100;

async function todas(base, token) {
  const salida = [];
  for (let page = 1; page < 50; page += 1) {
    const r = await api(base, `/api/admin/inmuebles?page=${page}&per_page=${LOTE}`, { token });
    salida.push(...r.items);
    if (salida.length >= r.total || r.items.length === 0) break;
  }
  return salida;
}

const tokProd = await login(env.PROD_BASE, env.PROD_EMAIL, env.PROD_PASSWORD, 'prod', ENV_DEFECTO);
const tokLocal = await login(env.LOCAL_API, env.LOCAL_EMAIL, env.LOCAL_PASSWORD, 'local (89)', ENV_DEFECTO);

const prod = await todas(env.PROD_BASE, tokProd);
const local = await todas(env.LOCAL_API, tokLocal);

/* slug → aviso de prod (solo los que tienen aviso). */
const avisoProd = new Map(prod.filter((i) => i.marketplace_id).map((i) => [i.slug, String(i.marketplace_id)]));
const porSlug = new Map(local.map((i) => [i.slug, i]));

const cuentas = await api(env.LOCAL_API, '/api/admin/facebook', { token: tokLocal });
/* Avisos ya reclamados en la 89: un aviso solo puede pertenecer a UNA ficha
 * dentro de la misma cuenta (índice único `idx_inmueble_facebook_aviso`). Se
 * relee de la API, no del espejo: el espejo es una copia. */
const nombreCuentaPorId = new Map(cuentas.map((c) => [c.id, c.nombre]));
const reclamados = new Map();
for (const inm of local) {
  const v = await api(env.LOCAL_API, `/api/admin/inmuebles/${inm.id}/facebook`, { token: tokLocal });
  for (const vin of v) {
    if (vin.marketplace_id) {
      reclamados.set(String(vin.marketplace_id), {
        cuenta: nombreCuentaPorId.get(vin.facebook_id) ?? vin.facebook_id,
        slug: inm.slug,
      });
    }
  }
}

const plan = [];
const omitidos = [];
for (const [slug, aviso] of avisoProd) {
  const inm = porSlug.get(slug);
  if (!inm) {
    omitidos.push({ slug, aviso, motivo: 'no existe en la 89' });
    continue;
  }
  if (inm.marketplace_id === aviso) {
    omitidos.push({ slug, aviso, motivo: 'ya vinculado' });
    continue;
  }
  const dup = reclamados.get(aviso);
  if (dup && dup.slug !== slug) {
    omitidos.push({ slug, aviso, motivo: `aviso ya reclamado por ${dup.slug} (cuenta ${dup.cuenta})` });
    continue;
  }
  plan.push({ slug, aviso, inmuebleId: inm.id });
}

console.log(`prod: ${prod.length} inmuebles, ${avisoProd.size} con aviso`);
console.log(`89:   ${local.length} inmuebles, ${local.filter((i) => i.marketplace_id).length} con espejo`);
console.log(`plan: ${plan.length} vínculos a crear; omitidos: ${omitidos.length}`);
for (const o of omitidos) console.log(`  OMITE ${o.slug} (${o.aviso}): ${o.motivo}`);
for (const p of plan) console.log(`  ${p.slug} → ${p.aviso}`);

if (plan.length === 0) process.exit(0);

/* La cuenta v1 se recrea solo si de verdad falta; si ya hay una con esa
 * versión, se usa esa (no se toca su nombre ni su principal). */
let cuenta = cuentas.find((c) => c.version === versionVinculo);
if (!cuenta) {
  if (!aplicar) {
    console.log(`\n[dry-run] habría creado la cuenta «${nombreCuenta}» v${versionVinculo}`);
    console.log(`[dry-run] ${plan.length} vínculos; nada escrito. Usa --aplicar.`);
    process.exit(0);
  }
  cuenta = await api(env.LOCAL_API, '/api/admin/facebook', {
    metodo: 'POST',
    token: tokLocal,
    json: { nombre: nombreCuenta, url: '', notas: `Recuperada desde producción el ${new Date().toISOString().slice(0, 10)} (GW-14)`, principal: false, version: versionVinculo },
  });
  console.log(`cuenta creada: ${cuenta.nombre} v${cuenta.version} (${cuenta.id})`);
} else {
  console.log(`cuenta existente con v${versionVinculo}: ${cuenta.nombre} (${cuenta.id})`);
}

if (!aplicar) {
  console.log(`\n[dry-run] ${plan.length} vínculos; nada escrito. Usa --aplicar.`);
  process.exit(0);
}

let hechos = 0;
const fallos = [];
for (const p of plan) {
  try {
    await api(env.LOCAL_API, `/api/admin/inmuebles/${p.inmuebleId}/facebook/${cuenta.id}/${versionVinculo}`, {
      metodo: 'PUT',
      token: tokLocal,
      json: { marketplace_id: p.aviso },
    });
    hechos += 1;
  } catch (e) {
    fallos.push(`${p.slug} (${p.aviso}): ${e.message}`);
  }
}

/* Verificación: el espejo es lo que lee el borrador, así que se cuenta, no se
 * supone. */
const despues = await todas(env.LOCAL_API, tokLocal);
const conEspejo = despues.filter((i) => i.marketplace_id).length;
console.log(`\ncreados: ${hechos}/${plan.length}; con espejo ahora: ${conEspejo}/${despues.length}`);
for (const f of fallos) console.log(`  FALLO ${f}`);
if (fallos.length > 0) process.exit(1);
