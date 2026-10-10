/* [09AA-30] Origen y coste por fila de caché. `origen`: `ia` = texto
 * generado por la IA, `releer` = fila solo-foto sin borrador; NULL = filas
 * anteriores a esta migración (desconocido). Tokens y tiempo son de la
 * generación original: un hit no gasta IA, así que no los repite. */
ALTER TABLE mp_respuestas_cache
    ADD COLUMN origen TEXT NULL,
    ADD COLUMN tokens_entrada BIGINT NULL,
    ADD COLUMN tokens_salida BIGINT NULL,
    ADD COLUMN ms_generacion BIGINT NULL;
