/* [169A-2+4+7] Reversion de la trazabilidad: se normalizan los NULL
 * a 0 antes de devolver el NOT NULL + DEFAULT 0 originales. */

UPDATE solicitudes SET precio_estimado = 0 WHERE precio_estimado IS NULL;
ALTER TABLE solicitudes ALTER COLUMN precio_estimado SET DEFAULT 0;
ALTER TABLE solicitudes ALTER COLUMN precio_estimado SET NOT NULL;
ALTER TABLE solicitudes DROP COLUMN IF EXISTS origen_contacto;
ALTER TABLE solicitudes DROP COLUMN IF EXISTS user_agent;
ALTER TABLE solicitudes DROP COLUMN IF EXISTS ip_origen;
