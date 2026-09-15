ALTER TABLE inmuebles
    ADD COLUMN copy_corta TEXT,
    ADD COLUMN copy_larga TEXT,
    ADD COLUMN copy_modelo TEXT,
    ADD COLUMN copy_actualizada_en TIMESTAMPTZ;
