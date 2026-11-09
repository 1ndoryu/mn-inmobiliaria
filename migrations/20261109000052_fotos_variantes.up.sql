/* [30AA-1] Variantes IA de las fotos (v2). Tabla aparte, no una columna
 * `version` en `fotos`: así ninguna consulta existente (web pública, sync
 * del escritorio, tira del modal, marketplace) cambia de resultado ni hay
 * que regenerar la caché `.sqlx`.
 *
 * `orden` es el de la foto original a la que sustituye: la variante v2 es la
 * MISMA foto contada de otra manera, no una foto nueva del inmueble. */
CREATE TABLE IF NOT EXISTS fotos_variantes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
    orden INTEGER NOT NULL,
    storage_key TEXT NOT NULL,
    version SMALLINT NOT NULL DEFAULT 2 CHECK (version >= 2),
    modelo TEXT NOT NULL DEFAULT '',
    prompt_usado TEXT NOT NULL DEFAULT '',
    creada_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (inmueble_id, orden, version)
);

CREATE INDEX IF NOT EXISTS idx_fotos_variantes_inmueble
    ON fotos_variantes (inmueble_id, version, orden);
