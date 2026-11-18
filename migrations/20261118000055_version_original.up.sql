/* [30AA-18] La v0 es la versión de las fotos ORIGINALES (las subidas, sin
 * pasar por Gemini): sirve para publicar el inmueble con sus fotos tal cual,
 * con un texto propio distinto del de la v1. Decisión de la usuaria: «pon que
 * sea v0» y para todos los inmuebles (2026-11-18).
 *
 * Solo se relajan los CHECK: la v0 es texto (`inmueble_versiones`) y sus fotos
 * salen de `fotos.origen = 'original'`, no de `fotos_variantes` (que sigue
 * siendo v2 en adelante porque ahí siempre es una variante generada). */

ALTER TABLE inmueble_versiones
    DROP CONSTRAINT IF EXISTS inmueble_versiones_version_check;
ALTER TABLE inmueble_versiones
    ADD CONSTRAINT inmueble_versiones_version_check CHECK (version >= 0);

/* `facebook.version` y `inmueble_facebook.version` admiten 0: si algún día
 * hay una cuenta para las originales, el modelo no la bloquea. */
ALTER TABLE facebook
    DROP CONSTRAINT IF EXISTS facebook_version_check;
ALTER TABLE facebook
    ADD CONSTRAINT facebook_version_check CHECK (version IS NULL OR version >= 0);

ALTER TABLE inmueble_facebook
    DROP CONSTRAINT IF EXISTS inmueble_facebook_version_check;
ALTER TABLE inmueble_facebook
    ADD CONSTRAINT inmueble_facebook_version_check CHECK (version >= 0);
