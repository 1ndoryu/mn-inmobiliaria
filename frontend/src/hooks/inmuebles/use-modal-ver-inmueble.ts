import { useEffect, useState } from 'react';
import type { Inmueble } from '@/domain/inmueble';
import type { FotoMejora } from '@/domain/foto-mejora';
import { descargarDataUrl, descargarUrl, esperar } from '@/platform/descarga';

/* Estado del visor (3 useState): `indice` | marcha `{descargando, publicando}` | `errorDescarga`. */

/* Nombre de archivo seguro a partir del título. */
export function slugificar(texto: string): string {
  const s = texto
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return s || 'inmueble';
}

/* Extensión para el nombre descargado: del mime en dataURL o de la ruta
 * en URL del servidor; por defecto jpg. */
export function extensionDeFoto(url: string): string {
  if (url.startsWith('data:')) {
    const m = /^data:(image\/[a-zA-Z0-9.+-]+);base64,/.exec(url);
    const mime = m?.[1] ?? '';
    if (mime.includes('png')) return 'png';
    if (mime.includes('webp')) return 'webp';
    return 'jpg';
  }
  const limpio = url.split(/[?#]/)[0];
  const ext = limpio.slice(limpio.lastIndexOf('.') + 1).toLowerCase();
  return ext === 'png' || ext === 'webp' ? ext : 'jpg';
}

/* Búsqueda pura: la mejorada de (inmueble, orden). */
function mejoradaDeLista(fotos: FotoMejora[], inmuebleId: string, orden: number): string | null {
  const f = fotos.find((x) => x.inmuebleId === inmuebleId && x.orden === orden);
  return f?.mejorada ?? null;
}

export function useModalVerInmueble(
  inmueble: Inmueble | null,
  fotosMejora: FotoMejora[],
  onPublicar: (id: string, publicado: boolean) => void,
) {
  const [indice, setIndice] = useState(0);
  const [marcha, setMarcha] = useState({ descargando: false, publicando: false });
  const [errorDescarga, setErrorDescarga] = useState<string | null>(null);

  // Cada inmueble empieza en su primera foto.
  useEffect(() => {
    setIndice(0);
  }, [inmueble?.id]);

  /* Versión mejorada de la foto `orden`, si ya se procesó. Solo guía visual:
   * el original del inmueble nunca se toca. Primero la recién generada en
   * esta sesión (`fotosMejora`) y luego la guardada en el servidor, para que
   * el visor no diga "aún sin mejorar" cuando sí hay mejorada. La descarga
   * vive en el adaptador de plataforma (`platform/descarga`), nunca con DOM
   * directo aquí. */
  function mejoradaDe(orden: number): string | null {
    if (!inmueble) return null;
    return (
      mejoradaDeLista(fotosMejora, inmueble.id, orden) ??
      inmueble.mejoradasServidor.find((m) => m.orden === orden)?.url ??
      null
    );
  }

  /* Descarga una por una (sin zip): la mejorada cuando existe, si no el original. */
  async function descargarTodas() {
    if (!inmueble || marcha.descargando) return;
    setMarcha((m) => ({ ...m, descargando: true }));
    setErrorDescarga(null);
    try {
      const base = slugificar(inmueble.titulo);
      for (let i = 0; i < inmueble.fotos.length; i++) {
        const url = mejoradaDe(i) ?? inmueble.fotos[i];
        const nombre = `${base}-foto-${i + 1}.${extensionDeFoto(url)}`;
        if (url.startsWith('data:')) {
          descargarDataUrl(url, nombre);
        } else {
          await descargarUrl(url, nombre);
        }
        if (i < inmueble.fotos.length - 1) await esperar(600);
      }
    } catch (e) {
      setErrorDescarga(e instanceof Error ? e.message : 'No se pudieron descargar las fotos.');
    } finally {
      setMarcha((m) => ({ ...m, descargando: false }));
    }
  }

  async function cambiarPublicacion() {
    if (!inmueble || marcha.publicando) return;
    setMarcha((m) => ({ ...m, publicando: true }));
    try {
      onPublicar(inmueble.id, !inmueble.publicado);
    } finally {
      setMarcha((m) => ({ ...m, publicando: false }));
    }
  }

  return {
    indice,
    setIndice,
    descargando: marcha.descargando,
    publicando: marcha.publicando,
    errorDescarga,
    mejoradaDe,
    mejoradaActual: mejoradaDe(indice),
    descargarTodas,
    cambiarPublicacion,
  };
}
