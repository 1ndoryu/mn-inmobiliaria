/* [18AA-6] Revierte `nombre_corto`. Las tres erratas de tecleo que corrige el
 * `up` no se deshacen: son arreglos de datos, no parte del esquema. */
ALTER TABLE inmuebles
  DROP COLUMN IF EXISTS nombre_corto;
