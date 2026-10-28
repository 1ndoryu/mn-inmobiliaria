/* Revierte el calendario de redes: borra publicaciones y piezas. Es la única
 * migración de esta serie que destruye datos; solo se ejecuta con copia. */
DROP TABLE IF EXISTS publicacion_piezas;
DROP TABLE IF EXISTS publicaciones_redes;
