/* [169A-2+4+7] Trazabilidad anti-spam y precio opcional.
 * `ip_origen`/`user_agent`: cabeceras reales o anonimizadas tras el proxy,
 * para auditar rafagas y origen (no se exponen en la API publica).
 * `origen_contacto`: canal declarado (allowlist en el modelo); `web` por defecto.
 * `precio_estimado` pasa a nullable (NULL = sin estimar); filas existentes
 * conservan su valor, sin mutar datos. */

ALTER TABLE solicitudes ADD COLUMN IF NOT EXISTS ip_origen TEXT;
ALTER TABLE solicitudes ADD COLUMN IF NOT EXISTS user_agent TEXT;
ALTER TABLE solicitudes
    ADD COLUMN IF NOT EXISTS origen_contacto TEXT NOT NULL DEFAULT 'web';
ALTER TABLE solicitudes ALTER COLUMN precio_estimado DROP NOT NULL;
ALTER TABLE solicitudes ALTER COLUMN precio_estimado DROP DEFAULT;
