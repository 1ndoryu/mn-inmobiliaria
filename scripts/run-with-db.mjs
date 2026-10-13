#!/usr/bin/env node

/* Ejecuta cualquier comando de cargo con DATABASE_URL y CARGO_TARGET_DIR
 * alineados a la rama/proyecto actual. */

import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { checksumsDivergentes, findPsql, getBranchDbContext, versionesAjenas } from './branch-db.mjs';
import { sincronizarClaveDevSilencioso } from './clave-dev.mjs';

function cargoCommand() {
  return process.platform === 'win32' ? 'cargo.exe' : 'cargo';
}

/* [10AA-11] En Windows, un `glory-backend.exe` vivo de esta misma rama bloquea
 * el relink de cargo (`Acceso denegado (os error 5)`). Solo se paran procesos
 * `glory-backend.exe` cuyo ejecutable cuelga de este CARGO_TARGET_DIR (la
 * rama): nunca otros proyectos ni opencode. El prefijo lleva `\` final para no
 * casar con `..._otra_rama`.
 * [10AA-30] Nunca se para un proceso con puerto en escucha: ese es un backend
 * que sirve (admin de la usuaria). Se deja vivo y se avisa; si el relink falla
 * por ello, la decisión de parar es de la usuaria. */
function liberarBinariosDeLaRama(targetDir) {
  if (process.platform !== 'win32') return;
  const prefijo = `${path.win32.resolve(targetDir)}\\`;
  const ps = "Get-CimInstance Win32_Process -Filter \"Name='glory-backend.exe'\" | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($env:RAMA_PREFIJO, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { $escucha = @(Get-NetTCPConnection -State Listen -OwningProcess $_.ProcessId -ErrorAction SilentlyContinue).Count -gt 0; if ($escucha) { 'VIVO ' + $_.ProcessId } else { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; 'PARADO ' + $_.ProcessId } }";
  const res = spawnSync('powershell.exe', ['-NoProfile', '-Command', ps], {
    encoding: 'utf8',
    env: { ...process.env, RAMA_PREFIJO: prefijo },
  });
  const lineas = (res.stdout || '').split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  const paradas = lineas.filter((l) => l.startsWith('PARADO ')).map((l) => l.slice(7));
  const vivos = lineas.filter((l) => l.startsWith('VIVO ')).map((l) => l.slice(5));
  if (paradas.length > 0) {
    console.warn(`[db] Paro procesos vivos de esta rama que bloquean el build: PID ${paradas.join(', ')}`);
  }
  if (vivos.length > 0) {
    console.warn(`[db] No paro procesos de esta rama con puerto en escucha (sirven): PID ${vivos.join(', ')}. Si el build falla con \`os error 5\`, decide tú si los paro.`);
  }
}

function migrarBdRama(dbUrl) {
  const version = spawnSync(cargoCommand(), ['sqlx', '--version'], { encoding: 'utf8' });
  if (version.status !== 0) {
    console.warn('[db] Aviso: `cargo-sqlx` no instalado; omito migracion previa.');
    console.warn('[db] Instala con `cargo install sqlx-cli --no-default-features --features postgres`.');
    return;
  }
  const mig = spawnSync(cargoCommand(), ['sqlx', 'migrate', 'run'], {
    stdio: 'inherit',
    env: { ...process.env, DATABASE_URL: dbUrl },
    shell: false,
  });
  if (mig.status !== 0) {
    console.error('[db] La migracion de la BD de rama fallo; repara el estado de');
    console.error('[db] migraciones antes de compilar (ver `Agente/completados/tareas-2026-10-01.md`).');
    process.exit(mig.status ?? 1);
  }
}

const SEMILLA_TESTS = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures', 'seed-test-inmobiliaria.sql');

/* [10AA-20] Datos que necesitan los tests de `handlers::chat::tools::pruebas`
 * (sin ellos: `RowNotFound`). Solo en BD de rama y solo con `test`: en main,
 * la BD es la de desarrollo de la usuaria. Idempotente (ON CONFLICT), se
 * aplica tras migrar. */
