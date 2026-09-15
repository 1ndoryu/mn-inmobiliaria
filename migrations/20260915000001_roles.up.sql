-- 159A-1: roles de usuario (owner crea admins, register queda como bootstrap)
ALTER TABLE users
    ADD COLUMN role TEXT NOT NULL DEFAULT 'admin'
    CHECK (role IN ('owner', 'admin'));

-- El primer usuario existente pasa a ser owner
UPDATE users
SET role = 'owner'
WHERE id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1);
