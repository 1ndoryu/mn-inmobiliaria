ALTER TABLE mp_respuestas_cache
    DROP COLUMN IF EXISTS ms_generacion,
    DROP COLUMN IF EXISTS tokens_salida,
    DROP COLUMN IF EXISTS tokens_entrada,
    DROP COLUMN IF EXISTS origen;
