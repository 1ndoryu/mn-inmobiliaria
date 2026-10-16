-- [16AA-9 F5] Rollback: solo la vista. Las dos tablas y sus escrituras nunca
-- se tocaron, así que no hay nada que deshacer en ellas.

DROP VIEW IF EXISTS ia_uso;
