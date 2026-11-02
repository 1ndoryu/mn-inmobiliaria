/* Marca de la primera pasada de sincronización de solicitudes [28AE-1].
 * Sin ella, si producción estaba vacía en la primera pasada, la siguiente
 * solicitud real volvía a contarse como primera carga y nacía vista (sin aviso).
 * Una sola fila: `id` fijo a TRUE. */

CREATE TABLE sync_solicitudes_estado (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    primera_pasada_en TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
