/* Vuelta atrás de la v0: no hay filas que borrar (la migración no siembra
 * nada), solo se cierran otra vez los rangos. Si hubiera v0 creadas, el
 * ALTER falla a propósito antes que borrar textos de la usuaria. */

ALTER TABLE inmueble_facebook
    DROP CONSTRAINT IF EXISTS inmueble_facebook_version_check;
ALTER TABLE inmueble_facebook
    ADD CONSTRAINT inmueble_facebook_version_check CHECK (version >= 1);

ALTER TABLE facebook
    DROP CONSTRAINT IF EXISTS facebook_version_check;
ALTER TABLE facebook
    ADD CONSTRAINT facebook_version_check CHECK (version IS NULL OR version >= 1);

ALTER TABLE inmueble_versiones
    DROP CONSTRAINT IF EXISTS inmueble_versiones_version_check;
ALTER TABLE inmueble_versiones
    ADD CONSTRAINT inmueble_versiones_version_check CHECK (version >= 2);
