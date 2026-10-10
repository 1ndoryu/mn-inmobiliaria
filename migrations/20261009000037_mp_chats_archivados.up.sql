/* [09AA-30] Chats archivados desde el menú de tres puntos del panel. Solo
 * ocultan el hilo de la lista (`resumen_chats`); su caché y sus correcciones
 * siguen intactas. */
CREATE TABLE mp_chats_archivados (
    thread_id TEXT PRIMARY KEY,
    archivado_en TIMESTAMPTZ NOT NULL DEFAULT now()
);
