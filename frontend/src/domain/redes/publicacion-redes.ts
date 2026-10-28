/* [28AA-6] Calendario de publicaciones en redes: una publicación por inmueble
 * y día, con cuatro destinos que deben estar todos marcados para quedar
 * `publicada`. El estado lo fija el servidor; aquí solo se muestra y se edita. */

export const ESTADOS_PUBLICACION = ['borrador', 'lista', 'publicada', 'saltada'] as const;
export type EstadoPublicacion = (typeof ESTADOS_PUBLICACION)[number];

/* Mismos cuatro destinos que el servidor (`DESTINOS`): el orden es el del checklist. */
export const DESTINOS_PUBLICACION = ['igPost', 'fanpage', 'igHistoria', 'fbPerfil'] as const;
export type DestinoPublicacion = (typeof DESTINOS_PUBLICACION)[number];

export const ETIQUETA_DESTINO: Record<DestinoPublicacion, string> = {
  igPost: 'Instagram · post',
  fanpage: 'Fanpage de Facebook',
  igHistoria: 'Instagram · historia',
  fbPerfil: 'Facebook · perfil',
};

export type Destinos = Record<DestinoPublicacion, boolean>;

export const DESTINOS_VACIOS: Destinos = {
  igPost: false,
  fanpage: false,
  igHistoria: false,
  fbPerfil: false,
};

/* Formatos de pieza que acepta el servidor (proporción con 2 % de tolerancia). */
export const FORMATOS_PIEZA = ['1:1', '3:4', '4:5', '9:16'] as const;
export type FormatoPieza = (typeof FORMATOS_PIEZA)[number];

export interface Pieza {
  id: string;
  formato: FormatoPieza;
  /* Ruta relativa a la API (`/uploads/…`). */
  url: string;
  ancho: number;
  alto: number;
  bytes: number;
}

export interface Publicacion {
  id: string;
  inmuebleId: string;
  tituloInmueble: string;
  estado: EstadoPublicacion;
  descripcion: string;
  tags: string[];
  /* Vacío si la descripción se escribió a mano. */
  modelo: string;
  destinos: Destinos;
  /* Día previsto (`YYYY-MM-DD`); vacío = sin fecha prevista. */
  programadaPara: string | null;
  /* Instante real de salida (ISO); solo existe si `estado === 'publicada'`. */
  publicadaEn: string | null;
  piezas: Pieza[];
  creadaEn: string;
}

/* Propuesta de la cola: el siguiente inmueble que toca publicar. */
export interface Propuesta {
  inmuebleId: string;
  titulo: string;
}

/* Todos los destinos marcados: único requisito para pasar a `publicada`. */
export function destinosCompletos(d: Destinos): boolean {
  return DESTINOS_PUBLICACION.every((k) => d[k]);
}

/* Cuántos destinos faltan, para mostrar «faltan 2 de 4» sin recalcular en cada vista. */
export function destinosPendientes(d: Destinos): number {
  return DESTINOS_PUBLICACION.filter((k) => !d[k]).length;
}

/* Publicación ya salida que se muestra en /publicidad (ver `PublicidadPublica` del servidor). */
export interface PublicidadPublicaItem {
  inmuebleId: string;
  titulo: string;
  slug: string;
  fotos: string[];
  descripcion: string;
  tags: string[];
  piezas: Pieza[];
  publicadaEn: string;
}
