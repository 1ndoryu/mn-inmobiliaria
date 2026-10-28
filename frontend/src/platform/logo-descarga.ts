import { cargarImagen } from './canvas-publicidad';
import { descargarDataUrl } from './descarga';

/* [28AA-8] Foto de publicidad con el logo centrado abajo, para descargarla.
 * Se dibuja en un canvas y sale como JPEG: la foto original del servidor no
 * se toca. Solo se usa al descargar; la web muestra las fotos sin logo. */

const RUTA_LOGO = '/img/logo-mn.svg';
/* Logo a 16 % del ancho de la foto, a 3 % del borde inferior. */
const FRACCION_LOGO = 0.16;
const FRACCION_MARGEN = 0.03;

export async function descargarFotoConLogo(url: string, nombre: string): Promise<void> {
  const [foto, logo] = await Promise.all([cargarImagen(url), cargarImagen(RUTA_LOGO)]);
  const canvas = document.createElement('canvas');
  canvas.width = foto.naturalWidth;
  canvas.height = foto.naturalHeight;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('El navegador no permite dibujar la foto.');
  ctx.drawImage(foto, 0, 0);
  dibujarLogoAbajo(ctx, logo, canvas.width, canvas.height);
  descargarDataUrl(canvas.toDataURL('image/jpeg', 0.9), nombre);
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
