/* [13AA-12] Estado de sincronización local ↔ servidor.
 * `fotos.sha256`/`bytes`: huella del archivo, fijada al subir y rellenada de
 * forma perezosa (acotada) para las filas antiguas; permite comparar fotos
 * entre entornos sin leer archivos en cada consulta. `storage_key` es único por
 * foto, así que la huella no caduca. `'sin-archivo'` marca la fila cuyo archivo
 * ya no existe en el volumen (para no reintentar eternamente).
 * `sync_base`: último contenido en que local y servidor coincidieron, por slug;
 * permite saber QUÉ lado cambió después (comparación de tres vías). Solo la
 * escribe la tarea de fondo del entorno local. */
ALTER TABLE fotos ADD COLUMN sha256 TEXT NULL;
ALTER TABLE fotos ADD COLUMN bytes INTEGER NULL;

CREATE TABLE sync_base (
    slug        TEXT PRIMARY KEY,
    campos      JSONB NOT NULL,
    fotos       JSONB NOT NULL,
    acordado_en TIMESTAMPTZ NOT NULL DEFAULT now()
);
