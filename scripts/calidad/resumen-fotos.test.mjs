/* [25AA-2] Pruebas de `frontend/src/domain/sync/resumen-fotos.ts`: el plan de sync
 * compara fotos por posición, y aquí se comprueba que mover la principal se cuenta
 * como un solo cambio. El frontend no tiene runner: se transpila el TS con el
 * `typescript` del propio frontend y se importa como módulo en memoria. */

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import test from 'node:test';

const requerir = createRequire(new URL('../../frontend/package.json', import.meta.url));
const ts = requerir('typescript');
const fuente = readFileSync(new URL('../../frontend/src/domain/sync/resumen-fotos.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(fuente, { compilerOptions: { module: ts.ModuleKind.ES2022, target: ts.ScriptTarget.ES2022 } });
const { eleccionDeGrupo, esFilaFoto, resumirFotos } = await import(
  `data:text/javascript;base64,${Buffer.from(js.outputText).toString('base64')}`
);

/* Fila mínima del plan: solo los campos que lee el resumen. */
const fila = (clave, elegido, local, servidor, accion = 'escribir') => ({ clave, elegido, local, servidor, accion });

test('esFilaFoto solo reconoce claves foto:*', () => {
  assert.equal(esFilaFoto('foto:original:0'), true);
  assert.equal(esFilaFoto('titulo'), false);
});

test('eleccionDeGrupo: todas, ninguna o mezcla', () => {
  const claves = ['foto:original:0', 'foto:original:1'];
  assert.equal(eleccionDeGrupo(claves, []), 'ultimo');
  assert.equal(eleccionDeGrupo(claves, claves), 'viejo');
  assert.equal(eleccionDeGrupo(claves, [claves[0]]), 'mixto');
});

test('cambiar la principal con varias posiciones desplazadas es un solo resumen', () => {
  const filas = [
    fila('foto:original:0', 'local', 'a', 'b'),
    fila('foto:original:1', 'local', 'b', 'c'),
    fila('foto:original:2', 'local', 'c', 'a'),
  ];
  const lineas = resumirFotos(filas);
  assert.match(lineas[0], /principal/);
  assert.ok(lineas.some((l) => l.includes('En el Servidor: se cambian 3 posiciones')));
});

test('fotos solo en un lado se añaden o se quitan, sin contar como cambio de posición', () => {
  const filas = [fila('foto:original:3', 'local', 'x', null), fila('foto:original:4', 'servidor', null, 'y', 'quitar')];
  const lineas = resumirFotos(filas);
  assert.ok(lineas.some((l) => l.includes('se añade 1 foto')));
  assert.ok(!lineas.some((l) => l.includes('principal')));
});

test('sin filas de foto no hay resumen', () => {
  assert.deepEqual(resumirFotos([fila('titulo', 'local', 'a', 'b')]), []);
});
