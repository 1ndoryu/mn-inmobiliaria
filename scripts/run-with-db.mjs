#!/usr/bin/env node

/* Ejecuta cualquier comando de cargo con DATABASE_URL y CARGO_TARGET_DIR
 * alineados a la rama/proyecto actual. */

import { spawn, spawnSync } from 'node:child_process';
import path from 'node:path';
import { checksumsDivergentes, getBranchDbContext, versionesAjenas } from './branch-db.mjs';

function cargoCommand() {
  return process.platform === 'win32' ? 'cargo.exe' : 'cargo';
}

/* [10AA-11] En Windows, un `glory-backend.exe` vivo de esta misma rama bloquea
 * el relink de cargo (`Acceso denegado (os error 5)`). Solo se paran procesos
 * `glory-backend.exe` cuyo ejecutable cuelga de este CARGO_TARGET_DIR (la
 * rama): nunca otros proyectos ni opencode. El prefijo lleva `\` final para no
 * casar con `..._otra_rama`. */
function liberarBinariosDeLaRama(targetDir) {
  if (process.platform !== 'win32') return;
  const prefijo = `${path.win32.resolve(targetDir)}\\`;
  const ps = "Get-CimInstance Win32_Process -Filter \"Name='glory-backend.exe'\" | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($env:RAMA_PREFIJO, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; $_.ProcessId }";
  const res = spawnSync('powershell.exe', ['-NoProfile', '-Command', ps], {
    encoding: 'utf8',
    env: { ...process.env, RAMA_PREFIJO: prefijo },
  });
  const pids = (res.stdout || '').split(/\s+/).filter(Boolean);
  if (pids.length > 0) {
    console.warn(`[db] Paro procesos vivos de esta rama que bloquean el build: PID ${pids.join(', ')}`);
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

const cargoArgs = process.argv.slice(2);
if (cargoArgs.length === 0) {
  console.error('Uso: node scripts/run-with-db.mjs <subcomando cargo> [...args]');
  process.exit(1);
}

console.log('');
const { dbName, dbUrl, cargoTargetDir } = getBranchDbContext();
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
liberarBinariosDeLaRama(cargoTargetDir);

const child = spawn(cargoCommand(), cargoArgs, {
  stdio: 'inherit',
  /* [05AA-3] sccache útil: sin incremental + basedirs para que la caché
   * acierte tras cada purga del target/ (el usuario arranca siempre de cero).
   * RUSTC_WRAPPER ya viene del entorno de usuario. */
  env: { ...process.env, DATABASE_URL: dbUrl, CARGO_TARGET_DIR: cargoTargetDir, CARGO_INCREMENTAL: '0', SCCACHE_BASEDIRS: process.env.SCCACHE_BASEDIRS || 'C:/Users/Owner/OneDrive/Documentos/area-trabajo' },
  shell: false,
});

child.on('error', (err) => {
  console.error('[run-with-db] Error:', err.message);
  process.exit(1);
});
child.on('exit', (code) => process.exit(code ?? 0));
