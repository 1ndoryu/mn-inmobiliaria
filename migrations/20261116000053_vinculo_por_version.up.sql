/* [30AA-7] El aviso de Marketplace es POR VERSIÓN: la v1 y la v2 del mismo
 * inmueble se publican como anuncios distintos y cada una tiene su propio ID.
 * Hasta ahora la clave era (inmueble, cuenta), así que la segunda versión no
 * podía tener aviso propio en la misma cuenta.
 *
 * `version = 1` es la fila de siempre: los vínculos existentes siguen valiendo
 * y `inmuebles.marketplace_id` continúa espejando la v1 de la principal. */

ALTER TABLE inmueble_facebook
    ADD COLUMN IF NOT EXISTS version SMALLINT NOT NULL DEFAULT 1
        CHECK (version >= 1);

ALTER TABLE inmueble_facebook DROP CONSTRAINT IF EXISTS inmueble_facebook_pkey;
ALTER TABLE inmueble_facebook
    ADD PRIMARY KEY (inmueble_id, facebook_id, version);

/* Un aviso sigue siendo único dentro de su cuenta, sea de la versión que sea. */
DROP INDEX IF EXISTS idx_inmueble_facebook_aviso;
CREATE UNIQUE INDEX IF NOT EXISTS idx_inmueble_facebook_aviso
    ON inmueble_facebook (facebook_id, marketplace_id)
    WHERE marketplace_id IS NOT NULL;
