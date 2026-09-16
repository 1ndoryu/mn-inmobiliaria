-- Reversión: restaura el vocabulario anterior ('piso' y 'otro').
UPDATE inmuebles SET tipo = 'piso' WHERE tipo = 'apartamento';
ALTER TABLE inmuebles DROP CONSTRAINT inmuebles_tipo_check;
ALTER TABLE inmuebles ALTER COLUMN tipo SET DEFAULT 'otro';
ALTER TABLE inmuebles
    ADD CHECK (tipo IN ('piso', 'casa', 'local', 'terreno', 'townhouse', 'otro'));