function sembrarBdRama(dbUrl) {
  if (!existsSync(SEMILLA_TESTS)) {
    console.warn(`[db] Aviso: falta ${SEMILLA_TESTS}; los tests de chat_tools fallaran con RowNotFound.`);
    return;
  }
  const psqlBin = findPsql();
  if (!psqlBin) {
    console.warn('[db] Aviso: psql no disponible; omito la semilla de tests.');
    return;
  }
  const match = dbUrl.match(/^postgres(?:ql)?:\/\/([^:]+):([^@]+)@([^:/]+):(\d+)\/(.+)$/);
  const [, dbUser, dbPass, dbHost, dbPort, dbName] = match;
  const conexion = ['-U', dbUser, '-h', dbHost, '-p', dbPort, '-d', dbName];
  const entorno = { ...process.env, PGPASSWORD: dbPass };
  /* Sin esquema (migraciones no aplicadas: `cargo sqlx` ausente) no hay nada
   * que sembrar; la semilla se aplicará en la siguiente pasada. */
  const esquema = spawnSync(psqlBin, [...conexion, '-t', '-A', '-c', "SELECT to_regclass('public.inmuebles') IS NOT NULL"], { encoding: 'utf8', env: entorno, shell: false });
  if (esquema.status !== 0) {
    console.error(`[db] No consulto el esquema de ${dbName}: ${(esquema.stderr || '').trim()}`);
    process.exit(esquema.status ?? 1);
  }
  if (esquema.stdout.trim() !== 't') {
    console.warn(`[db] Aviso: ${dbName} aun sin tabla inmuebles; semilla de tests omitida.`);
    return;
  }
  const siembra = spawnSync(psqlBin, [...conexion, '-v', 'ON_ERROR_STOP=1', '-q', '-f', SEMILLA_TESTS], {
    stdio: 'inherit',
    env: entorno,
    shell: false,
  });
  if (siembra.status !== 0) {
    console.error(`[db] La semilla de tests fallo en ${dbName}; no compilo contra una BD a medias.`);
    process.exit(siembra.status ?? 1);
  }
}

/* Parámetros de conexión psql a partir de una URL postgres://. */
function conexionPsql(url) {
  const match = url.match(/^postgres(?:ql)?:\/\/([^:]+):([^@]+)@([^:/]+):(\d+)\/(.+)$/);
  const [, dbUser, dbPass, dbHost, dbPort, dbName] = match;
  return { args: ['-U', dbUser, '-h', dbHost, '-p', dbPort, '-d', dbName], env: { ...process.env, PGPASSWORD: dbPass } };
}

/* [13AA-9] Copia las filas de `tabla` de main a la BD de la rama sin borrar
 * nada (ON CONFLICT: lo que la rama ya tiene se deja). Solo lee main.
 * `sqlEscritura(json)` devuelve el SQL que inserta esas filas y acaba en un
 * SELECT de conteo. Devuelve ese conteo, o null si falla (aviso `[db]`, sin
 * bloquear el arranque). */
function copiarDesdeMain(baseDbUrl, ramaDbUrl, tabla, sqlEscritura) {
  const psqlBin = findPsql();
  if (!psqlBin) {
    console.warn(`[db] Aviso: psql no disponible; no copio ${tabla} de main a esta rama.`);
    return null;
  }
  const origen = conexionPsql(baseDbUrl);
  const leido = spawnSync(psqlBin, [...origen.args, '-v', 'ON_ERROR_STOP=1', '-t', '-A', '-c', `SELECT COALESCE(json_agg(t), '[]'::json) FROM ${tabla} t`], { encoding: 'utf8', env: origen.env, shell: false });
  if (leido.status !== 0) {
    console.warn(`[db] Aviso: no pude leer ${tabla} en main: ${(leido.stderr || '').trim()}`);
    return null;
  }
  const json = leido.stdout.trim();
  /* El JSON va por stdin con cita dólar: nunca como argumento (límite de
   * longitud de Windows) ni interpolado en la línea de comandos. */
  if (json.includes('$payload$')) {
    console.warn(`[db] Aviso: ${tabla} de main con texto inesperado; no lo copio.`);
    return null;
  }
  const destino = conexionPsql(ramaDbUrl);
  const escrito = spawnSync(psqlBin, [...destino.args, '-v', 'ON_ERROR_STOP=1', '-t', '-A', '-f', '-'], {
    input: sqlEscritura(json),
    encoding: 'utf8',
    env: destino.env,
    shell: false,
  });
  if (escrito.status !== 0) {
    console.warn(`[db] Aviso: no pude copiar ${tabla} a esta rama: ${(escrito.stderr || '').trim()}`);
    return null;
  }
  return escrito.stdout.trim().split(/\r?\n/).pop();
}

