/* [09AA-7] `responder` idempotente: el cliente manda una clave por envío y un
 * reintento con la misma clave no duplica el mensaje ni el envío a WhatsApp.
 * Solo el staff la usa hoy; las filas de IA/cliente quedan a NULL (sin
 * restricción), así que el índice parcial no afecta al resto del hilo. */
ALTER TABLE agent_messages
    ADD COLUMN idempotency_key TEXT NULL
    CONSTRAINT chk_agent_messages_idem_len
        CHECK (char_length(idempotency_key) BETWEEN 1 AND 64);

CREATE UNIQUE INDEX uq_agent_messages_idem
    ON agent_messages (session_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
