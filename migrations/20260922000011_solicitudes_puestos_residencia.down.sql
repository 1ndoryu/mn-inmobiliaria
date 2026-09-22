ALTER TABLE solicitudes
    DROP CONSTRAINT IF EXISTS chk_solicitudes_puestos_no_negativo,
    DROP COLUMN IF EXISTS residencia,
    DROP COLUMN IF EXISTS puestos;
