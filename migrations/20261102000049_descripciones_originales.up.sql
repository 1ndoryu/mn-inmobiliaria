/* Respaldo privado de la descripción original [28AE-2]. Se guarda ANTES de que
 * la IA la procese, sea cual sea la vía (panel, completar al publicar, CLI).
 * Tabla aparte, sin columnas en `inmuebles`: ninguna API pública ni DTO de
 * inmueble la toca. `inmueble_id` es opcional (la ficha nueva aún no existe al
 * llamar a la IA) y no se pierde el respaldo si se borra el inmueble. */

CREATE TABLE descripciones_originales (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    inmueble_id UUID NULL REFERENCES inmuebles(id) ON DELETE SET NULL,
    texto TEXT NOT NULL,
    origen TEXT NOT NULL DEFAULT 'panel',
    proveedor TEXT NULL,
    modelo TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_descripciones_originales_inmueble ON descripciones_originales(inmueble_id);
