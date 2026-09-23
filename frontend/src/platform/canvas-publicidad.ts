import {
  COLOR_ACENTO,
  COLOR_FONDO_CLARO,
  COLOR_TINTA,
  LEYENDA_PRECIO,
  PLANTILLA_ALTO,
  PLANTILLA_ANCHO,
  PRESETS_EXPORTACION,
  TELEFONO_PUBLICIDAD,
  WEB_PUBLICIDAD,
  lineasTituloDe,
  precioCortoDe,
  specsVisiblesDe,
  type ComposicionPublicidad,
  type FormatoPublicidad,
  type IconoSpec,
} from '../domain/plantilla-publicidad';
import type { Inmueble } from '../domain/inmueble';

// Renderer Canvas 2D de la imagen publicitaria (plantilla "Post I").
// Dibuja en unidades de plantilla (197x262) con el contexto escalado, así el
// mismo código sirve para la vista previa y la exportación en alta resolución.

const FUENTE = `'Inter', 'Söhne', 'Helvetica Neue', Arial, system-ui, sans-serif`;

interface IconoPlantilla {
  d: string;
  x0: number;
  y0: number;
  w: number;
  h: number;
  trazo: number;
  relleno: boolean;
}

// Trazos copiados del SVG del Figma (coordenadas originales).
const ICONO_CAMA: IconoPlantilla = {
  d: 'M11.6818 12.3046V10.3886C11.6818 10.2615 11.7322 10.1397 11.8221 10.0499C11.9119 9.96004 12.0337 9.90957 12.1608 9.90957M12.1608 9.90957H15.9928M12.1608 9.90957V8.95157C12.1608 8.82453 12.2112 8.70269 12.3011 8.61286C12.3909 8.52303 12.5127 8.47256 12.6398 8.47256H15.5138C15.6408 8.47256 15.7627 8.52303 15.8525 8.61286C15.9423 8.70269 15.9928 8.82453 15.9928 8.95157V9.90957M15.9928 9.90957C16.1198 9.90957 16.2417 9.96004 16.3315 10.0499C16.4213 10.1397 16.4718 10.2615 16.4718 10.3886V12.3046M14.0768 8.47256V9.90957M11.6818 11.8256H16.4718',
  x0: 11.68, y0: 8.47, w: 4.79, h: 3.83, trazo: 0.446, relleno: false,
};
const ICONO_BANO: IconoPlantilla = {
  d: 'M28.8091 8.47299L28.3301 8.952M30.4856 12.0655V12.5445M26.8931 10.389H31.6832M28.0906 12.0655V12.5445M28.5696 8.7125L28.2394 8.38222C28.1701 8.31264 28.0822 8.26462 27.9862 8.24397C27.8902 8.22332 27.7903 8.23092 27.6985 8.26587C27.6068 8.30081 27.5271 8.36159 27.4692 8.44085C27.4113 8.52012 27.3776 8.61447 27.3721 8.7125V11.5865C27.3721 11.7136 27.4226 11.8354 27.5124 11.9252C27.6023 12.015 27.7241 12.0655 27.8511 12.0655H30.7251C30.8522 12.0655 30.974 12.015 31.0639 11.9252C31.1537 11.8354 31.2042 11.7136 31.2042 11.5865V10.389',
  x0: 26.89, y0: 8.22, w: 4.79, h: 4.32, trazo: 0.446, relleno: false,
};
// Carro (puestos) y pin (ubicación) del Figma "Post con Ubicacion y
// puestos de estacionamiento" (banda y 8.06–12.73, naranja #FEA429).
// El carro son 6 subpaths de relleno (cabina + carrocería + manijas +
// ruedas); el pin 2 subpaths de trazo (exterior + interior).
const ICONO_CARRO: IconoPlantilla = {
  d: 'M45.8583 8.05656C46.009 8.05833 46.1509 8.11549 46.2655 8.21173C46.3779 8.30633 46.4592 8.4347 46.5043 8.57599L46.7277 9.23738L46.9401 8.98317C47.0198 8.88772 47.1615 8.87492 47.257 8.95455C47.3526 9.03424 47.3653 9.17588 47.2856 9.27149L46.8168 9.83494C46.7646 9.89758 46.6824 9.92687 46.6022 9.91197C46.5221 9.89702 46.4556 9.8406 46.4295 9.7634L46.0773 8.72015L46.0762 8.71575C46.0545 8.64614 46.0174 8.59219 45.9761 8.55728C45.9556 8.54004 45.9341 8.52781 45.9133 8.51987L45.8528 8.50776H44.1482C44.1091 8.50678 44.066 8.5192 44.0249 8.55178C43.9829 8.58509 43.9446 8.63854 43.9215 8.70805L43.5704 9.7623C43.5446 9.83967 43.4789 9.89668 43.3988 9.91197C43.3184 9.92713 43.2354 9.89774 43.1831 9.83494L42.7143 9.27149C42.6346 9.17588 42.6473 9.03424 42.7429 8.95455C42.8384 8.87492 42.9801 8.88772 43.0598 8.98317L43.27 9.23518L43.4945 8.56498C43.5424 8.42136 43.6278 8.29174 43.7454 8.19853C43.8622 8.10604 44.005 8.05504 44.1548 8.05766V8.05656H45.8583Z M43.8295 10.5922C43.9537 10.5925 44.054 10.6935 44.054 10.8178C44.054 10.9421 43.9537 11.0431 43.8295 11.0434H43.8262C43.7017 11.0434 43.6006 10.9423 43.6006 10.8178C43.6006 10.6933 43.7017 10.5922 43.8262 10.5922H43.8295Z M46.1771 10.5922C46.3014 10.5925 46.4016 10.6935 46.4016 10.8178C46.4016 10.9421 46.3014 11.0431 46.1771 11.0434H46.1738C46.0494 11.0434 45.9482 10.9423 45.9482 10.8178C45.9482 10.6933 46.0494 10.5922 46.1738 10.5922H46.1771Z M46.8873 10.2544C46.8873 10.0274 46.7411 9.91653 46.643 9.91653H43.3569C43.2588 9.91653 43.1126 10.0274 43.1126 10.2544V11.3813C43.1126 11.6082 43.2588 11.7191 43.3569 11.7191H46.643C46.7411 11.7191 46.8873 11.6082 46.8873 11.3813V10.2544ZM47.3385 11.3813C47.3385 11.7767 47.0635 12.1703 46.643 12.1703H43.3569C42.9364 12.1703 42.6614 11.7767 42.6614 11.3813V10.2544C42.6614 9.85894 42.9364 9.46533 43.3569 9.46533H46.643C47.0635 9.46533 47.3385 9.85894 47.3385 10.2544V11.3813Z M43.131 12.5082V11.9447C43.131 11.8202 43.2322 11.7191 43.3566 11.7191C43.4811 11.7191 43.5822 11.8202 43.5822 11.9447V12.5082C43.5822 12.6326 43.4811 12.7338 43.3566 12.7338C43.2322 12.7338 43.131 12.6326 43.131 12.5082Z M46.4178 12.5082V11.9447C46.4178 11.8202 46.5189 11.7191 46.6434 11.7191C46.7679 11.7191 46.869 11.8202 46.869 11.9447V12.5082C46.869 12.6326 46.7679 12.7338 46.6434 12.7338C46.5189 12.7338 46.4178 12.6326 46.4178 12.5082Z',
  x0: 42.6614, y0: 8.05656, w: 4.6771, h: 4.6772, trazo: 0, relleno: true,
};
// Regla (ruler de lucide 24x24: cuerpo diagonal + 4 marcas) para los m².
// Mismo trazo redondo que el resto de la banda.
const ICONO_METROS: IconoPlantilla = {
  d: 'M21.3 15.3a2.4 2.4 0 0 1 0 3.4l-2.6 2.6a2.4 2.4 0 0 1-3.4 0L2.7 8.7a2.41 2.41 0 0 1 0-3.4l2.6-2.6a2.41 2.41 0 0 1 3.4 0Z M14.5 12.5l2-2 M11.5 9.5l2-2 M8.5 6.5l2-2 M17.5 15.5l2-2',
  x0: 0, y0: 0, w: 24, h: 24, trazo: 2, relleno: false,
};
const ICONO_UBICACION: IconoPlantilla = {
  d: 'M58.2774 12.6868C58.7124 12.3112 60.0078 11.0951 60.0078 9.92746C60.0078 9.43127 59.8107 8.9554 59.4598 8.60454C59.109 8.25368 58.6331 8.05656 58.1369 8.05656C57.6407 8.05656 57.1648 8.25368 56.814 8.60454C56.4631 8.9554 56.266 9.43127 56.266 9.92746C56.266 11.0951 57.5614 12.3112 57.9963 12.6868C58.0369 12.7173 58.0862 12.7337 58.1369 12.7337C58.1876 12.7337 58.2369 12.7173 58.2774 12.6868Z M58.1369 10.629C58.5244 10.629 58.8385 10.3149 58.8385 9.92746C58.8385 9.53999 58.5244 9.22588 58.1369 9.22588C57.7494 9.22588 57.4353 9.53999 57.4353 9.92746C57.4353 10.3149 57.7494 10.629 58.1369 10.629Z',
  x0: 56.266, y0: 8.05656, w: 3.7418, h: 4.6771, trazo: 0.4, relleno: false,
};
const ICONO_TELEFONO: IconoPlantilla = {
  d: 'M6.29832 257.053C6.33264 257.069 6.3713 257.072 6.40793 257.063C6.44457 257.054 6.47699 257.033 6.49987 257.003L6.55886 256.925C6.58981 256.884 6.62995 256.851 6.67609 256.827C6.72224 256.804 6.77312 256.792 6.82471 256.792H7.32318C7.41132 256.792 7.49584 256.827 7.55817 256.89C7.62049 256.952 7.6555 257.037 7.6555 257.125V257.623C7.6555 257.711 7.62049 257.796 7.55817 257.858C7.49584 257.92 7.41132 257.956 7.32318 257.956C6.52996 257.956 5.76923 257.64 5.20833 257.08C4.64744 256.519 4.33234 255.758 4.33234 254.965C4.33234 254.877 4.36735 254.792 4.42967 254.73C4.49199 254.667 4.57652 254.632 4.66465 254.632H5.16313C5.25126 254.632 5.33579 254.667 5.39811 254.73C5.46043 254.792 5.49544 254.877 5.49544 254.965V255.463C5.49544 255.515 5.48343 255.566 5.46036 255.612C5.43729 255.658 5.40379 255.698 5.36252 255.729L5.28475 255.787C5.25425 255.811 5.23275 255.844 5.22391 255.881C5.21506 255.918 5.21942 255.958 5.23624 255.992C5.46332 256.453 5.8368 256.826 6.29832 257.053Z',
  x0: 4.33, y0: 254.63, w: 3.33, h: 3.33, trazo: 0, relleno: true,
};
const ICONO_GLOBO: IconoPlantilla = {
  d: 'M48.8651 258.067C49.8435 258.067 50.6366 257.274 50.6366 256.295C50.6366 255.317 49.8435 254.524 48.8651 254.524C47.8867 254.524 47.0935 255.317 47.0935 256.295C47.0935 257.274 47.8867 258.067 48.8651 258.067ZM48.1564 256.295C48.1564 255.636 48.4102 255.001 48.8651 254.524C49.32 255.001 49.5737 255.636 49.5737 256.295C49.5737 256.955 49.32 257.589 48.8651 258.067C48.4102 257.589 48.1564 256.955 48.1564 256.295ZM47.0935 256.295H50.6366',
  x0: 47.09, y0: 254.52, w: 3.55, h: 3.55, trazo: 0.367, relleno: false,
};

