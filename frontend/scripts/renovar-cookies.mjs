#!/usr/bin/env node
/* F22: renueva GEMINI_PSID/GEMINI_PSIDTS sin copiar/pegar a mano.
 *
 * Usa tu Chrome de verdad (con la sesión de Gemini ya abierta) a través de
 * su puerto de depuración, lee las cookies y las guarda en `.env.local`.
 * El backend las recarga solo al detectar el cambio: no hay que reiniciar.
 *
 * Requisito único: Chrome debe estar abierto con depuración activada. Una
 * sola vez: cierra Chrome del todo y ábrelo con
  *   --remote-debugging-port=9223
 * (o añade ese flag al acceso directo que uses siempre). Si este script no
 * ve el puerto, te dice exactamente qué hacer y sale con código 2.
 *
 * Los valores nunca se imprimen: solo su longitud. Códigos de salida:
 * 0 = OK, 2 = sin puerto debug, 3 = hay que iniciar sesión a mano,
 * 4 = no se encontraron las cookies (¿cuenta equivocada?).
 *
 * F26: ese Chrome tiene varios perfiles y el primero con PSID puede estar
 * degradado (texto OK, imágenes rechazadas). Por eso cada candidata se valida
 * con un login real y solo se escribe la que Google acepta como AVAILABLE. */

