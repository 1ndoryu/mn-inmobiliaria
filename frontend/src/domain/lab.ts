/* Estado de la conexión del lab (opencode-propio-dev) con el backend [10AA-23].
 * Puro: el backend ya clasifica el latido del token CLI; aquí solo los tipos. */

export type ConexionLab = 'cargando' | 'sin-leer' | 'conectado' | 'sin-senal' | 'rechazado';

export interface ResumenLab {
  conexion: ConexionLab;
  /* Segundos desde el último latido aceptado; null si nunca desde que arrancó el backend. */
  ultimoOkHaceS: number | null;
}
