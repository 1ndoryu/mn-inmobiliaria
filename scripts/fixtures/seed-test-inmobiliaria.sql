-- Datos sintéticos para la BD de prueba `glory_backend_inmobiliaria_test` (09AA-31).
-- Los tests de chat_tools (buscar_*, enviar_fotos_*) necesitan al menos un inmueble
-- publicado y disponible, con 2 habitaciones, ubicación con «Caroní» y 2 fotos.
-- Idempotente: IDs fijos y ON CONFLICT. Es un registro ficticio, no un inmueble real.
--
-- Aplicar SOLO a la BD de prueba (nunca a la de desarrollo ni a producción):
--   psql "<BASE>/glory_backend_inmobiliaria_test" -v ON_ERROR_STOP=1 -f scripts/fixtures/seed-test-inmobiliaria.sql

INSERT INTO inmuebles (
    id, titulo, descripcion, ubicacion, precio, tipo, operacion,
    habitaciones, banos, metros, metros_terreno, estado, publicado, slug
) VALUES (
    '00000000-0000-4000-8000-0000000000a1',
    'Apartamento de prueba (sintético)',
    'Registro ficticio para tests automáticos. No es un inmueble real.',
    'Puerto Ordaz, Riberas del Caroní',
    35000, 'apartamento', 'venta',
    2, 1, 80, 0, 'disponible', TRUE,
    'seed-test-apartamento-caroni'
)
ON CONFLICT (id) DO UPDATE SET
    titulo = EXCLUDED.titulo,
    descripcion = EXCLUDED.descripcion,
    ubicacion = EXCLUDED.ubicacion,
    precio = EXCLUDED.precio,
    tipo = EXCLUDED.tipo,
    operacion = EXCLUDED.operacion,
    habitaciones = EXCLUDED.habitaciones,
    banos = EXCLUDED.banos,
    metros = EXCLUDED.metros,
    metros_terreno = EXCLUDED.metros_terreno,
    estado = EXCLUDED.estado,
    publicado = EXCLUDED.publicado,
    updated_at = NOW();

INSERT INTO fotos (id, inmueble_id, storage_key, orden, origen) VALUES
    ('00000000-0000-4000-8000-0000000000f1', '00000000-0000-4000-8000-0000000000a1',
     'seed-test/apartamento-caroni-1.jpg', 0, 'original'),
    ('00000000-0000-4000-8000-0000000000f2', '00000000-0000-4000-8000-0000000000a1',
     'seed-test/apartamento-caroni-2.jpg', 1, 'original')
ON CONFLICT (id) DO UPDATE SET
    storage_key = EXCLUDED.storage_key,
    orden = EXCLUDED.orden,
    origen = EXCLUDED.origen;
