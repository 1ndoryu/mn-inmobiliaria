/* Revierte «ambas»: los inmuebles ambas vuelven a venta (su precio de venta
 * ya está en `precio`); el canon guardado se pierde. */
UPDATE inmuebles SET operacion = 'venta' WHERE operacion = 'ambas';

ALTER TABLE inmuebles DROP COLUMN IF EXISTS precio_alquiler;
ALTER TABLE inmuebles DROP CONSTRAINT inmuebles_operacion_check;
ALTER TABLE inmuebles ADD CONSTRAINT inmuebles_operacion_check
    CHECK (operacion IN ('venta', 'alquiler'));