import { readFile, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const AQUI = path.dirname(fileURLToPath(import.meta.url));
const RAIZ = path.dirname(AQUI);
const ENV_PATH = path.join(RAIZ, '.env.local');
const CDP = 'http://127.0.0.1:9223';
// Perfil dedicado de depuración: el acceso directo del escritorio arranca
// Chrome con --user-data-dir=%LOCALAPPDATA%\ChromeHoracioDebug (cuenta
// Horacio Cruz, andrewnaw2@gmail.com), separado del Chrome normal para que el
// CDP vea siempre esa sesión sin cerrar nada. Un user-data-dir nuevo crea su
// perfil como directorio "Default".
const PERFIL_ESPERADO = 'Default';
// En el perfil dedicado solo vive la cuenta Horacio: se usa /app a secas
// (en un perfil nuevo /u/5 redirigiría a /u/0 igualmente).
const GEMINI_URL = 'https://gemini.google.com/app';

function instruccionesDebug() {
  return [
    'No veo Chrome con depuración en 127.0.0.1:9223.',
    'Usa el acceso directo "Chrome-Horacio-debug" del escritorio, que arranca con:',
    '  chrome.exe --remote-debugging-port=9223 --user-data-dir=%LOCALAPPDATA%\\ChromeHoracioDebug',
    '  (perfil dedicado: no toca tu Chrome normal, puedes dejarlo abierto).',
    '  Primera vez: inicia sesión en Google (cuenta Horacio Cruz) en esa ventana.',
    '  El propio acceso directo renueva las cookies solo (espera a que diga AVAILABLE).',
  ].join('\n');
}

async function hayPuertoDebug() {
  try {
    const r = await fetch(`${CDP}/json/version`, { signal: AbortSignal.timeout(3000) });
    return r.ok;
  } catch {
    return false;
  }
}

async function actualizarEnvLocal(psid, psidts) {
  let texto = '';
  try {
    texto = await readFile(ENV_PATH, 'utf8');
  } catch {
    // No existe: se crea con las dos claves.
  }
  const lineas = texto.split('\n');
  let vistoPsid = false;
  let vistoPsidts = false;
  const nuevas = lineas.map((l) => {
    const t = l.trim();
    if (t.startsWith('GEMINI_PSID=')) {
      vistoPsid = true;
      return 'GEMINI_PSID=' + psid;
    }
    if (t.startsWith('GEMINI_PSIDTS=')) {
      vistoPsidts = true;
      return 'GEMINI_PSIDTS=' + psidts;
    }
    return l;
  });
  if (!vistoPsid) nuevas.push('GEMINI_PSID=' + psid);
  if (!vistoPsidts) nuevas.push('GEMINI_PSIDTS=' + psidts);
  await writeFile(ENV_PATH, nuevas.join('\n').replace(/\n{3,}/g, '\n\n'), 'utf8');
}

let navegador = null;
try {
  if (!(await hayPuertoDebug())) {
    console.error(instruccionesDebug());
    process.exit(2);
  }
  navegador = await chromium.connectOverCDP(CDP);
  const contextos = navegador.contexts();
  if (contextos.length === 0) {
    console.error('Chrome conectado pero sin ventanas/contextos. Abre una ventana y reintenta.');
    process.exit(2);
  }
  // F26: ese Chrome tiene varios perfiles y el primero con PSID puede no ser
  // el bueno (texto OK pero imágenes rechazadas = sesión degradada, estado
  // distinto de AVAILABLE). Se recogen las candidatas de TODOS los contextos
  // y solo se escribe la que Google acepta con un login real (sin gastar imagen).
  async function candidatas() {
    const lista = [];
    for (let i = 0; i < contextos.length; i++) {
      let cs = [];
      try {
        cs = await contextos[i].cookies('https://gemini.google.com');
      } catch {
        continue; // Ese contexto no deja leer cookies: se salta.
      }
      // Google moderno ya no emite la PSID pelada en perfiles nuevos: la
      // sesión vive en __Secure-1PSID (+__Secure-1PSIDTS), que es justo lo que
      // gemini_webapi espera como secure_1psid/secure_1psidts. La pelada queda
      // como legado.
      const psid = cs.find((x) => x.name === '__Secure-1PSID')?.value
        ?? cs.find((x) => x.name === 'PSID')?.value ?? '';
      if (!psid) continue;
      const psidts = cs.find((x) => x.name === '__Secure-1PSIDTS')?.value
        ?? cs.find((x) => x.name === '__Secure-3PSIDTS')?.value ?? '';
      lista.push({ indice: i, psid, psidts });
    }
    return lista;
  }
  /* Valida una candidata con un login real y devuelve el estado que Google
   * le da (AVAILABLE = sirve para texto e imágenes). Los valores nunca se
   * imprimen ni viajan en argv: van por entorno al hijo. */
  async function validarCandidata(cand) {
    const comprobacion = [
      'import asyncio, os',
      'async def main():',
      '    from gemini_webapi import GeminiClient',
      '    c = GeminiClient(os.environ.get("CAND_PSID", ""), os.environ.get("CAND_PSIDTS", "") or None)',
      '    try:',
      '        await c.init(timeout=30, auto_refresh=False)',
      '        print(c.account_status.name)',
      '    finally:',
      '        try:',
      '            await c.close()',
      '        except Exception:',
      '            pass',
      'asyncio.run(main())',
    ].join('\n');
    const python = process.env.MEJORA_PYTHON || 'python';
    return await new Promise((resolve) => {
      let salida = '';
      let hijo = null;
      try {
        hijo = spawn(python, ['-c', comprobacion], {
          env: { ...process.env, CAND_PSID: cand.psid, CAND_PSIDTS: cand.psidts },
          stdio: ['ignore', 'pipe', 'ignore'],
        });
      } catch {
        resolve({ estado: null, motivo: 'sin-python' });
        return;
      }
      const temporizador = setTimeout(() => {
        try {
          hijo.kill();
        } catch {
          // Ya terminó: el close resuelve.
        }
        resolve({ estado: null, motivo: 'timeout' });
      }, 90000);
      hijo.on('error', () => {
        clearTimeout(temporizador);
        resolve({ estado: null, motivo: 'sin-python' });
      });
      hijo.stdout.on('data', (c) => {
        salida += c.toString();
      });
      hijo.on('close', (codigo) => {
        clearTimeout(temporizador);
        const estado = salida.split('\n').map((l) => l.trim()).find(Boolean) ?? '';
        resolve(codigo === 0 && estado ? { estado, motivo: null } : { estado: null, motivo: 'login-fallo' });
      });
    });
  }
  let cands = await candidatas();
  if (cands.length === 0) {
    // Sin PSID en ningún contexto: se abre Gemini en el primero para iniciar
    // sesión/cargar y se relee.
    const pagina = await contextos[0].newPage();
    try {
      await pagina.goto(GEMINI_URL, { waitUntil: 'domcontentloaded', timeout: 45000 });
      await pagina.waitForURL(/gemini\.google\.com\/.*app/, { timeout: 45000 });
    } catch {
      const url = pagina.url();
      if (url.includes('accounts.google.com')) {
        console.error(
          'Ese Chrome no tiene la sesión de Google iniciada. Inicia sesión una vez en accounts.google.com y reintenta.',
        );
        process.exit(3);
      }
      throw new Error(`Gemini no cargó a tiempo (estoy en ${url}). Reintenta.`);
    } finally {
      await pagina.close().catch(() => {});
    }
    cands = await candidatas();
  }
  if (cands.length === 0) {
    console.error(
      'No encontré la cookie PSID en ese Chrome. Abre a mano https://gemini.google.com/app en esa ventana, comprueba que ves tus chats y reintenta.',
    );
    process.exit(4);
  }
  // Se valida cada candidata en orden y se escribe la primera AVAILABLE.
  let elegida = null;
  let verificada = true;
  const estados = [];
  for (const cand of cands) {
    const r = await validarCandidata(cand);
    if (r.motivo === 'sin-python' || r.motivo === 'timeout') {
      verificada = false;
      break;
    }
    estados.push(`ctx${cand.indice}=${r.estado ?? r.motivo}`);
    if (r.estado === 'AVAILABLE') {
      elegida = cand;
      break;
    }
  }
  if (!elegida && !verificada) {
    // Sin validador no se puede distinguir: se conserva el comportamiento
    // anterior (primera candidata) avisando de que va sin verificar.
    elegida = cands[0];
  }
  if (!elegida) {
    console.error(
      `Ninguna sesión de ese Chrome sirve para imágenes (${estados.join(', ')}). ` +
        'Abre a mano https://gemini.google.com/app en esa ventana de depuración, genera una imagen para comprobar que la cuenta puede, y reintenta.',
    );
    process.exit(4);
  }
  await actualizarEnvLocal(elegida.psid, elegida.psidts);
  console.log(
    `OK: .env.local actualizado desde el perfil horacio (${PERFIL_ESPERADO}, contexto ${elegida.indice} de ${contextos.length} CDP, estado ${verificada ? 'AVAILABLE' : 'sin-verificar'}, PSID ${elegida.psid.length} chars${elegida.psidts ? `, PSIDTS ${elegida.psidts.length} chars` : ', sin PSIDTS (opcional)'}). El backend las recarga solo.`,
  );
} catch (e) {
  console.error(`Fallo: ${e instanceof Error ? e.message : e}`);
  process.exit(1);
} finally {
  // Se cierra la conexión CDP; el Chrome del usuario sigue abierto.
  await navegador?.close().catch(() => {});
}
