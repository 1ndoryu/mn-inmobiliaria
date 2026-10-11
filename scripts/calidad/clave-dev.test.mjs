/* [10AA-27] Pruebas de `scripts/clave-dev.mjs`: formato del hash, lectura de
 * `.env.local` y guarda de host. Sin BD real: el acceso a psql se inyecta. */

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import {
  claveCoincide,
  hashearClave,
  leerCredencialesDev,
  sincronizarClaveDev,
} from '../clave-dev.mjs';

const BASE = 'C:\\tmp';
const URL_LOCAL = 'postgres://u:p@localhost:5432/glory_test';

function envTemporal(contenido) {
  mkdirSync(BASE, { recursive: true });
  const dir = mkdtempSync(path.join(BASE, 'clave-dev-'));
  const ruta = path.join(dir, '.env.local');
  writeFileSync(ruta, contenido);
  return { ruta, limpiar: () => rmSync(dir, { recursive: true, force: true }) };
}

/* psql falso: devuelve filas según el SQL y apunta cada escritura. */
function psqlFalso({ hashActual = '', tablaUsers = 't' } = {}) {
  const escrituras = [];
  const psql = (sql) => {
    if (sql.includes('to_regclass')) return `${tablaUsers}\n`;
    if (sql.startsWith('SELECT password_hash')) return `${hashActual}\n`;
    escrituras.push(sql);
    return '';
  };
  return { psql, escrituras };
}

test('hashearClave genera PHC argon2id verificable y con sal distinta cada vez', () => {
  const a = hashearClave('clave-de-prueba');
  const b = hashearClave('clave-de-prueba');
  assert.match(a, /^\$argon2id\$v=19\$m=19456,t=2,p=1\$[A-Za-z0-9+/]+\$[A-Za-z0-9+/]+$/);
  assert.notEqual(a, b);
  assert.ok(claveCoincide('clave-de-prueba', a));
  assert.ok(!claveCoincide('otra-clave', a));
});

test('claveCoincide rechaza hashes vacíos o con otro formato', () => {
  assert.equal(claveCoincide('x', ''), false);
  assert.equal(claveCoincide('x', undefined), false);
  assert.equal(claveCoincide('x', '$2b$12$abcdefghijklmnopqrstuv'), false);
});

test('leerCredencialesDev lee email y clave, con comillas y CRLF', () => {
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\r\nVITE_DEV_PASSWORD="abc 123"\r\n');
  try {
    assert.deepEqual(leerCredencialesDev(ruta), { email: 'dev@local.test', clave: 'abc 123' });
  } finally {
    limpiar();
  }
});

test('leerCredencialesDev devuelve null si falta archivo o variable', () => {
  assert.equal(leerCredencialesDev('C:\\tmp\\no-existe\\.env.local'), null);
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\n');
  try {
    assert.equal(leerCredencialesDev(ruta), null);
  } finally {
    limpiar();
  }
});

test('leerCredencialesDev rechaza un email que rompería el SQL', () => {
  const { ruta, limpiar } = envTemporal("VITE_DEV_EMAIL=a'b@x.test\nVITE_DEV_PASSWORD=zzz\n");
  try {
    assert.throws(() => leerCredencialesDev(ruta), /email válido/);
  } finally {
    limpiar();
  }
});

test('sincronizarClaveDev: hash correcto → ok y sin escrituras', () => {
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\nVITE_DEV_PASSWORD=secreta\n');
  try {
    const { psql, escrituras } = psqlFalso({ hashActual: hashearClave('secreta') });
    assert.equal(sincronizarClaveDev(URL_LOCAL, { rutaEnv: ruta, psql }), 'ok');
    assert.equal(escrituras.length, 0);
  } finally {
    limpiar();
  }
});

test('sincronizarClaveDev: hash desfasado → UPDATE con hash válido y sin la clave en el SQL', () => {
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\nVITE_DEV_PASSWORD=secreta\n');
  try {
    const { psql, escrituras } = psqlFalso({ hashActual: hashearClave('vieja') });
    assert.equal(sincronizarClaveDev(URL_LOCAL, { rutaEnv: ruta, psql }), 'actualizado');
    assert.equal(escrituras.length, 1);
    assert.match(escrituras[0], /^UPDATE users SET password_hash = '\$argon2id\$/);
    assert.ok(!escrituras[0].includes('secreta'));
    const hash = escrituras[0].match(/'(\$argon2id[^']+)'/)[1];
    assert.ok(claveCoincide('secreta', hash));
  } finally {
    limpiar();
  }
});

test('sincronizarClaveDev: usuario inexistente → INSERT como admin', () => {
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\nVITE_DEV_PASSWORD=secreta\n');
  try {
    const { psql, escrituras } = psqlFalso({ hashActual: '' });
    assert.equal(sincronizarClaveDev(URL_LOCAL, { rutaEnv: ruta, psql }), 'creado');
    assert.match(escrituras[0], /^INSERT INTO users .*'admin'\)$/);
  } finally {
    limpiar();
  }
});

test('sincronizarClaveDev: sin tabla users o sin credenciales → omitido', () => {
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\nVITE_DEV_PASSWORD=secreta\n');
  try {
    const sinTabla = psqlFalso({ tablaUsers: 'f' });
    assert.equal(sincronizarClaveDev(URL_LOCAL, { rutaEnv: ruta, psql: sinTabla.psql }), 'omitido');
    assert.equal(sincronizarClaveDev(URL_LOCAL, { rutaEnv: 'C:\\tmp\\nada\\.env.local', psql: sinTabla.psql }), 'omitido');
  } finally {
    limpiar();
  }
});

test('sincronizarClaveDev se niega a tocar una BD que no es local', () => {
  const { ruta, limpiar } = envTemporal('VITE_DEV_EMAIL=dev@local.test\nVITE_DEV_PASSWORD=secreta\n');
  try {
    const { psql, escrituras } = psqlFalso();
    assert.throws(
      () => sincronizarClaveDev('postgres://u:p@db.produccion.example:5432/glory', { rutaEnv: ruta, psql }),
      /no local/,
    );
    assert.equal(escrituras.length, 0);
  } finally {
    limpiar();
  }
});