const PIE_CURVO_D =
  'M59.6571 251.067H-2.89508V265.451H203.494V220.627C203.494 220.627 169.112 216.109 149.304 222.3C129.496 228.49 115.451 251.067 115.451 251.067H59.6571Z';

const ICONOS_SPECS: Record<IconoSpec, IconoPlantilla> = {
  habitaciones: ICONO_CAMA,
  banos: ICONO_BANO,
  metros: ICONO_METROS,
  puestos: ICONO_CARRO,
  ubicacion: ICONO_UBICACION,
};

export function altoUnidadesDe(formato: FormatoPublicidad): number {
  if (formato === 'post-4-5') return (PLANTILLA_ANCHO * 1350) / 1080;
  // Cuadrado: el bloque inferior se ancla abajo (translate negativo) y la
  // franja de foto central se recorta; el resto del dibujo no cambia.
  if (formato === 'cuadrado-1-1') return PLANTILLA_ANCHO;
  return PLANTILLA_ALTO;
}

const imagenes = new Map<string, Promise<HTMLImageElement>>();

/** Carga por fetch+blob para no manchar el canvas (las fotos son mismo origen). */
export function cargarImagen(url: string): Promise<HTMLImageElement> {
  let pendiente = imagenes.get(url);
  if (!pendiente) {
    pendiente = (async () => {
      const respuesta = await fetch(url);
      if (!respuesta.ok) throw new Error(`Foto ${respuesta.status}`);
      const blob = await respuesta.blob();
      const objeto = URL.createObjectURL(blob);
      try {
        const img = new Image();
        img.src = objeto;
        await img.decode();
        return img;
      } finally {
        URL.revokeObjectURL(objeto);
      }
    })();
    imagenes.set(url, pendiente);
  }
  return pendiente;
}

