-- Vocabulario de tipos: 'piso' -> 'apartamento', se retira 'otro'.
-- Sin filas afectadas hoy (solo casa/local/townhouse en datos), los UPDATE
-- quedan como red de seguridad para cualquier borrador legacy.
UPDATE inmuebles SET tipo = 'apartamento' WHERE tipo = 'piso';
UPDATE inmuebles SET tipo = 'apartamento' WHERE tipo = 'otro';
ALTER TABLE inmuebles DROP CONSTRAINT inmuebles_tipo_check;
ALTER TABLE inmuebles ALTER COLUMN tipo SET DEFAULT 'apartamento';
ALTER TABLE inmuebles
    ADD CHECK (tipo IN ('apartamento', 'casa', 'local', 'terreno', 'townhouse'));
