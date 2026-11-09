/* [30AA-1] Facebook como entidad: el mismo inmueble se publica en varias
 * cuentas/páginas, cada una con su propio aviso de Marketplace. Antes solo
 * había `inmuebles.marketplace_id` (UNIQUE), que impedía vincular el mismo
 * inmueble a dos cuentas a la vez.
 *
 * `principal` marca la cuenta por defecto (Maye hoy): es la que se refleja en
 * `inmuebles.marketplace_id` para no romper el vínculo que ya lee el
 * laboratorio de Marketplace. */

CREATE TABLE IF NOT EXISTS facebook (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    nombre TEXT NOT NULL,
    url TEXT NOT NULL DEFAULT '',
    notas TEXT NOT NULL DEFAULT '',
    principal BOOLEAN NOT NULL DEFAULT FALSE,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Un solo principal a la vez; parcial para no bloquear filas sin principal.
CREATE UNIQUE INDEX IF NOT EXISTS idx_facebook_principal
    ON facebook (principal) WHERE principal;

CREATE TABLE IF NOT EXISTS inmueble_facebook (
    inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
    facebook_id UUID NOT NULL REFERENCES facebook(id) ON DELETE CASCADE,
    /* Dígitos del aviso en ESA cuenta (`NULL` = cuenta vinculada sin aviso). */
    marketplace_id TEXT,
    publicado_en TIMESTAMPTZ,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (inmueble_id, facebook_id)
);

-- Un aviso pertenece a una sola ficha dentro de la misma cuenta.
CREATE UNIQUE INDEX IF NOT EXISTS idx_inmueble_facebook_aviso
    ON inmueble_facebook (facebook_id, marketplace_id)
    WHERE marketplace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_inmueble_facebook_facebook
    ON inmueble_facebook (facebook_id);

/* Semilla: la cuenta de Maye, que es la que hoy tiene todos los avisos. */
INSERT INTO facebook (nombre, url, notas, principal)
SELECT 'Maye', '', 'Cuenta original con todos los avisos históricos.', TRUE
WHERE NOT EXISTS (SELECT 1 FROM facebook);

/* Traslado de los vínculos existentes: cada `inmuebles.marketplace_id` pasa a
 * ser el aviso de esa ficha en la cuenta principal. `inmuebles.marketplace_id`
 * se conserva como espejo de esta fila (lo leen el laboratorio y el chat). */
INSERT INTO inmueble_facebook (inmueble_id, facebook_id, marketplace_id)
SELECT i.id, f.id, i.marketplace_id
FROM inmuebles i
CROSS JOIN LATERAL (SELECT id FROM facebook WHERE principal LIMIT 1) f
WHERE i.marketplace_id IS NOT NULL
ON CONFLICT DO NOTHING;
