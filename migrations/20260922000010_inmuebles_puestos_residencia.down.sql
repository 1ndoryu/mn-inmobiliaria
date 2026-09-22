ALTER TABLE inmuebles
    DROP CONSTRAINT IF EXISTS chk_inmuebles_puestos_no_negativo,
    DROP COLUMN IF EXISTS residencia,
    DROP COLUMN IF EXISTS puestos;
