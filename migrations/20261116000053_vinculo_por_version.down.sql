/* Vuelta atrás: se conservan solo los vínculos de la v1 (los de versiones
 * posteriores no caben en la clave vieja y perderían su aviso de todos modos). */
DELETE FROM inmueble_facebook WHERE version > 1;

ALTER TABLE inmueble_facebook DROP CONSTRAINT IF EXISTS inmueble_facebook_pkey;
ALTER TABLE inmueble_facebook
    ADD PRIMARY KEY (inmueble_id, facebook_id);

ALTER TABLE inmueble_facebook DROP COLUMN IF EXISTS version;
