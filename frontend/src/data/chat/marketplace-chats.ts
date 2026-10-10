// Chats del asistente Marketplace (07AA-7): hilos con borradores por chat.
// Reutiliza `apiFetch` (JWT + 401); tipos en snake_case como los devuelve
// el backend (`services::marketplace::{ChatResumen, ChatFila}`).

import { apiFetch } from '../inmuebles/api';

/* Fila de `GET /api/admin/marketplace/chats`. */
export interface ChatResumen {
  thread_id: string;
  borradores: number;
  usos: number;
  corregidas: number;
  ultimo: string;
  /* [09AA-19 F7c] Fuente de verdad futura del vínculo (el backend aún no la
   * envía): ausente = sin dato (la vista de huérfanos los muestra todos con
   * aviso). Cuando exista, `true` = hilo con ficha, `false` = huérfano.
   * [09AA-23] El backend ya la envía (siempre presente en filas nuevas);
   * se conserva opcional por compatibilidad con respuestas viejas. */
  aviso_conocido?: boolean;
  /* [09AA-23] Título del inmueble vinculado (ID exacto o título emparejado);
   * ausente/null = huérfano («Sin ficha»). */
  inmueble_vinculado?: string | null;
  /* [09AA-28] URL pública (`/uploads/…`, relativa a la API) de la portada del
   * inmueble vinculado; ausente/null = sin vínculo o ficha sin fotos. */
  inmueble_foto?: string | null;
}

/* Fila de `GET /api/admin/marketplace/chats/:thread`.
 * `origen`: `"ia"` | `"releer"` | `null` (null = fila anterior, origen desconocido).
 * `coste`: tokens y ms de la generación IA original; cada campo null si no se midió. */
export interface ChatFila {
  excerpt_texto: string;
  respuesta: string;
  usos: number;
  corregida: boolean;
  valida_hasta: string;
  origen: string | null;
  coste: { tokens_entrada: number | null; tokens_salida: number | null; ms: number | null };
}

/* [09AA-31] Una página del panel. `hay_mas`: queda otra tras `chats`. */
export interface PaginaChats {
  chats: ChatResumen[];
  total: number;
  hay_mas: boolean;
}

/* [09AA-31] `cursor` = última fila ya vista (paginación keyset, sin OFFSET).
 * [10AA-4] `soloHuerfanos`: el backend filtra los hilos con ficha conocida. */
export function listarChats({
  limite = 25,
  cursor,
  soloHuerfanos = false,
}: { limite?: number; cursor?: ChatResumen; soloHuerfanos?: boolean } = {}): Promise<PaginaChats> {
  const q = new URLSearchParams({ limite: String(limite) });
  if (cursor) {
    q.set('antes_ultimo', cursor.ultimo);
    q.set('antes_hilo', cursor.thread_id);
  }
  if (soloHuerfanos) q.set('solo_huerfanos', 'true');
  return apiFetch<PaginaChats>(`/api/admin/marketplace/chats?${q.toString()}`);
}

export function leerChat(thread: string): Promise<ChatFila[]> {
  return apiFetch<ChatFila[]>(`/api/admin/marketplace/chats/${encodeURIComponent(thread)}`);
}

/* [08AA-39] Limpieza total del panel: borra toda la caché de borradores.
 * [09AA-30] Sin botón en el panel: queda como cliente del endpoint admin
 * `DELETE /chats`. */
export function limpiarChats(): Promise<{ borrados: number }> {
  return apiFetch<{ borrados: number }>('/api/admin/marketplace/chats', { method: 'DELETE' });
}

/* [09AA-30] Menú de tres puntos por conversación: clientes de los endpoints
 * admin `/chats/:thread/*`. `archivarChat` solo oculta el hilo de la lista;
 * `borrarChat` quita la conversación entera; `borrarBorradorChat` quita los
 * borradores no corregidos del hilo. */
export function archivarChat(thread: string): Promise<{ archivado: boolean }> {
  return apiFetch<{ archivado: boolean }>(
    `/api/admin/marketplace/chats/${encodeURIComponent(thread)}/archivar`,
    { method: 'POST' },
  );
}

export function borrarChat(thread: string): Promise<{ borrados: number }> {
  return apiFetch<{ borrados: number }>(
    `/api/admin/marketplace/chats/${encodeURIComponent(thread)}`,
    { method: 'DELETE' },
  );
}

export function borrarBorradorChat(thread: string): Promise<{ borrados: number }> {
  return apiFetch<{ borrados: number }>(
    `/api/admin/marketplace/chats/${encodeURIComponent(thread)}/borrar-borrador`,
    { method: 'POST' },
  );
}