/* [13AA-8] El laboratorio (opencode-propio-dev) se autentica en MN con tokens
 * emitidos en la BD de main. Cada rama tiene su propia BD, sin esos tokens: al
 * cambiar de rama, MN responde 401 al laboratorio y este aparece como
 * «no conectado». Aquí se copian a la BD de la rama los tokens de main. Se
 * copian también los revocados: así una revocación en main llega a la rama
 * (ON CONFLICT mantiene `revocada` a verdadero). */
function copiarTokensLabABdRama(baseDbUrl, ramaDbUrl) {
  const vigentes = copiarDesdeMain(baseDbUrl, ramaDbUrl, 'mp_tokens_emitidos', (json) => `
INSERT INTO mp_tokens_emitidos (jti, sub, emitida_en, expira_en, revocada)
SELECT jti, sub, emitida_en, expira_en, revocada
FROM json_populate_recordset(NULL::mp_tokens_emitidos, $payload$${json}$payload$::json)
ON CONFLICT (jti) DO UPDATE SET revocada = mp_tokens_emitidos.revocada OR EXCLUDED.revocada;
SELECT count(*) FROM mp_tokens_emitidos WHERE NOT revocada AND expira_en > now();
`);
  if (vigentes === null) {
    console.warn('[db] El laboratorio aparecerá como no conectado en esta rama.');
    return;
  }
  console.log(`[db] Tokens del laboratorio vigentes en esta rama: ${vigentes}`);
}

/* Slug del registro sintético que siembra `test` (scripts/fixtures/seed-test-inmobiliaria.sql). */
const SLUG_SEMILLA_TESTS = 'seed-test-apartamento-caroni';

/* Inmuebles de la rama sin contar la semilla de tests. `null` si no se puede leer. */
function inmueblesPropiosEnRama(ramaDbUrl) {
  const psqlBin = findPsql();
  if (!psqlBin) return null;
  const destino = conexionPsql(ramaDbUrl);
  const leido = spawnSync(psqlBin, [...destino.args, '-v', 'ON_ERROR_STOP=1', '-t', '-A', '-c', `SELECT count(*) FROM inmuebles WHERE slug <> '${SLUG_SEMILLA_TESTS}'`], { encoding: 'utf8', env: destino.env, shell: false });
  if (leido.status !== 0) return null;
  return Number(leido.stdout.trim());
}

/* [13AA-9] Los inmuebles y sus fotos viven en la BD de main. Una rama nueva
 * solo tiene el registro sintético de los tests (ver sembrarBdRama) y el panel
 * no muestra los inmuebles reales. Se copian desde main solo si la rama no
 * tiene inmuebles propios: si ya los tiene, sus cambios mandan y no se toca
 * nada. No se copian las visitas: tienen nombres y teléfonos. */
function copiarInmueblesABdRama(baseDbUrl, ramaDbUrl) {
  const propios = inmueblesPropiosEnRama(ramaDbUrl);
  if (propios === null) {
    console.warn('[db] Aviso: no pude contar los inmuebles de esta rama; no copio desde main.');
    return;
  }
  if (propios > 0) {
    console.log(`[db] La rama ya tiene ${propios} inmueble(s) propio(s); no copio desde main.`);
    return;
  }
  const inmuebles = copiarDesdeMain(baseDbUrl, ramaDbUrl, 'inmuebles', (json) => `
INSERT INTO inmuebles SELECT * FROM json_populate_recordset(NULL::inmuebles, $payload$${json}$payload$::json) ON CONFLICT DO NOTHING;
SELECT count(*) FROM inmuebles;
`);
  if (inmuebles === null) return;
  const fotos = copiarDesdeMain(baseDbUrl, ramaDbUrl, 'fotos', (json) => `
INSERT INTO fotos SELECT * FROM json_populate_recordset(NULL::fotos, $payload$${json}$payload$::json) ON CONFLICT DO NOTHING;
SELECT count(*) FROM fotos;
`);
  console.log(`[db] Inmuebles en esta rama: ${inmuebles}; fotos: ${fotos ?? 'no copiadas'}`);
}

