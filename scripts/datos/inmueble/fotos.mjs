// Preflight de fotos: valida cada archivo y deja copias sin EXIF en C:\tmp.
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { AQUI, magia } from '../../dev/lib-api.mjs';

const MAX_BYTES = 10 * 1024 * 1024;

/* Devuelve [{ruta, ext, bytes}] con EXIF ya pelado en C:\tmp.
 * Nunca publica con GPS: si PIL falla, se aborta (nada silencioso). */
export function prepararFotos(carpeta) {
  if (!carpeta || !existsSync(carpeta) || !statSync(carpeta).isDirectory()) {
    console.error(`FALLO preflight: carpeta de fotos inexistente: ${carpeta}`);
    process.exit(2);
  }
  const ficheros = readdirSync(carpeta)
    .filter((f) => statSync(join(carpeta, f)).isFile())
    .sort((a, b) => a.localeCompare(b, 'es'));
  if (!ficheros.length) {
    console.error(`FALLO preflight: sin archivos en ${carpeta}`);
    process.exit(2);
  }
  const tmp = join('C:\\tmp', `inmueble-${Date.now()}`);
  mkdirSync(tmp, { recursive: true });
  const listas = [];
  for (const f of ficheros) {
    const ruta = join(carpeta, f);
    const buf = readFileSync(ruta);
    if (buf.length > MAX_BYTES) {
      console.error(`FALLO preflight: ${f} supera 10 MiB (${(buf.length / 1048576).toFixed(1)} MiB).`);
      process.exit(2);
    }
    const ext = magia(buf);
    if (!ext) {
      console.error(`FALLO preflight: ${f} no es JPEG/PNG/WebP (¿HEIC? pide reemplazo).`);
      process.exit(2);
    }
    const limpia = join(tmp, `${listas.length}.${ext}`);
    try {
      const salida = execFileSync('python', [join(AQUI, '..', 'datos', 'pelar-exif.py'), ruta, limpia], { encoding: 'utf8' });
      const info = JSON.parse(salida.trim().split('\n').pop());
      if (info.gps) console.log(`aviso ${f}: traía GPS, pelado antes de subir`);
    } catch (e) {
      console.error(`FALLO EXIF ${f}: sin pelado no se publica (privacidad). ${e.message.split('\n')[0]}`);
      process.exit(2);
    }
    listas.push({ ruta: limpia, ext, nombre: f });
  }
  return { listas, tmp };
}
