/* [09AA-30 F4] Fecha de creación de cada fila de caché: base de
 * `ultimo_borrador_en` en el panel. Las filas anteriores quedan a NULL (fecha
 * desconocida); solo las nuevas reciben `now()` por defecto. No es la hora del
 * último mensaje del cliente: esa no llega al backend. */
ALTER TABLE mp_respuestas_cache
    ADD COLUMN creada_en TIMESTAMPTZ NULL;
ALTER TABLE mp_respuestas_cache
    ALTER COLUMN creada_en SET DEFAULT now();
