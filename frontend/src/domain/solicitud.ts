/* [169A-2] Solicitud "Publicar mi inmueble": lo que el visitante deja
 * (entra en `pendiente`) y el borrador local que sobrevive al cierre.
 * El precio viaja como texto en el formulario (vacío = sin estimar) y
 * solo se convierte a número al enviar. Las fotos ya subidas viajan
 * como claves `solicitudes/<uuid>/<uuid>.<ext>` generadas por el servidor. */

export type OperacionSolicitud = 'venta' | 'alquiler';

export interface BorradorSolicitud {
  nombre: string;
  telefono: string;
  email: string;
  descripcion: string;
  ubicacion: string;
  residencia: string;
  puestos: string;
  precioEstimado: string;
  operacion: OperacionSolicitud;
  fotosClaves: string[];
}

export const BORRADOR_SOLICITUD_VACIO: BorradorSolicitud = {
  nombre: '',
  telefono: '',
  email: '',
  descripcion: '',
  ubicacion: '',
  residencia: '',
  puestos: '',
  precioEstimado: '',
  operacion: 'venta',
  fotosClaves: [],
};

export const MAX_FOTOS_SOLICITUD = 10;

export function borradorSolicitudTieneContenido(b: BorradorSolicitud): boolean {
  return (
    b.nombre.trim() !== '' ||
    b.telefono.trim() !== '' ||
    b.email.trim() !== '' ||
    b.descripcion.trim() !== '' ||
    b.ubicacion.trim() !== '' ||
    b.residencia.trim() !== '' ||
    b.puestos.trim() !== '' ||
    b.precioEstimado.trim() !== '' ||
    b.fotosClaves.length > 0
  );
}

/* Validación de formato básica en el boundary (el servidor re-valida):
 * nombre obligatorio, teléfono 6-15 dígitos, email con formato si viene,
 * precio no negativo si viene. Devuelve el primer problema o null. */
export function validarBorradorSolicitud(b: BorradorSolicitud): string | null {
  if (b.nombre.trim() === '') return 'Cuéntanos tu nombre para poder contactarte.';
  const digitos = b.telefono.replace(/\D/g, '');
  if (digitos.length < 6 || digitos.length > 15)
    return 'El teléfono debe tener entre 6 y 15 dígitos.';
  if (b.email.trim() !== '' && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(b.email.trim()))
    return 'Ese correo no parece válido (puedes dejarlo vacío).';
  if (b.puestos.trim() !== '') {
    const puestos = Number(b.puestos.replace(',', '.'));
    if (!Number.isInteger(puestos) || puestos < 0)
      return 'Los puestos deben ser un número entero positivo (o vacío).';
  }
  if (b.precioEstimado.trim() !== '') {
    const precio = Number(b.precioEstimado.replace(',', '.'));
    if (!Number.isFinite(precio) || precio < 0)
      return 'El precio estimado debe ser un número positivo (o vacío).';
  }
  if (b.fotosClaves.length > MAX_FOTOS_SOLICITUD)
    return `Puedes adjuntar ${MAX_FOTOS_SOLICITUD} fotos como máximo.`;
  return null;
}

export function precioAEnviar(texto: string): number | null {
  const limpio = texto.trim().replace(',', '.');
  if (limpio === '') return null;
  const n = Number(limpio);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

/* Puestos a enviar: entero >= 0; vacío = 0 (el servidor los admite sin indicar). */
export function puestosAEnviar(texto: string): number {
  const limpio = texto.trim().replace(',', '.');
  if (limpio === '') return 0;
  const n = Number(limpio);
  return Number.isInteger(n) && n >= 0 ? n : 0;
}
