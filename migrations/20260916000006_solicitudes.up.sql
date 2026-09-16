-- 169A-2: solicitudes "Publicar mi inmueble" (pendientes de revisión)
-- La foto vive en disco (`UPLOAD_DIR/solicitudes/<uuid>/<uuid>.<ext>`);
-- aquí solo se guardan las claves. Sin FK: la solicitud puede descartarse
-- sin arrastrar nada; las huérfanas se limpian en un LOTE posterior.

CREATE TABLE solicitudes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    nombre TEXT NOT NULL,
    telefono TEXT NOT NULL,
    email TEXT NOT NULL,
    descripcion TEXT NOT NULL DEFAULT '',
    ubicacion TEXT NOT NULL DEFAULT '',
    precio_estimado DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (precio_estimado >= 0),
    operacion TEXT NOT NULL DEFAULT 'venta'
        CHECK (operacion IN ('venta', 'alquiler')),
    estado TEXT NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'revisada', 'aceptada', 'descartada')),
    fotos TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_solicitudes_estado ON solicitudes(estado, created_at DESC);
