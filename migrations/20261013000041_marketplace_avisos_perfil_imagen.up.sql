/* [13AA-4] Foto del aviso (URL del CDN de Facebook): el modal «Vincular con
 * Facebook» la muestra para reconocer el aviso sin abrir Facebook. La lab la
 * refresca en cada lectura diaria; las URLs de fbcdn caducan. */
ALTER TABLE marketplace_avisos_perfil ADD COLUMN imagen_url TEXT NULL;
