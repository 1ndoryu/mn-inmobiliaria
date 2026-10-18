// Subcomando `mejorar`: pide a la mejora local (3122) una versión de cada foto original.
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync, spawnSync } from 'node:child_process';
import { api, leerEnv, login, magia, AQUI, ENV_DEFECTO } from '../../dev/lib-api.mjs';
import { listar, REQUERIDAS } from './comun.mjs';

/* [15AA-11] Gemini devolvia las fotos con un velo amarillento: el "corrige
 * luz y color de forma natural" de la version anterior le daba pie a tocar
 * el balance de blancos. [16AA-5] Se le pide ademas enderezar la toma.
 * Mismo texto que `PROMPT_DEFECTO` en `frontend/src/domain/foto-mejora.ts`
 * (el panel), que no se puede importar desde aqui por ser TypeScript. */
const PROMPT_DEFECTO = 'Mejora esta foto inmobiliaria a fotografia profesional: sube nitidez y resolucion, arregla la exposicion (sombras y altas luces) sin tocar el color, limpia suciedad leve en paredes, telas u objetos y retira personas y distracciones menores. Haz la imagen ultra HD, mas grande, con mas definicion: maxima resolucion y detalle que puedas devolver. CONSERVA LOS COLORES DEL ORIGINAL: no cambies el balance de blancos ni la temperatura de color; no apliques filtros ni tonos calidos, ambar, dorados o amarillentos, ni vinetas ni degradados. Los blancos y los grises siguen siendo neutros y las paredes, del mismo color que en la original. No cambies la estructura, geometria, muebles ni elementos fijos; no inventes nada que no exista en la foto original. Corrige el angulo si la camara no esta muy bien alineada: endereza la perspectiva (verticales y horizontales) sin deformar la estancia.';

const dormir = (ms) => new Promise((r) => setTimeout(r, ms));

function aDataUrl(bytes, ext) {
  const mime = ext === 'png' ? 'image/png' : ext === 'webp' ? 'image/webp' : 'image/jpeg';
  return `data:${mime};base64,${Buffer.from(bytes).toString('base64')}`;
}

function deDataUrl(dataUrl) {
  const m = /^data:(image\/[a-zA-Z0-9.+-]+);base64,(.+)$/.exec(dataUrl || '');
  if (!m) return null;
  const ext = m[1] === 'image/png' ? 'png' : m[1] === 'image/webp' ? 'webp' : 'jpg';
  return { ext, bytes: Buffer.from(m[2], 'base64') };
}

/* El worker guarda lo que Gemini devuelve con extensión .png sin validar:
 * a veces no es PNG real (el frontend lo tapa al re-codificar en canvas).
 * Aquí se re-codifica a JPG con PIL. */
function normalizarJpg(bytes) {
  const base = join(tmpdir(), `mejorada-${Date.now()}-${Math.floor(Math.random() * 1e6)}`);
  try {
    writeFileSync(`${base}.bin`, bytes);
    execFileSync('python', ['-c',
      'import sys; from PIL import Image; ' +
      `img = Image.open(${JSON.stringify(`${base}.bin`)}).convert("RGB"); ` +
      `img.save(${JSON.stringify(`${base}.jpg`)}, "JPEG", quality=92)`]);
    return readFileSync(`${base}.jpg`);
  } finally {
    rmSync(`${base}.bin`, { force: true });
    rmSync(`${base}.jpg`, { force: true });
  }
}

async function saludMejora(mejoraBase) {
  try {
    const r = await fetch(`${mejoraBase}/api/salud`, { signal: AbortSignal.timeout(10000) });
    if (!r.ok) return null;
    return await r.json();
  } catch {
    return null;
  }
}

/* Token autónomo: salud → (sin puerto debug? abro el Chrome yo) → renovar →
 * rechequeo. Solo molesta a ella si falta login/cuenta (códigos 3/4). */