function dibujarCover(
  ctx: CanvasRenderingContext2D,
  img: HTMLImageElement,
  x: number, y: number, w: number, h: number,
): void {
  const escala = Math.max(w / img.naturalWidth, h / img.naturalHeight);
  const sw = w / escala;
  const sh = h / escala;
  const sx = (img.naturalWidth - sw) / 2;
  const sy = (img.naturalHeight - sh) / 2;
  ctx.drawImage(img, sx, sy, sw, sh, x, y, w, h);
}

function dibujarIcono(
  ctx: CanvasRenderingContext2D,
  icono: IconoPlantilla,
  x: number, y: number, alto: number, color: string,
): number {
  const s = alto / icono.h;
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(s, s);
  ctx.translate(-icono.x0, -icono.y0);
  const trazo = new Path2D(icono.d);
  if (icono.relleno) {
    ctx.fillStyle = color;
    ctx.fill(trazo);
  } else {
    ctx.strokeStyle = color;
    ctx.lineWidth = icono.trazo;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.stroke(trazo);
  }
  ctx.restore();
  return icono.w * s;
}

async function asegurarFuentes(): Promise<void> {
  try {
    await Promise.all([
      document.fonts.load(`700 32px ${FUENTE}`),
      document.fonts.load(`400 32px ${FUENTE}`),
    ]);
  } catch {
    // Sin Inter se usa el relevo del sistema; no bloquea la exportación.
  }
}

