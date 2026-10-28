/* Adaptador de plataforma (DOM/Canvas): carga y reescalado de imágenes.
 * Cualquier acceso directo a `document`, `Image` o `URL` vive aquí, nunca
 * en data. Sentinel (dom-access-outside-platform). */

import type { FormatoPieza } from '../domain/redes/publicacion-redes';

/* Carga un `File` como imagen lista para dibujar (libera su object URL). */
export function cargarImagenArchivo(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error(`No se pudo leer la imagen ${file.name}.`));
    };
    img.src = url;
  });
}

/* Dibuja `img` a `w×h` y la devuelve como JPEG dataURL con `calidad`. */
export function reescalarAJpeg(img: HTMLImageElement, w: number, h: number, calidad: number): string {  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('El navegador no permite procesar imágenes.');
  ctx.drawImage(img, 0, 0, w, h);
  return canvas.toDataURL('image/jpeg', calidad);
}

/* [08AA-23] Lienzo fuera de pantalla para componer (detalle público):
 * `document.createElement` vive aquí, nunca en features. */
export function crearLienzo(): HTMLCanvasElement {
  return document.createElement('canvas');
}

/* [28AA-6] Recorte centrado a la proporción del formato (`'4:5'`…) y JPEG de
 * `anchoMax` px como mucho. El servidor valida la proporción con 2 % de margen,
 * así que el redondeo del alto no hace falta corregirlo aquí. */
export function recortarAFormato(img: HTMLImageElement, formato: FormatoPieza, anchoMax = 1080): Promise<Blob> {
  const [rw, rh] = formato.split(':').map(Number);
  const relacion = rw / rh;
  /* Se recorta la franja sobrante, nunca se estira: la foto conserva sus píxeles. */
  let origenW = img.naturalWidth;
  let origenH = img.naturalHeight;
  if (origenW / origenH > relacion) origenW = origenH * relacion;
  else origenH = origenW / relacion;
  const origenX = (img.naturalWidth - origenW) / 2;
  const origenY = (img.naturalHeight - origenH) / 2;
  const ancho = Math.min(anchoMax, Math.round(origenW));
  const alto = Math.round(ancho / relacion);

  const canvas = crearLienzo();
  canvas.width = ancho;
  canvas.height = alto;
  const ctx = canvas.getContext('2d');
  if (!ctx) return Promise.reject(new Error('El navegador no permite procesar imágenes.'));
  ctx.drawImage(img, origenX, origenY, origenW, origenH, 0, 0, ancho, alto);
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error('No se pudo codificar la imagen.'))),
      'image/jpeg',
      0.9,
    );
  });
}
