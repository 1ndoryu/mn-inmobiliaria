DROP INDEX IF EXISTS uq_agent_messages_idem;

ALTER TABLE agent_messages
    DROP CONSTRAINT IF EXISTS chk_agent_messages_idem_len;

ALTER TABLE agent_messages
    DROP COLUMN IF EXISTS idempotency_key;
