import { crearZip, type EntradaZip } from '../domain/zip-almacenado';
import { cargarImagen } from './canvas-publicidad';
import { descargarBlob } from './descarga';

/* [28AA-8] Foto de publicidad con el logo centrado abajo, para descargarla.
 * Se dibuja en un canvas y sale como JPEG: la foto original del servidor no
 * se toca. Solo se usa al descargar; la web muestra las fotos sin logo. */

/* Solo el dibujo blanco, sin el cuadro naranja: se ve sobre la foto y es más pequeño. */
const RUTA_LOGO = '/img/logo-mn-blanco.svg';
/* Logo a 7 % del ancho de la foto, a 3 % del borde inferior. */
const FRACCION_LOGO = 0.07;
const FRACCION_MARGEN = 0.03;

/** La foto con el logo abajo, como JPEG. */
export async function fotoConLogo(url: string): Promise<Blob> {
  const [foto, logo] = await Promise.all([cargarImagen(url), cargarImagen(RUTA_LOGO)]);
  const canvas = document.createElement('canvas');
  canvas.width = foto.naturalWidth;
  canvas.height = foto.naturalHeight;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('El navegador no permite dibujar la foto.');
  ctx.drawImage(foto, 0, 0);
  dibujarLogoAbajo(ctx, logo, canvas.width, canvas.height);
  return aBlobJpeg(canvas);
}

export async function descargarFotoConLogo(url: string, nombre: string): Promise<void> {
  descargarBlob(await fotoConLogo(url), nombre);
}

/* Todas las fotos en un único ZIP. Va en serie: así no hay varias fotos
 * decodificadas a la vez en memoria. */
export async function descargarFotosZip(urls: string[], nombreZip: string): Promise<void> {
  const entradas: EntradaZip[] = [];
  for (const [i, url] of urls.entries()) {
    const jpeg = await fotoConLogo(url);
    entradas.push({ nombre: `${String(i + 1).padStart(2, '0')}.jpg`, datos: new Uint8Array(await jpeg.arrayBuffer()) });
  }
  descargarBlob(new Blob([crearZip(entradas)], { type: 'application/zip' }), nombreZip);
}

function aBlobJpeg(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error('No se pudo codificar la foto.'))),
      'image/jpeg',
      0.9,
    );
  });
}

function dibujarLogoAbajo(
  ctx: CanvasRenderingContext2D,
  logo: HTMLImageElement,
  ancho: number,
  alto: number,
): void {
  const anchoLogo = ancho * FRACCION_LOGO;
  const altoLogo = anchoLogo * (logo.naturalHeight / logo.naturalWidth);
  const x = (ancho - anchoLogo) / 2;
  const y = alto - altoLogo - alto * FRACCION_MARGEN;
  ctx.drawImage(logo, x, y, anchoLogo, altoLogo);
}