async function asegurarToken(o, mejoraBase) {
  let s = await saludMejora(mejoraBase);
  if (s?.listo) {
    console.log('Mejora lista (token válido).');
    return;
  }
  console.log(`Token no listo (${s?.detalle ?? 'sin servidor de mejora'}); renuevo solo…`);
  const renovar = join(AQUI, '..', '..', 'frontend', 'scripts', 'renovar-cookies.mjs');
  const correr = () => spawnSync('node', [renovar], { encoding: 'utf8' });
  let r = correr();
  if (r.status === 2) {
    // Sin puerto debug: abro yo el Chrome dedicado y reintento una vez.
    console.log('Abro el Chrome de mejora yo…');
    try {
      const exe = `${process.env.ProgramFiles}\\Google\\Chrome\\Application\\chrome.exe`;
      const perfil = `${process.env.LOCALAPPDATA}\\ChromeHoracioDebug`;
      execFileSync(exe, [`--remote-debugging-port=9223`, `--user-data-dir=${perfil}`, 'https://gemini.google.com/app'], { stdio: 'ignore' });
      await dormir(10000);
      r = correr();
    } catch (e) {
      console.error(`FALLO: no pude abrir chrome.exe solo (${e.message.split('\n')[0]}). Abre el acceso directo "Chrome-Horacio-debug" del escritorio y reejecuta.`);
      process.exit(2);
    }
  }
  if (r.status === 0) {
    s = await saludMejora(mejoraBase);
    if (s?.listo) {
      console.log('Token renovado solo, sigo sin molestar.');
      return;
    }
  }
  console.error(
    'FALLO token: necesito tu ayuda una vez — abre el acceso directo "Chrome-Horacio-debug" del escritorio' +
    (r.status === 3 ? ' e inicia sesión en Google (cuenta Horacio Cruz)' : '') +
    ', y reejecuta. Las originales ya están publicadas; la mejora espera.',
  );
  process.exit(2);
}

