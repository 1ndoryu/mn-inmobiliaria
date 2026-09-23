/* Canal de WhatsApp para el contacto público.
 * El chat con IA solo vive en local (desarrollo); en producción el botón
 * Mensaje y el botón "Me interesa esta propiedad" abren wa.me con el
 * número de la inmobiliaria y un mensaje prediseñado. En móvil wa.me
 * abre la app de WhatsApp (si está instalada) con el texto ya escrito;
 * en escritorio abre WhatsApp Web. */

export const NUMERO_WHATSAPP = '584249208855';

/* Verdadero solo en desarrollo: `vite dev` o nombre local. Cualquier
 * dominio real (producción) devuelve falso y desactiva el chat IA. */
export function esEntornoLocal(): boolean {
  if (import.meta.env.DEV) return true;
  const anfitrion = window.location.hostname.toLowerCase();
  return (
    anfitrion === 'localhost' ||
    anfitrion === '127.0.0.1' ||
    anfitrion === '::1' ||
    anfitrion === '[::1]' ||
    anfitrion.endsWith('.local')
  );
}

export function enlaceWhatsApp(mensaje: string): string {
  return `https://wa.me/${NUMERO_WHATSAPP}?text=${encodeURIComponent(mensaje)}`;
}

export function mensajeGeneral(): string {
  return 'Hola, quiero información sobre las propiedades publicadas en MN Inmobiliaria.';
}

export function mensajePropiedad(
  titulo: string,
  ubicacion: string,
  precioTexto: string,
  operacion: string,
): string {
  const lugar = ubicacion.trim() ? ` (${ubicacion.trim()})` : '';
  return `Hola, me interesa esta propiedad: ${titulo}${lugar}. Precio: ${precioTexto} (${operacion}).`;
}
