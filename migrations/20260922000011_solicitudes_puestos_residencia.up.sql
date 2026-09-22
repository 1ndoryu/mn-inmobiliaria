ALTER TABLE solicitudes
    ADD COLUMN IF NOT EXISTS puestos INTEGER NOT NULL DEFAULT 0
        CONSTRAINT chk_solicitudes_puestos_no_negativo CHECK (puestos >= 0),
    ADD COLUMN IF NOT EXISTS residencia TEXT NOT NULL DEFAULT '';