export async function cmdMejorar(o) {
  if (!o.slug) {
    console.error('FALLO preflight: falta --slug.');
    process.exit(2);
  }
  const mejoraBase = (o.mejora ?? 'http://127.0.0.1:3122').replace(/\/$/, '');
  await asegurarToken(o, mejoraBase);
  const limite = Math.max(1, Number(o.limite ?? 40) || 40);
  const repetir = o.repetir === true || o.repetir === '';
  /* `--prompt-extra "<texto>"` añade una instrucción puntual a este lote
   * (p. ej. «sube el brillo, quita los objetos de la cocina») sin tocar el
   * prompt por defecto, que es el que usa el panel. */
  const extra = typeof o['prompt-extra'] === 'string' ? o['prompt-extra'].trim() : '';
  const prompt = extra ? `${PROMPT_DEFECTO} ${extra}` : PROMPT_DEFECTO;
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const items = await listar(LOCAL, tL);
  const it = items.find((i) => i.slug === o.slug);
  if (!it) {
    console.error(`FALLO: slug ${o.slug} no existe en local.`);
    process.exit(2);
  }
  const originales = it.fotos.filter((f) => f.origen === 'original').sort((a, b) => a.orden - b.orden);
  const mejoradas = new Set(it.fotos.filter((f) => f.origen === 'mejorada').map((f) => f.orden));
  /* [15AA-11] `--foto <id,id,…>` rehace solo esas originales (ids de la
   * original, no de la mejorada) y siempre repite: es para arreglar una
   * mejora concreta sin tocar las demas del inmueble. */
  const pedidas = new Set(String(o.foto ?? '').split(',').map((s) => s.trim()).filter(Boolean));
  const objetivo = pedidas.size ? originales.filter((f) => pedidas.has(f.id)) : originales;
  if (pedidas.size && objetivo.length !== pedidas.size) {
    const faltan = [...pedidas].filter((id) => !objetivo.some((f) => f.id === id));
    console.error(`FALLO: no son originales de ${o.slug}: ${faltan.join(', ')}`);
    process.exit(2);
  }
  const pendientes = objetivo
    .filter((f) => repetir || pedidas.size > 0 || !mejoradas.has(f.orden))
    .slice(0, limite);
  console.log(`${o.slug}: ${originales.length} originales, ${mejoradas.size} mejoradas, pendientes ${pendientes.length}`);
  let hechas = 0;
  for (const f of pendientes) {
    if (repetir || pedidas.size > 0) {
      // Re-mejora: borra la mejorada vieja del mismo orden para no duplicar el pareo.
      const vieja = it.fotos.find((x) => x.origen === 'mejorada' && x.orden === f.orden);
      if (vieja) {
        await api(LOCAL, `/api/admin/fotos/${vieja.id}`, { metodo: 'DELETE', token: tL });
        console.log(`orden ${f.orden}: mejorada anterior borrada`);
      }
    }
    const bytes = await api(LOCAL, f.url, { token: tL, bytes: true });
    const ext = magia(bytes) ?? 'jpg';
    const fotoId = `${it.id}:${f.orden}@${Date.now()}`; // Único por intento: evita reutilizar un `lista` viejo (F21).
    const post = await api(mejoraBase, '/api/mejora', {
      metodo: 'POST',
      json: { fotoId, original: aDataUrl(bytes, ext), prompt },
    });
    console.log(`orden ${f.orden}: job ${post.jobId.slice(0, 8)}… encolado, espero (puede tardar minutos)`);
    const t0 = Date.now();
    for (;;) {
      await dormir(20000);
      const job = await api(mejoraBase, `/api/mejora/${post.jobId}`, {});
      if (job.estado === 'lista' && job.imagen) {
        const dec = deDataUrl(job.imagen);
        if (!dec) {
          console.error(`orden ${f.orden}: la mejora vino en formato raro, se reintentará luego`);
          break;
        }
        let up = dec.bytes;
        let upExt = dec.ext;
        // Siempre se normaliza a JPG: lo que devuelve Gemini no siempre es
        // un PNG decodificable aunque traiga la magia (el frontend lo tapa
        // al re-codificar en canvas). Si PIL no lo abre, se guarda el testigo.
        console.log(`orden ${f.orden}: mejora recibida (${(up.length / 1024).toFixed(0)} KB, magia=${magia(up) ?? 'ninguna'}), normalizo a JPG…`);
        try {
          up = normalizarJpg(up);
          upExt = 'jpg';
        } catch (e) {
          const testigo = join(tmpdir(), `mejorada-fallo-${f.orden}.bin`);
          try { writeFileSync(testigo, dec.bytes); } catch {}
          console.error(`orden ${f.orden}: bytes no abribles (${e.message.split('\n')[0]}); testigo en ${testigo}, sigo con la siguiente`);
          break;
        }
        await api(LOCAL, `/api/admin/fotos/upload?inmueble_id=${it.id}&filename=${encodeURIComponent(`mejorada-${f.orden + 1}.${upExt}`)}&origen=mejorada&orden=${f.orden}`, {
          metodo: 'POST', token: tL, bytes: up,
        });
        console.log(`orden ${f.orden}: mejorada subida (${(up.length / 1024).toFixed(0)} KB)`);
        hechas += 1;
        break;
      }
      if (job.estado === 'error') {
        console.error(`orden ${f.orden}: error final (${(job.error ?? '').slice(0, 120)}), sigo con la siguiente`);
        break;
      }
      if (Date.now() - t0 > 15 * 60 * 1000) {
        console.error(`orden ${f.orden}: 15 min sin terminar, sigo con la siguiente (el job sigue en cola)`);
        break;
      }
    }
  }
  const fin = (await listar(LOCAL, tL)).find((i) => i.id === it.id);
  console.log(`Mejora OK: ${hechas}/${pendientes.length} nuevas; total mejoradas=${fin.fotos.filter((x) => x.origen === 'mejorada').length}`);
}
