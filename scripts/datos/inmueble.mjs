// [08AA-27] CLI único para publicar/gestionar inmuebles (local + prod).
// Uso:
//   node scripts/datos/inmueble.mjsestado --slug <slug>
//   node scripts/datos/inmueble.mjspublicar --fotos <carpeta> --datos '{"titulo":...}' [--borrador] [--solo-local|--solo-prod] [--sobrescribir]
//   node scripts/datos/inmueble.mjsverificar [--slug <slug>] [--sin-bytes] [--par N]
//   node scripts/datos/inmueble.mjsmejorar --slug <slug> [--foto <id,...>] [--repetir] [--prompt-extra "…"]
// --datos acepta JSON inline o @ruta.json. --dry-run muestra sin escribir.
// Exit: 0 ok | 1 verificar con diferencias | 2 preflight/auth | 3 error de sync | 4 verificación fallida.
// Cada subcomando vive en scripts/datos/inmueble/ (un módulo por responsabilidad).
import { args } from './inmueble/comun.mjs';
import { cmdEstado, cmdPublicar } from './inmueble/publicar.mjs';
import { cmdMejorar } from './inmueble/mejorar.mjs';
import { cmdPush, cmdPushVinculos } from './inmueble/sincronizar.mjs';
import { cmdVerificar } from './inmueble/verificar.mjs';

async function main() {
  const { cmd, o } = args();
  if (cmd === 'estado') return cmdEstado(o);
  if (cmd === 'publicar') return cmdPublicar(o);
  if (cmd === 'mejorar') return cmdMejorar(o);
  if (cmd === 'push') return o.vinculos === true || o.vinculos === '' ? cmdPushVinculos(o) : cmdPush(o);
  if (cmd === 'verificar') return cmdVerificar(o);
  console.log('Uso: node scripts/datos/inmueble.mjs<estado|publicar|mejorar|push|verificar> [opciones]');
  console.log('  estado --slug <slug> | publicar --fotos <carpeta> --datos \'{...}\'|@f.json [--borrador] [--solo-local|--solo-prod] [--sobrescribir] [--dry-run]');
  console.log('  mejorar --slug <slug> [--limite N] [--repetir] [--foto <id,id,…>]  (solo local)');
  console.log('  push --slug <slug> [--dry-run] [--sobrescribir]  (local → prod)');
  console.log('  push --vinculos [--dry-run]  (todos los vínculos de Marketplace, local → prod)');
  console.log('  verificar [--slug <slug>] [--sin-bytes] [--par N]  (prod↔local, solo lectura)');
  process.exit(2);
}

main().catch((e) => {
  console.error(`FALLO: ${e.message}`);
  process.exit(3);
});
