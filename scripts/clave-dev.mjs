/* [10AA-27] Mantiene la clave del admin LOCAL alineada con la auto-entrada del
 * panel (`frontend/.env.local`: VITE_DEV_EMAIL + VITE_DEV_PASSWORD).
 *
 * Por qué existe: cada BD (`glory_backend`, `glory_backend_inmobiliaria`, las de
 * rama) guarda su propio `admin@admin.com` con su propio hash, y nada las unía
 * con `.env.local`; al cambiar de BD o crear la cuenta con otra clave, la
 * auto-entrada empezaba a dar 401 y la usuaria tenía que loguearse a mano.
 * La fuente de verdad pasa a ser `.env.local`: `run-with-db.mjs run` llama aquí
 * antes de arrancar el backend y, si el hash no coincide, lo reescribe.
 *
 * Seguridad: solo actúa contra una BD en localhost, nunca imprime la clave y la
 * clave jamás viaja en el SQL (solo el hash argon2id resultante).
 *
 * CLI: node scripts/clave-dev.mjs   (usa DATABASE_URL de .env / de la rama) */

import { argon2Sync, randomBytes, timingSafeEqual } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { consultaPsql, getBranchDbContext } from './branch-db.mjs';

const raiz = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const ENV_LOCAL = path.join(raiz, 'frontend', '.env.local');

/* Mismos parámetros que `Argon2::default()` del backend (argon2 0.5). */
const PARAMS = { memory: 19456, passes: 2, parallelism: 1, tagLength: 32 };
const PHC = /^\$argon2id\$v=19\$m=(\d+),t=(\d+),p=(\d+)\$([A-Za-z0-9+/]+)\$([A-Za-z0-9+/]+)$/;
const HOSTS_LOCALES = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

const b64 = (buf) => buf.toString('base64').replace(/=+$/, '');
const deB64 = (texto) => Buffer.from(texto, 'base64');

function derivar(clave, sal, { memory, passes, parallelism, tagLength }) {
  return argon2Sync('argon2id', { message: clave, nonce: sal, memory, passes, parallelism, tagLength });
}

/* Hash PHC argon2id con sal nueva, aceptado por `PasswordHash::new` del backend. */
export function hashearClave(clave) {
  const sal = randomBytes(16);
  const tag = derivar(clave, sal, PARAMS);
  const { memory: m, passes: t, parallelism: p } = PARAMS;
  return `$argon2id$v=19$m=${m},t=${t},p=${p}$${b64(sal)}$${b64(tag)}`;
}

/* ¿`hash` corresponde a `clave`? Recalcula con la sal y los parámetros del propio hash. */
export function claveCoincide(clave, hash) {
  const partes = PHC.exec(hash ?? '');
  if (!partes) return false;
  const [, memory, passes, parallelism, sal, tag] = partes;
  const esperado = deB64(tag);
  const obtenido = derivar(clave, deB64(sal), {
    memory: Number(memory),
    passes: Number(passes),
    parallelism: Number(parallelism),
    tagLength: esperado.length,
  });
  return obtenido.length === esperado.length && timingSafeEqual(obtenido, esperado);
}

/* Credenciales de auto-entrada, o null si no están configuradas. */
export function leerCredencialesDev(rutaEnv = ENV_LOCAL) {
  if (!existsSync(rutaEnv)) return null;
  const vars = {};
  for (const linea of readFileSync(rutaEnv, 'utf8').split(/\r?\n/)) {
    const m = linea.match(/^(VITE_DEV_EMAIL|VITE_DEV_PASSWORD)=(.*)$/);
    if (m) vars[m[1]] = m[2].trim().replace(/^(["'])(.*)\1$/, '$2');
  }
  const email = vars.VITE_DEV_EMAIL;
  const clave = vars.VITE_DEV_PASSWORD;
  if (!email || !clave) return null;
  /* El email entra en el SQL entre comillas: se rechaza cualquier carácter que
   * pueda romperlas en vez de intentar escaparlo. */
  if (!/^[^\s'"\\;]+@[^\s'"\\;]+$/.test(email)) {
    throw new Error('VITE_DEV_EMAIL de frontend/.env.local no es un email válido');
  }
  return { email, clave };
}

function hostDe(dbUrl) {
  return dbUrl.match(/@([^:/]+|\[[^\]]+\]):\d+\//)?.[1] ?? '';
}

/* Devuelve 'omitido' | 'ok' | 'actualizado' | 'creado'. Lanza ante un fallo real. */
export function sincronizarClaveDev(dbUrl, { rutaEnv = ENV_LOCAL, psql = consultaPsql(dbUrl) } = {}) {
  const creds = leerCredencialesDev(rutaEnv);
  if (!creds) return 'omitido';
  if (!HOSTS_LOCALES.has(hostDe(dbUrl))) {
    throw new Error(`BD no local (${hostDe(dbUrl) || 'desconocida'}): no se toca ninguna clave`);
  }
  if (psql("SELECT to_regclass('users') IS NOT NULL").trim() !== 't') return 'omitido';

  const actual = psql(`SELECT password_hash FROM users WHERE email = '${creds.email}'`).trim();
  if (actual && claveCoincide(creds.clave, actual)) return 'ok';

  const hash = hashearClave(creds.clave);
  if (actual) {
    psql(`UPDATE users SET password_hash = '${hash}' WHERE email = '${creds.email}'`);
    return 'actualizado';
  }
  psql(
    `INSERT INTO users (id, email, password_hash, role) VALUES (gen_random_uuid(), '${creds.email}', '${hash}', 'admin')`,
  );
  return 'creado';
}

/* Para el wrapper: nunca bloquea el arranque; solo avisa. */
export function sincronizarClaveDevSilencioso(dbUrl) {
  try {
    const estado = sincronizarClaveDev(dbUrl);
    if (estado === 'actualizado' || estado === 'creado') {
      console.log(`[clave-dev] admin local ${estado}: la auto-entrada vuelve a coincidir con frontend/.env.local`);
    }
  } catch (err) {
    console.warn(`[clave-dev] No se pudo alinear la clave del admin local: ${err.message}`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { dbName, dbUrl } = getBranchDbContext({ verbose: false });
  const estado = sincronizarClaveDev(dbUrl);
  console.log(`[clave-dev] ${dbName}: ${estado}`);
}
