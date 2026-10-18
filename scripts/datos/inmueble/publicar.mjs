// Subcomandos `estado` y `publicar`: crear o reemplazar un inmueble en local y/o prod.
import { readFileSync, rmSync } from 'node:fs';
import { api, leerEnv, login, nucleo, ENV_DEFECTO } from '../../dev/lib-api.mjs';
import { diffNucleo, leerDatos, listar, porTitulo, REQUERIDAS } from './comun.mjs';
import { prepararFotos } from './fotos.mjs';

/* Sincroniza UN destino: crea o (con --sobrescribir) reemplaza + fotos + publica. */
async function sincronizarDestino(base, quien, token, datos, fotos, { borrador, sobrescribir, dry }) {
  const items = await listar(base, token);
  const existente = porTitulo(items, datos.titulo);
  if (existente && !sobrescribir) {
    const dif = diffNucleo(existente, { ...existente, ...datos });
    console.error(
      `FALLO ${quien}: ya existe "${datos.titulo}" (slug ${existente.slug})` +
      (dif.length ? `, difiere en: ${dif.join(', ')}` : ', idéntico') +
      '. Reejecuta con --sobrescribir para reemplazar.',
    );
    process.exit(3);
  }
  if (dry) {
    console.log(`[dry-run] ${quien}: ${existente ? 'reemplazar' : 'crear'} "${datos.titulo}" + ${fotos.length} fotos`);
    return { id: existente?.id ?? null, slug: existente?.slug ?? null, seco: true };
  }
  let id;
  if (!existente) {
    const creado = await api(base, '/api/admin/inmuebles', { metodo: 'POST', token, json: datos });
    id = creado.id;
    console.log(`OK ${quien}: creado ${creado.slug} (${id})`);
  } else {
    await api(base, `/api/admin/inmuebles/${existente.id}`, { metodo: 'PUT', token, json: { ...nucleo(existente), ...datos } });
    id = existente.id;
    console.log(`OK ${quien}: reemplazado ${existente.slug}`);
    const actuales = (await listar(base, token)).find((i) => i.id === id);
    for (const f of actuales.fotos) {
      await api(base, `/api/admin/fotos/${f.id}`, { metodo: 'DELETE', token });
    }
  }
  // Fotos: originales con orden 0..N (bytes ya limpios de EXIF).
  let orden = 0;
  for (const f of fotos) {
    const bytes = readFileSync(f.ruta);
    await api(base, `/api/admin/fotos/upload?inmueble_id=${id}&filename=${encodeURIComponent(`foto-${orden + 1}.${f.ext}`)}&origen=original&orden=${orden}`, {
      metodo: 'POST', token, bytes,
    });
    orden += 1;
  }
  if (!borrador) {
    await api(base, `/api/admin/inmuebles/${id}/publicacion`, { metodo: 'PATCH', token, json: { publicado: true } });
  }
  // Verificación del destino.
  const fin = (await listar(base, token)).find((i) => i.id === id);
  const pub = await api(base, `/api/public/inmuebles/${fin.slug}`).catch(() => null);
  const okFotos = fin.fotos.length === fotos.length;
  const okPub = borrador ? true : pub !== null && pub.publicado !== false;
  console.log(`${quien}: ${fin.slug} fotos=${fin.fotos.length}/${fotos.length} publico=${pub !== null}`);
  if (!okFotos || !okPub) {
    console.error(`FALLO verificación en ${quien}.`);
    process.exit(4);
  }
  return { id, slug: fin.slug };
}

export async function cmdEstado(o) {
  if (!o.slug) {
    console.error('FALLO preflight: falta --slug.');
    process.exit(2);
  }
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = cfg.PROD_BASE.replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  for (const [quien, base, token] of [['local', LOCAL, tL], ['prod', PROD, tP]]) {
    const items = await listar(base, token);
    const it = items.find((i) => i.slug === o.slug);
    if (!it) {
      console.log(`${quien}: slug ${o.slug} NO existe`);
      continue;
    }
    const pub = await api(base, `/api/public/inmuebles/${o.slug}`).catch(() => null);
    const porOrigen = {};
    for (const f of it.fotos) porOrigen[f.origen] = (porOrigen[f.origen] ?? 0) + 1;
    console.log(`${quien}: "${it.titulo}" publicado=${it.publicado} fotos=${it.fotos.length} (${JSON.stringify(porOrigen)}) publico=${pub !== null}`);
  }
}

export async function cmdPublicar(o) {
  const datos = leerDatos(o.datos);
  if (!datos.titulo || !String(datos.titulo).trim()) {
    console.error('FALLO preflight: --datos.titulo es obligatorio.');
    process.exit(2);
  }
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = cfg.PROD_BASE.replace(/\/$/, '');
  const soloLocal = o['solo-local'] === true || o['solo-local'] === '';
  const soloProd = o['solo-prod'] === true || o['solo-prod'] === '';
  const dry = o['dry-run'] === true || o['dry-run'] === '';
  const { listas, tmp } = prepararFotos(o.fotos);
  console.log(`Fotos listas: ${listas.length} (EXIF pelado en ${tmp})`);
  try {
    if (!soloProd) {
      const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
      await sincronizarDestino(LOCAL, 'local', tL, datos, listas, { borrador: !!o.borrador, sobrescribir: !!o.sobrescribir, dry });
    }
    if (!soloLocal) {
      const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
      await sincronizarDestino(PROD, 'prod', tP, datos, listas, { borrador: !!o.borrador, sobrescribir: !!o.sobrescribir, dry });
    }
    console.log(dry ? 'dry-run OK (nada escrito)' : 'Publicación OK en ' + (soloLocal ? 'local' : soloProd ? 'prod' : 'local+prod'));
  } finally {
    if (!dry) rmSync(tmp, { recursive: true, force: true });
  }
}
