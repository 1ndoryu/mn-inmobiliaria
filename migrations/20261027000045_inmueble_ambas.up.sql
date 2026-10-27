/* Modo «ambas»: el inmueble se ofrece en venta y en alquiler a la vez.
 * `precio` sigue siendo el de venta (no cambia ningún dato existente) y
 * `precio_alquiler` es el canon mensual; solo se usa cuando operacion = 'ambas'.
 * Las solicitudes (captaciones) no cambian: siguen siendo venta o alquiler. */
ALTER TABLE inmuebles DROP CONSTRAINT inmuebles_operacion_check;
ALTER TABLE inmuebles ADD CONSTRAINT inmuebles_operacion_check
    CHECK (operacion IN ('venta', 'alquiler', 'ambas'));

ALTER TABLE inmuebles
    ADD COLUMN precio_alquiler DOUBLE PRECISION NOT NULL DEFAULT 0
        CHECK (precio_alquiler >= 0);
