// Servicio de imágenes desacoplado del input de ficheros: mañana la app móvil
// implementará pickImages() con la cámara/galería nativa sin cambiar el modal.
// Sin límite de fotos por inmueble: el tope real lo pone el almacenamiento local.
// El DOM/Canvas vive en `platform/imagen` (Sentinel dom-access-outside-platform).
import { cargarImagenArchivo, reescalarAJpeg } from '../../platform/imagen';

/** Comprime una imagen a JPEG dataURL (máx 1280px, calidad 0.8). */
export async function comprimirImagen(file: File, ladoMax = 1280, calidad = 0.8): Promise<string> {
  const img = await cargarImagenArchivo(file);
  const escala = Math.min(1, ladoMax / Math.max(img.width, img.height));
  const w = Math.max(1, Math.round(img.width * escala));
  const h = Math.max(1, Math.round(img.height * escala));
  return reescalarAJpeg(img, w, h, calidad);
}

export async function ficherosADataUrls(files: FileList | File[]): Promise<{ urls: string[]; errores: string[] }> {
  const urls: string[] = [];
  const errores: string[] = [];
  for (const file of Array.from(files)) {
    if (!file.type.startsWith('image/')) {
      errores.push(`${file.name} no es una imagen.`);
      continue;
    }
    try {
      urls.push(await comprimirImagen(file));
    } catch (error) {
      errores.push(error instanceof Error ? error.message : `Error con ${file.name}.`);
    }
  }
  return { urls, errores };
}
