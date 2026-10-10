/* [10AA-13] Tests de la guardia de checksums de migraciones. Sin BD: la
 * comparación con `_sqlx_migrations` se prueba con filas en memoria.
 * Ejecutar: node --test scripts/calidad/guardia-migraciones.test.mjs */

import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { compararChecksums, migracionesEnDisco } from '../branch-db.mjs';

const sha384 = (texto) => createHash('sha384').update(texto).digest('hex');

let raiz;
before(() => {
  /* Temporales solo en C:\tmp (AGENTS.md §0.2). */
  const base = process.platform === 'win32' ? 'C:\\tmp' : tmpdir();
  mkdirSync(base, { recursive: true });
  raiz = mkdtempSync(path.join(base, 'mn-guardia-'));
});
after(() => rmSync(raiz, { recursive: true, force: true }));

test('compararChecksums: checksums iguales no divergen', () => {
  const ficheros = [{ version: 1, nombre: '1_a.sql', sha384: 'aa' }];
  assert.deepEqual(compararChecksums([{ version: 1, checksum: 'aa' }], ficheros), []);
});

test('compararChecksums: fichero editado tras aplicarse diverge', () => {
  const ficheros = [{ version: 35, nombre: '35_x.up.sql', sha384: 'nuevo' }];
  assert.deepEqual(compararChecksums([{ version: 35, checksum: 'viejo' }], ficheros), [
    { version: 35, fichero: '35_x.up.sql', bd: 'viejo', disco: 'nuevo' },
  ]);
});

test('compararChecksums: versión sin fichero no se cuenta (lo cubre versionesAjenas)', () => {
  assert.deepEqual(compararChecksums([{ version: 99, checksum: 'x' }], []), []);
});

test('compararChecksums: BD sin migraciones aplicadas no diverge (BD nueva)', () => {
  const ficheros = [{ version: 1, nombre: '1_a.sql', sha384: 'aa' }];
  assert.deepEqual(compararChecksums([], ficheros), []);
});

test('migracionesEnDisco: solo up y simples, con sha384 de los bytes exactos', () => {
  const dir = path.join(raiz, 'migrations');
  mkdirSync(dir, { recursive: true });
  const contenido = 'CREATE TABLE t (id int);\r\n';
  writeFileSync(path.join(dir, '20260101000001_a.up.sql'), contenido);
  writeFileSync(path.join(dir, '20260101000001_a.down.sql'), 'DROP TABLE t;');
  writeFileSync(path.join(dir, '20260102000000_b.sql'), 'SELECT 1;');
  writeFileSync(path.join(dir, 'LEEME.md'), 'no es migracion');

  assert.deepEqual(migracionesEnDisco(dir), [
    { version: 20260101000001, nombre: '20260101000001_a.up.sql', sha384: sha384(contenido) },
    { version: 20260102000000, nombre: '20260102000000_b.sql', sha384: sha384('SELECT 1;') },
  ]);
});

test('migracionesEnDisco + compararChecksums: cambiar un byte (CRLF) hace divergir', () => {
  const dir = path.join(raiz, 'alterada');
  mkdirSync(dir, { recursive: true });
  const original = 'SELECT 1;\n';
  writeFileSync(path.join(dir, '7_x.sql'), original);
  const aplicada = [{ version: 7, checksum: sha384(original) }];
  assert.deepEqual(compararChecksums(aplicada, migracionesEnDisco(dir)), []);

  writeFileSync(path.join(dir, '7_x.sql'), 'SELECT 1;\r\n');
  const divergentes = compararChecksums(aplicada, migracionesEnDisco(dir));
  assert.equal(divergentes.length, 1);
  assert.equal(divergentes[0].fichero, '7_x.sql');
});
