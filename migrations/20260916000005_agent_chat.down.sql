-- 169A-1 (F5) rollback: retira las tablas del chat con IA.
DROP TABLE IF EXISTS agent_outbox;
DROP TABLE IF EXISTS agent_response_cycles;
DROP TABLE IF EXISTS agent_messages;
DROP TABLE IF EXISTS agent_sessions;
