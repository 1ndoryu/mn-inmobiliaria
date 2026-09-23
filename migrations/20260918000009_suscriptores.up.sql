/* [189A-1] Suscriptores del pie público: solo el correo (único; el
 * servicio lo normaliza a minúsculas) + marcas temporales.
 * Sin estado ni gestión: el panel admin queda pendiente. */

CREATE TABLE IF NOT EXISTS suscriptores (
    id UUID PRIMARY KEY,
    email VARCHAR(254) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