function tamanoAjustado(ctx: CanvasRenderingContext2D, texto: string, max: number, base: number): number {
  let t = base;
  ctx.font = `700 ${t}px ${FUENTE}`;
  while (t > 6 && ctx.measureText(texto).width > max) {
    t -= 0.5;
    ctx.font = `700 ${t}px ${FUENTE}`;
  }
  return t;
}

export async function renderizarPublicidad(
  canvas: HTMLCanvasElement,
  inmueble: Inmueble,
  comp: ComposicionPublicidad,
  anchoPx: number,
  vigente?: () => boolean,
): Promise<void> {
  const altoU = altoUnidadesDe(comp.formato);
  const s = anchoPx / PLANTILLA_ANCHO;
  canvas.width = Math.round(anchoPx);
  canvas.height = Math.round(altoU * s);
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Sin contexto 2D');

  await asegurarFuentes();
  const [fondo, grande, mediano] = await Promise.all([
    cargarImagen(comp.fondo),
    cargarImagen(comp.circularGrande),
    cargarImagen(comp.circularMediano),
  ]);
  // Un render anterior (otro formato/receta) que termine tarde no debe
  // pintar sobre el lienzo: el tamaño ya lo fijó el render más reciente.
  if (vigente && !vigente()) return;

  ctx.save();
  ctx.scale(s, s);
  // Remuestreo de máxima calidad: las fotos fuente (~900-1300px) se amplían
  // hasta 2160px en la exportación; 'high' da el reescalado más nítido.
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.fillStyle = COLOR_FONDO_CLARO;
  ctx.fillRect(0, 0, PLANTILLA_ANCHO, altoU);

  dibujarCover(ctx, fondo, 0, 0, PLANTILLA_ANCHO, altoU);

  // Degradado claro arriba (y 0 -> 83.7).
  const degradado = ctx.createLinearGradient(0, 0, 0, 83.67);
  degradado.addColorStop(0, 'rgba(239,239,239,1)');
  degradado.addColorStop(0.45, 'rgba(239,239,239,1)');
  degradado.addColorStop(1, 'rgba(239,239,239,0)');
  ctx.fillStyle = degradado;
  ctx.fillRect(0, 0, PLANTILLA_ANCHO, 84);

  // Specs con valor (cama | baño | m² | carro | pin); el 0 oculta icono y valor.
  // Sin ubicación no hay pin (ver specsVisiblesDe).
  ctx.fillStyle = COLOR_TINTA;
  ctx.textBaseline = 'alphabetic';
  let x = 10;
  const altoIcono = 4.4;
  const baseSpecs = 11.8;
  ctx.font = `700 3.6px ${FUENTE}`;
  for (const spec of specsVisiblesDe(inmueble)) {
    const wIcono = dibujarIcono(ctx, ICONOS_SPECS[spec.icono], x, 8.3, altoIcono, COLOR_ACENTO);
    x += wIcono + 1.8;
    ctx.fillText(spec.texto, x, baseSpecs);
    x += ctx.measureText(spec.texto).width + 4.2;
  }

  // Título en dos líneas, algo más grande y con menos interlineado.
  // La línea 2 puede ser personalizada (toda en tinta) o automática
  // ("en " en tinta + zona en naranja).
  const lineas = lineasTituloDe(inmueble, comp);
  const texto2 = `${lineas.prefijo2}${lineas.zona2}`;
  ctx.font = `700 11.5px ${FUENTE}`;
  const ancho1 = ctx.measureText(lineas.linea1).width;
  const ancho2 = ctx.measureText(texto2).width;
  const t = tamanoAjustado(ctx, ancho1 >= ancho2 ? lineas.linea1 : texto2, 177, 11.5);
  ctx.font = `700 ${t}px ${FUENTE}`;
  ctx.fillStyle = COLOR_TINTA;
  ctx.fillText(lineas.linea1, 10, 30);
  ctx.fillText(lineas.prefijo2, 10, 43.5);
  if (lineas.zona2) {
    ctx.fillStyle = COLOR_ACENTO;
    ctx.fillText(lineas.zona2, 10 + ctx.measureText(lineas.prefijo2).width, 43.5);
  }

  // Pastilla con el precio bajo el título (como el Figma con precio):
  // franja clara que sale del borde izquierdo, cifra en naranja con el $
  // al final pegado al número y leyenda "Listo para firmar" debajo en tinta.
  // Con precio 0 no se dibuja nada (ni pastilla clara, ni cifra, ni leyenda).
  if ((comp.conPrecio ?? true) && inmueble.precio > 0) {
    const precio = precioCortoDe(inmueble.precio);
    ctx.font = `700 7.5px ${FUENTE}`;
    const anchoPrecio = ctx.measureText(precio).width;
    const anchoPastilla = Math.max(46.77, anchoPrecio + 14);
    const pastilla = ctx.createLinearGradient(-1.2, 0, -1.2 + anchoPastilla, 0);
    pastilla.addColorStop(0, 'rgba(239,239,239,1)');
    pastilla.addColorStop(1, 'rgba(255,255,255,0.68)');
    ctx.fillStyle = pastilla;
    ctx.fillRect(-1.2, 94.75, anchoPastilla, 18.34);
    ctx.fillStyle = COLOR_ACENTO;
    ctx.fillText(precio, 4, 103.6);
    // Leyenda ajustada al ancho del Figma (x 4 -> 31.2).
    ctx.fillStyle = COLOR_TINTA;
    const baseLeyenda = 4;
    ctx.font = `700 ${baseLeyenda}px ${FUENTE}`;
    const tamLeyenda = (baseLeyenda * 27.2) / Math.max(1, ctx.measureText(LEYENDA_PRECIO).width);
    ctx.font = `700 ${tamLeyenda}px ${FUENTE}`;
    ctx.fillText(LEYENDA_PRECIO, 4, 106.1 + tamLeyenda * 0.72);
  }

  // Bloque inferior anclado abajo (en 4:5 sube dy unidades).
  ctx.save();
  ctx.translate(0, altoU - PLANTILLA_ALTO);
  ctx.fillStyle = COLOR_FONDO_CLARO;
  ctx.fill(new Path2D(PIE_CURVO_D));

  const dibujarCircular = (
    img: HTMLImageElement, cx: number, cy: number, rx: number, ry: number,
  ): void => {
    ctx.save();
    ctx.beginPath();
    ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
    ctx.clip();
    dibujarCover(ctx, img, cx - rx, cy - ry, rx * 2, ry * 2);
    ctx.restore();
    ctx.save();
    ctx.strokeStyle = COLOR_FONDO_CLARO;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.ellipse(cx, cy, rx - 1, ry - 1, 0, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  };
  dibujarCircular(mediano, 123.16, 238.97, 18.92, 18.92);
  dibujarCircular(grande, 169.11, 229.08, 25.02, 25.61);

  // Franja de contacto: los iconos bajan un poco para alinear su centro
  // óptico con el texto (que no se mueve: sigue en y 258).
  ctx.fillStyle = COLOR_TINTA;
  ctx.font = `700 3.2px ${FUENTE}`;
  dibujarIcono(ctx, ICONO_TELEFONO, 4.2, 254.8, 3.9, COLOR_ACENTO);
  ctx.fillText(TELEFONO_PUBLICIDAD, 10, 258);
  const trasTelefono = 10 + ctx.measureText(TELEFONO_PUBLICIDAD).width + 4;
  const anchoGlobo = dibujarIcono(ctx, ICONO_GLOBO, trasTelefono, 254.8, 3.9, COLOR_ACENTO);
  ctx.fillText(WEB_PUBLICIDAD, trasTelefono + anchoGlobo + 1.6, 258);
  ctx.restore();

  ctx.restore();
}

function blobDe(canvas: HTMLCanvasElement, tipo: 'png' | 'jpeg'): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error('Sin blob'))),
      tipo === 'png' ? 'image/png' : 'image/jpeg',
      0.92,
    );
  });
}

/** Renderiza en alta resolución y dispara la descarga del JPG/PNG. */
export async function exportarPublicidad(
  inmueble: Inmueble,
  comp: ComposicionPublicidad,
  tipo: 'png' | 'jpeg',
): Promise<string> {
  const preset = PRESETS_EXPORTACION.find((p) => p.formato === comp.formato)
    ?? PRESETS_EXPORTACION[0]!;
  const canvas = document.createElement('canvas');
  await renderizarPublicidad(canvas, inmueble, comp, preset.anchoPx);
  const blob = await blobDe(canvas, tipo);
  const url = URL.createObjectURL(blob);
  const nombre = `publicidad-${inmueble.id.slice(0, 8)}-${comp.formato}.${tipo === 'png' ? 'png' : 'jpg'}`;
  const enlace = document.createElement('a');
  enlace.href = url;
  enlace.download = nombre;
  document.body.appendChild(enlace);
  enlace.click();
  enlace.remove();
  URL.revokeObjectURL(url);
  return nombre;
}
