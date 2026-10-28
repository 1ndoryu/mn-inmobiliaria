// Pestaña activa del panel de gestión: se recuerda entre recargas y de ella
// salen los textos de la cabecera. Sin almacenamiento, todo vive en memoria.
import type { VistaApp } from './layout';

const CLAVE = 'admin:vista';
const VISTAS: VistaApp[] = ['inmuebles', 'imagenes', 'mensajes', 'publicidad', 'solicitudes', 'calendario'];

/* La pestaña sobrevive a recargas; sin almacenamiento se arranca en inmuebles. */
export function leerVistaGuardada(): VistaApp {
  try {
    const guardada = localStorage.getItem(CLAVE);
    if (VISTAS.includes(guardada as VistaApp)) return guardada as VistaApp;
  } catch {
    // Sin almacenamiento se arranca en inmuebles; no bloquea.
  }
  return 'inmuebles';
}

export function guardarVista(vista: VistaApp): void {
  try {
    localStorage.setItem(CLAVE, vista);
  } catch {
    // Sin almacenamiento la pestaña solo vive en memoria; no bloquea.
  }
}

const TITULOS: Partial<Record<VistaApp, string>> = {
  imagenes: 'Imágenes',
  mensajes: 'Mensajes',
  publicidad: 'Publicidad',
  solicitudes: 'Solicitudes',
  calendario: 'Calendario',
};

export function tituloVista(vista: VistaApp): string {
  return TITULOS[vista] ?? 'Inmuebles';
}

/* Subtítulo con el recuento de la vista activa (fotos o inmuebles). */
export function descripcionVista(vista: VistaApp, fotos: number, total: number): string {
  if (vista === 'imagenes') return `${fotos} foto${fotos === 1 ? '' : 's'} con copia original intacta`;
  if (vista === 'mensajes') return 'Conversaciones del chat con IA';
  if (vista === 'publicidad') return 'Imágenes listas para redes';
  if (vista === 'solicitudes') return 'Propuestas de «Publicar mi inmueble» para revisar';
  if (vista === 'calendario') return 'Publicaciones en redes por día y por mes';
  if (total === 0) return 'Registra tu primer inmueble';
  return `${total} inmueble${total === 1 ? '' : 's'} en el servidor`;
}
