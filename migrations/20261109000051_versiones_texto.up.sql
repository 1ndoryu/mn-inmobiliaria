/* [30AA-1] Versionado de textos para publicar el mismo inmueble en otro
 * Facebook sin repetir título ni descripción. La v1 NO se duplica: sigue
 * viviendo en `inmuebles.titulo`/`descripcion` (así la web pública no cambia
 * ni una consulta). Aquí solo viven las variantes (v2 en adelante).
 *
 * `estado` es el visto bueno manual: la variante nace en `borrador` y solo
 * pasa a `lista` cuando la dueña revisa que no inventó datos. */

CREATE TABLE IF NOT EXISTS inmueble_versiones (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
    version SMALLINT NOT NULL CHECK (version >= 2),
    titulo TEXT NOT NULL DEFAULT '',
    descripcion TEXT NOT NULL DEFAULT '',
    estado TEXT NOT NULL DEFAULT 'borrador'
        CHECK (estado IN ('borrador', 'lista')),
    modelo TEXT NOT NULL DEFAULT '',
    prompt_usado TEXT NOT NULL DEFAULT '',
    creada_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizada_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (inmueble_id, version)
);

/* Rastro de cada cambio de texto, sea generado o editado a mano. No se borra
 * al borrar una versión: es la evidencia de qué se dijo antes. */
CREATE TABLE IF NOT EXISTS inmueble_texto_historial (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
    version SMALLINT NOT NULL,
    campo TEXT NOT NULL CHECK (campo IN ('titulo', 'descripcion')),
    valor_anterior TEXT NOT NULL DEFAULT '',
    valor_nuevo TEXT NOT NULL DEFAULT '',
    origen TEXT NOT NULL DEFAULT 'ia',
    modelo TEXT NOT NULL DEFAULT '',
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_inmueble_texto_historial_inmueble
    ON inmueble_texto_historial (inmueble_id, creado_en DESC);

/* Ajustes de producto que no son del chat. Se reutiliza `agent_config` para no
 * crear una tabla de clave/valor paralela: misma forma (TEXT), mismas
 * funciones de lectura. La clave es `inmueble_versiones`. */
INSERT INTO agent_config (key, value)
VALUES ('inmueble_versiones', '2')
ON CONFLICT (key) DO NOTHING;
