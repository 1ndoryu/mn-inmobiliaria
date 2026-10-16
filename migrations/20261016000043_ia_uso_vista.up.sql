-- [16AA-9 F5] Uso de IA unificado en una sola vista de lectura.
--
-- «Cuánto gasta la IA» tenía dos contabilidades que no se sumaban:
--   · `uso_mensajes`        — motor de chat: tokens por mensaje, del `usage`
--                             exacto de Responses (`tokens_in`/`tokens_out`);
--                             `tokens_est` (len/4) es la reserva cuando el
--                             proveedor no lo trajo.
--   · `mp_respuestas_cache` — marketplace: tokens de la generación del
--                             borrador, por respuesta cacheada (`origen='ia'`).
-- La vista las apila con columnas comunes. Ningún productor cambia: el motor
-- sigue escribiendo por el trigger `trg_uso_mensajes` y el marketplace por
-- `borrador.rs`.
--
-- `mp_uso_minuto` NO entra, aunque el plan lo nombrara: no son tokens, son
-- peticiones por minuto (`consumir_minuto`, el 429 de los endpoints mp).
-- Sumarlo al gasto mezclaría unidades. Se queda como está, de limitador.
--
-- `tokens_entrada`/`tokens_salida` en NULL = esa fila no trae `usage` exacto
-- (mensajes no-IA, filas viejas, o generación sin coste): no se inventa un 0.

CREATE OR REPLACE VIEW ia_uso AS
SELECT
    'motor'::TEXT AS origen,
    u.modelo      AS modelo,
    u.created_at  AS momento,
    u.tokens_in   AS tokens_entrada,
    u.tokens_out  AS tokens_salida
FROM uso_mensajes u
UNION ALL
SELECT
    'marketplace'::TEXT,
    NULL,
    c.creada_en,
    c.tokens_entrada,
    c.tokens_salida
FROM mp_respuestas_cache c
WHERE c.origen = 'ia';