const cargoArgs = process.argv.slice(2);
if (cargoArgs.length === 0) {
  console.error('Uso: node scripts/run-with-db.mjs <subcomando cargo> [...args]');
  process.exit(1);
}

console.log('');
const { branch, isDefaultBranch, dbName, dbUrl, baseDbUrl, cargoTargetDir } = getBranchDbContext();
console.log('');

/* [011A-3] La BD de rama se crea vacia y los macros `query_*!` validan
 * contra la BD viva: sin migraciones, `check/test` fallan con errores
 * cripticos (`no existe la relacion ...`). Migrar aqui deja la BD lista
 * antes de compilar. Si falta `cargo-sqlx` se avisa y se sigue (el backend
 * automigra al arrancar); si la migracion falla, se corta con el error
 * visible en vez de dejar que los macros fallen despues. */
/* [10AA-1] Una BD con migraciones sin fichero es de otro proyecto (p. ej.
 * `glory_backend`): migrar o compilar contra ella corrompe ambas. Se corta
 * antes de tocarla. */
const ajenas = versionesAjenas(dbUrl);
if (ajenas.length > 0) {
  console.error(`[db] La BD ${dbName} tiene migraciones sin fichero en migrations/: ${ajenas.join(', ')}`);
  console.error('[db] Suele ser la BD de otro proyecto. Revisa DATABASE_URL en .env; no migro ni compilo.');
  process.exit(1);
}
/* [10AA-13] Un fichero de migración editado tras aplicarse hace que sqlx
 * rechace la BD al arrancar (`was previously applied but has been modified`).
 * Se corta antes de migrar y compilar, con las versiones afectadas. */
const divergentes = checksumsDivergentes(dbUrl);
if (divergentes.length > 0) {
  console.error(`[db] Migraciones ya aplicadas en ${dbName} con fichero modificado:`);
  for (const d of divergentes) console.error(`[db]   v${d.version} ${d.fichero}`);
  console.error('[db] Causa habitual: EOL del checkout (ver 10AA-12 y .gitattributes). No migro ni compilo.');
  process.exit(1);
}
migrarBdRama(dbUrl);
/* Solo para `test`: con `run` el panel de la usuaria mostraba el inmueble
 * ficticio y, al borrarlo, volvía en el siguiente arranque. */
if (branch !== 'main' && branch !== 'master' && cargoArgs[0] === 'test') sembrarBdRama(dbUrl);
/* [10AA-27] Solo al arrancar el backend: deja el admin local con la clave de
 * `frontend/.env.local` para que la auto-entrada del panel no dé 401. */
if (cargoArgs[0] === 'run') sincronizarClaveDevSilencioso(dbUrl);
/* [13AA-8] Solo al arrancar el backend, en ramas: el laboratorio necesita sus
 * tokens en la BD de la rama (ver copiarTokensLabABdRama). */
if (!isDefaultBranch && cargoArgs[0] === 'run') {
  copiarTokensLabABdRama(baseDbUrl, dbUrl);
  copiarInmueblesABdRama(baseDbUrl, dbUrl);
}
liberarBinariosDeLaRama(cargoTargetDir);

const child = spawn(cargoCommand(), cargoArgs, {
  stdio: 'inherit',
  // [10AA-2 F4] Incremental activo: acelera el ciclo editar-compilar. sccache cachea las dependencias igual.
  env: { ...process.env, DATABASE_URL: dbUrl, CARGO_TARGET_DIR: cargoTargetDir, SCCACHE_BASEDIRS: process.env.SCCACHE_BASEDIRS || 'C:/Users/Owner/OneDrive/Documentos/area-trabajo' },
  shell: false,
});

child.on('error', (err) => {
  console.error('[run-with-db] Error:', err.message);
  process.exit(1);
});
child.on('exit', (code) => process.exit(code ?? 0));
