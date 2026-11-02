/* Revierte el respaldo de descripciones originales [28AE-2]. OJO: borra los
 * textos originales guardados; hacer copia antes si ya hay datos. */
DROP INDEX IF EXISTS idx_descripciones_originales_inmueble;
DROP TABLE IF EXISTS descripciones_originales;
