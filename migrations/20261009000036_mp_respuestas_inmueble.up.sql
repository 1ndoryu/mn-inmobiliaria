/* [09AA-30 F2] Caché compartida por inmueble. Clave: ficha (`catalog_hash`),
 * precio citado (`precio_hash`) y mensajes del cliente normalizados
 * (`mensaje_clave`). Solo entran hilos con ficha conocida y 1-2 mensajes del
 * cliente. `respuesta` lleva `{{nombre}}` en lugar del nombre del hilo y se
 * rellena al servir. La fila del hilo (`mp_respuestas_cache`) guarda la clave
 * en `mensaje_clave` para que una corrección o regeneración llegue aquí. */
CREATE TABLE mp_respuestas_inmueble (
    catalog_hash TEXT NOT NULL,
    precio_hash TEXT NOT NULL,
    mensaje_clave TEXT NOT NULL,
    respuesta TEXT NOT NULL,
    valida_hasta TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '90 days',
    usos INTEGER NOT NULL DEFAULT 0,
    corregida BOOLEAN NOT NULL DEFAULT FALSE,
    origen TEXT NULL,
    tokens_entrada BIGINT NULL,
    tokens_salida BIGINT NULL,
    ms_generacion BIGINT NULL,
    PRIMARY KEY (catalog_hash, precio_hash, mensaje_clave)
);

ALTER TABLE mp_respuestas_cache
    ADD COLUMN mensaje_clave TEXT NULL;
