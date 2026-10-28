/* Calendario de publicaciones en redes [28AA-1]. Cada publicación es un
 * inmueble en un día: sus piezas son las imágenes subidas a mano (un formato
 * por pieza) y el checklist marca en qué destinos ya se publicó. Saltar una
 * propuesta crea una fila en estado `saltada`, que no cuenta para la cola
 * (la cola solo mira `publicada_en`). */
CREATE TABLE publicaciones_redes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
    estado TEXT NOT NULL DEFAULT 'borrador'
        CHECK (estado IN ('borrador', 'lista', 'publicada', 'saltada')),
    descripcion TEXT NOT NULL DEFAULT '' CHECK (char_length(descripcion) <= 2200),
    tags TEXT[] NOT NULL DEFAULT '{}',
    prompt_usado TEXT NOT NULL DEFAULT '',
    modelo TEXT NOT NULL DEFAULT '',
    -- Destinos del checklist [D1]: IG post, fanpage, IG historia, FB perfil.
    ig_post BOOLEAN NOT NULL DEFAULT FALSE,
    fanpage BOOLEAN NOT NULL DEFAULT FALSE,
    ig_historia BOOLEAN NOT NULL DEFAULT FALSE,
    fb_perfil BOOLEAN NOT NULL DEFAULT FALSE,
    -- Día previsto en el calendario [D2]; `publicada_en` es el día real.
    programada_para DATE,
    publicada_en TIMESTAMPTZ,
    creada_en TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Cola: último `publicada_en` por inmueble, y calendario por día.
CREATE INDEX publicaciones_redes_inmueble_idx
    ON publicaciones_redes (inmueble_id, publicada_en DESC);
CREATE INDEX publicaciones_redes_programada_idx
    ON publicaciones_redes (programada_para);

CREATE TABLE publicacion_piezas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publicacion_id UUID NOT NULL REFERENCES publicaciones_redes(id) ON DELETE CASCADE,
    -- Formato detectado por proporción al subir [28AA-4].
    formato TEXT NOT NULL CHECK (formato IN ('1:1', '3:4', '4:5', '9:16')),
    storage_key TEXT NOT NULL,
    ancho INTEGER NOT NULL CHECK (ancho > 0),
    alto INTEGER NOT NULL CHECK (alto > 0),
    bytes INTEGER NOT NULL CHECK (bytes >= 0),
    creada_en TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX publicacion_piezas_publicacion_idx ON publicacion_piezas (publicacion_id);
