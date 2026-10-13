/* [13AA-4] Avisos del perfil de Facebook Marketplace, leídos por la lab una vez
 * al día. Cada lectura reemplaza el estado: los avisos que ya no aparecen
 * (vendidos o borrados) quedan `presente = FALSE` y la fila no se borra. */
CREATE TABLE marketplace_avisos_perfil (
    aviso_id TEXT PRIMARY KEY,
    titulo   TEXT NOT NULL,
    precio   TEXT NULL,
    url      TEXT NOT NULL,
    visto_en TIMESTAMPTZ NOT NULL DEFAULT now(),
    presente BOOLEAN NOT NULL DEFAULT TRUE
);
