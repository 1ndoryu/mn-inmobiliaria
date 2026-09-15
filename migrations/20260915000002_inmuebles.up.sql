-- 159A-1: catálogo de inmuebles + fotos
-- Enums como CHECK para no atar el template a tipos PG específicos.
-- Sin obligatorios: todo con DEFAULT, el admin admite borradores vacíos.

CREATE TABLE inmuebles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    titulo TEXT NOT NULL DEFAULT '',
    descripcion TEXT NOT NULL DEFAULT '',
    ubicacion TEXT NOT NULL DEFAULT '',
    precio DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (precio >= 0),
    tipo TEXT NOT NULL DEFAULT 'otro'
        CHECK (tipo IN ('piso', 'casa', 'local', 'terreno', 'townhouse', 'otro')),
    operacion TEXT NOT NULL DEFAULT 'venta'
        CHECK (operacion IN ('venta', 'alquiler')),
    habitaciones INTEGER NOT NULL DEFAULT 0 CHECK (habitaciones >= 0),
    banos INTEGER NOT NULL DEFAULT 0 CHECK (banos >= 0),
    metros DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (metros >= 0),
    metros_terreno DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (metros_terreno >= 0),
    estado TEXT NOT NULL DEFAULT 'disponible'
        CHECK (estado IN ('disponible', 'reservado', 'vendido', 'alquilado')),
    publicado BOOLEAN NOT NULL DEFAULT FALSE,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_inmuebles_slug ON inmuebles(slug);
CREATE INDEX idx_inmuebles_publicado ON inmuebles(publicado, created_at DESC)
    WHERE publicado;

CREATE TABLE fotos (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
    storage_key TEXT NOT NULL,
    orden INTEGER NOT NULL DEFAULT 0,
    origen TEXT NOT NULL DEFAULT 'original'
        CHECK (origen IN ('original', 'mejorada')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fotos_inmueble ON fotos(inmueble_id, orden);
