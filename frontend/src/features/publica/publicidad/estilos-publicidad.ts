import { CLASE_BORDE, CLASE_HOVER_CAJA, CLASE_RELLENO_SUAVE } from '../disenno';

/* Botón de la página /publicidad: caja con borde de tinta, sin redondeados ni sombras. */
export const BOTON_PUBLICIDAD = `border ${CLASE_BORDE} bg-transparent px-3 py-1 text-black ${CLASE_HOVER_CAJA} disabled:opacity-50`;
export const BOTON_PUBLICIDAD_ACTIVO = `border ${CLASE_BORDE} ${CLASE_RELLENO_SUAVE} px-3 py-1 text-black`;
export const CAJA_PUBLICIDAD = `border ${CLASE_BORDE} p-4`;
