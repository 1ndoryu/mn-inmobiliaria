// Subcomando `push`: empuja un slug de local a prod (núcleo, estado y fotos faltantes).
import { api, leerEnv, login, magia, nucleo, ENV_DEFECTO } from '../../dev/lib-api.mjs';
import { diffNucleo, listar, REQUERIDAS } from './comun.mjs';

/* [18AA-2] `push --vinculos`: empareja por slug TODOS los inmuebles locales con
 * vínculo y escribe en prod solo ese campo. Es la puesta al día de una vez:
 * empujar el núcleo entero inmueble a inmueble pisaría en prod lo que allí sea
 * más nuevo, y el vínculo es lo único que se quiere mover. Sin --dry-run no
 * toca nada. */
export async function cmdPushVinculos(o) {
  const seco = o['dry-run'] === true || o['dry-run'] === '';
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = (cfg.PROD_BASE ?? 'https://mn-inmobiliaria.com').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  const locales = (await listar(LOCAL, tL)).filter((i) => i.marketplace_id);
  const prod = await listar(PROD, tP);
  let iguales = 0;
  let puestos = 0;
  let sinPar = 0;
  for (const loc of locales) {
    const pr = prod.find((i) => i.slug === loc.slug);
    if (!pr) {
      console.log(`  sin par en prod: ${loc.slug} (${loc.marketplace_id})`);
      sinPar++;
      continue;
    }
    if (pr.marketplace_id === loc.marketplace_id) {
      iguales++;
      continue;
    }
    if (!seco) await api(PROD, `/api/admin/inmuebles/${pr.id}`, { metodo: 'PUT', token: tP, json: { marketplace_id: loc.marketplace_id } });
    console.log(`  ${seco ? '[dry-run] ' : ''}${loc.slug}: ${pr.marketplace_id ?? '(sin vínculo)'} -> ${loc.marketplace_id}`);
    puestos++;
  }
  console.log(`vínculos: ${locales.length} con vínculo en local | ${puestos} ${seco ? 'por poner' : 'puestos'} | ${iguales} ya iguales | ${sinPar} sin par en prod`);
  return sinPar === 0 ? 0 : 1;
}

/* F4: empuja un slug de local a prod (núcleo + estado + fotos faltantes).
 * Sin --dry-run no toca nada. Pareo de fotos por (origen, orden). */
export async function cmdPush(o) {
  if (!o.slug) {
    console.error('FALLO preflight: falta --slug.');
    process.exit(2);
  }
  const seco = o['dry-run'] === true || o['dry-run'] === '';
  const sobrescribir = o.sobrescribir === true || o.sobrescribir === '';
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = (cfg.PROD_BASE ?? 'https://mn-inmobiliaria.com').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  const loc = (await listar(LOCAL, tL)).find((i) => i.slug === o.slug);
  if (!loc) {
    console.error(`FALLO: slug ${o.slug} no existe en local.`);
    process.exit(2);
  }
  let pr = (await listar(PROD, tP)).find((i) => i.slug === o.slug) ?? null;
  const plan = [];
  if (!pr) {
    plan.push('crear inmueble en prod');
    if (!seco) {
      // El POST responde 201 sin cuerpo: el id se localiza por diferencia de listados.
      // El título va ya en el alta porque el slug se genera de él; con `{}` queda «inmueble».
      const antes = new Set((await listar(PROD, tP)).map((i) => i.id));
      await api(PROD, '/api/admin/inmuebles', { metodo: 'POST', token: tP, json: { titulo: loc.titulo } });
      const creado = (await listar(PROD, tP)).find((i) => !antes.has(i.id));
      if (!creado) {
        console.error('FALLO: no localizo en prod el inmueble recién creado.');
        process.exit(1);
      }
      await api(PROD, `/api/admin/inmuebles/${creado.id}`, { metodo: 'PUT', token: tP, json: nucleo(loc) });
      // El alta nace sin publicar: si en local está publicado, se publica aquí también.
      if (loc.publicado) await api(PROD, `/api/admin/inmuebles/${creado.id}/publicacion`, { metodo: 'PATCH', token: tP, json: { publicado: true } });
      pr = (await listar(PROD, tP)).find((i) => i.id === creado.id);
    }
  } else {
    const dif = diffNucleo(pr, loc);
    if (dif.length || sobrescribir) {
      plan.push(`actualizar núcleo (${dif.length ? dif.join(',') : 'forzado'})`);
      if (!seco) await api(PROD, `/api/admin/inmuebles/${pr.id}`, { metodo: 'PUT', token: tP, json: nucleo(loc) });
    }
    if (pr.publicado !== loc.publicado) {
      plan.push(`publicado=${loc.publicado}`);
      if (!seco) await api(PROD, `/api/admin/inmuebles/${pr.id}/publicacion`, { metodo: 'PATCH', token: tP, json: { publicado: loc.publicado } });
    }
    if (sobrescribir && pr.fotos.length) {
      plan.push(`borrar ${pr.fotos.length} fotos de prod`);
      // Borrar una original arrastra a su mejorada: si ya no existe (404), se da por borrada.
      if (!seco) {
        for (const f of pr.fotos) {
          await api(PROD, `/api/admin/fotos/${f.id}`, { metodo: 'DELETE', token: tP }).catch((e) => {
            if (!/→ 404/.test(e.message)) throw e;
          });
        }
      }
      pr = { ...pr, fotos: [] };
    }
  }
  const enProd = new Set((pr?.fotos ?? []).map((f) => `${f.origen}:${f.orden}`));
  const faltan = loc.fotos.filter((f) => !enProd.has(`${f.origen}:${f.orden}`));
  plan.push(`subir ${faltan.length}/${loc.fotos.length} fotos`);
  if (!seco) {
    pr ??= (await listar(PROD, tP)).find((i) => i.slug === o.slug);
    for (const f of faltan) {
      const bytes = await api(LOCAL, f.url, { token: tL, bytes: true });
      const ext = magia(bytes) ?? 'jpg';
      await api(PROD, `/api/admin/fotos/upload?inmueble_id=${pr.id}&filename=${encodeURIComponent(`${f.origen}-${f.orden + 1}.${ext}`)}&origen=${f.origen}&orden=${f.orden}`, {
        metodo: 'POST', token: tP, bytes,
      });
    }
  }
  console.log(`${seco ? '[dry-run] ' : ''}push ${o.slug}: ${plan.join(' | ') || 'sin cambios'}`);
  if (!seco) {
    const fin = (await listar(PROD, tP)).find((i) => i.slug === o.slug);
    const pub = await fetch(`${PROD}/api/public/inmuebles/${o.slug}`, { signal: AbortSignal.timeout(15000) });
    console.log(`Push OK: prod=${fin.fotos.length} fotos publicado=${fin.publicado} publico=${pub.ok}`);
    if (!pub.ok || fin.fotos.length !== loc.fotos.length) {
      console.error('FALLO verificación post-push: revisa el slug en prod.');
      process.exit(1);
    }
  }
}
