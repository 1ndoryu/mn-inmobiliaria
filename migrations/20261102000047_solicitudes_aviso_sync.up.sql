/* Aviso de solicitudes nuevas y sincronización desde producción [28AE-1].
 * `aviso_visto` = FALSE hace que la solicitud cuente en el aviso del admin.
 * Las que ya existen nacen vistas: migrar no debe avisar por lo antiguo.
 * `sincronizada_en` marca las que llegaron de producción (NULL = alta local). */

ALTER TABLE solicitudes ADD COLUMN aviso_visto BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE solicitudes SET aviso_visto = TRUE;
ALTER TABLE solicitudes ADD COLUMN sincronizada_en TIMESTAMPTZ NULL;
CREATE INDEX idx_solicitudes_no_vistas ON solicitudes(aviso_visto) WHERE aviso_visto = FALSE;
