/* [30AA-13] Cada versión se publica en UNA cuenta fija: la v1 en el Facebook
 * de Maye y la v2 en el de Wan (decisión de la usuaria, 2026-11-16). Lo que
 * falta por publicar se mide contra esa cuenta y su versión, no contra
 * cualquier cuenta.
 *
 * `NULL` = cuenta todavía sin versión asignada (no participa en el export). */

/* El CHECK tiene que admitir NULL: se añade a una columna vacía y solo
 * después se siembran las versiones. */
ALTER TABLE facebook
    ADD COLUMN IF NOT EXISTS version SMALLINT
        CHECK (version IS NULL OR version >= 1);

/* Una cuenta por versión: dos cuentas no pueden reclamar la v2. */
CREATE UNIQUE INDEX IF NOT EXISTS idx_facebook_version
    ON facebook (version)
    WHERE version IS NOT NULL;

/* Sembrar lo que ya existe sin inventar: la principal es la v1 y las demás
 * se reparten por antigüedad. En esta base queda Maye = 1 y Wan = 2. */
UPDATE facebook SET version = 1 WHERE principal AND version IS NULL;

WITH orden AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY creado_en, nombre) + 1 AS n
    FROM facebook
    WHERE version IS NULL
)
UPDATE facebook f SET version = orden.n
FROM orden
WHERE f.id = orden.id;
