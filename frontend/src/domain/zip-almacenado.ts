/* ZIP sin compresión (método STORE) para empaquetar las fotos: ya son JPEG y
 * comprimirlas no ganaría casi nada. Dominio puro: devuelve bytes; el Blob lo
 * monta la plataforma (ver platform/logo-descarga.ts). */

export interface EntradaZip {
  nombre: string;
  datos: Uint8Array;
}

const FIRMA_LOCAL = 0x04034b50;
const FIRMA_CENTRAL = 0x02014b50;
const FIRMA_FIN = 0x06054b50;
/* Bit 11: nombres en UTF-8 (acentos en los nombres de archivo). */
const FLAGS_UTF8 = 0x0800;

const TABLA_CRC: Uint32Array = (() => {
  const tabla = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    tabla[n] = c >>> 0;
  }
  return tabla;
})();

/** CRC-32 (polinomio IEEE) que exige la cabecera de cada entrada del ZIP. */
export function crc32(datos: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of datos) c = TABLA_CRC[(c ^ b) & 0xff]! ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/* Fecha y hora en formato MS-DOS, las que lleva el ZIP. */
function fechaDos(d: Date): { hora: number; fecha: number } {
  return {
    hora: (d.getHours() << 11) | (d.getMinutes() << 5) | Math.floor(d.getSeconds() / 2),
    fecha: ((d.getFullYear() - 1980) << 9) | ((d.getMonth() + 1) << 5) | d.getDate(),
  };
}

function concatenar(partes: Uint8Array[]): Uint8Array<ArrayBuffer> {
  const total = partes.reduce((suma, p) => suma + p.length, 0);
  const salida = new Uint8Array(total);
  let pos = 0;
  for (const p of partes) {
    salida.set(p, pos);
    pos += p.length;
  }
  return salida;
}

/* Un ZIP en memoria: por cada entrada, cabecera local + datos; al final, el
 * índice central con el desplazamiento de cada una y el registro de cierre. */
export function crearZip(entradas: EntradaZip[], ahora: Date = new Date()): Uint8Array<ArrayBuffer> {
  const enc = new TextEncoder();
  const { hora, fecha } = fechaDos(ahora);
  const locales: Uint8Array[] = [];
  const centrales: Uint8Array[] = [];
  let offset = 0;
  for (const e of entradas) {
    const nombre = enc.encode(e.nombre);
    const crc = crc32(e.datos);
    const tam = e.datos.length;

    const local = new DataView(new ArrayBuffer(30));
    local.setUint32(0, FIRMA_LOCAL, true);
    local.setUint16(4, 20, true); // versión necesaria para extraer
    local.setUint16(6, FLAGS_UTF8, true);
    // Método 0 (STORE) en el desplazamiento 8: ya viene a cero.
    local.setUint16(10, hora, true);
    local.setUint16(12, fecha, true);
    local.setUint32(14, crc, true);
    local.setUint32(18, tam, true);
    local.setUint32(22, tam, true);
    local.setUint16(26, nombre.length, true);
    locales.push(new Uint8Array(local.buffer), nombre, e.datos);

    const central = new DataView(new ArrayBuffer(46));
    central.setUint32(0, FIRMA_CENTRAL, true);
    central.setUint16(4, 20, true); // versión con la que se creó
    central.setUint16(6, 20, true); // versión necesaria para extraer
    central.setUint16(8, FLAGS_UTF8, true);
    central.setUint16(12, hora, true);
    central.setUint16(14, fecha, true);
    central.setUint32(16, crc, true);
    central.setUint32(20, tam, true);
    central.setUint32(24, tam, true);
    central.setUint16(28, nombre.length, true);
    central.setUint32(42, offset, true); // dónde empieza la cabecera local
    centrales.push(new Uint8Array(central.buffer), nombre);

    offset += 30 + nombre.length + tam;
  }
  const tamCentral = centrales.reduce((suma, p) => suma + p.length, 0);
  const fin = new DataView(new ArrayBuffer(22));
  fin.setUint32(0, FIRMA_FIN, true);
  fin.setUint16(8, entradas.length, true);
  fin.setUint16(10, entradas.length, true);
  fin.setUint32(12, tamCentral, true);
  fin.setUint32(16, offset, true);
  return concatenar([...locales, ...centrales, new Uint8Array(fin.buffer)]);
}
