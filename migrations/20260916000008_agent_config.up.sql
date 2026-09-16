-- 169A-4 (F7): config del chat con IA. DDL identico a
-- glory-agent/migrations/0002_config.sql. Claves del nucleo:
-- prompt_extra, ai_enabled_global. Claves del producto: contacto_telefono,
-- whatsapp_admin, tools_deshabilitadas (csv).

CREATE TABLE IF NOT EXISTS agent_config (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL DEFAULT '',
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
