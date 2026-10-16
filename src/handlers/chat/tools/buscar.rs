/* [16AA-9 F3] `buscar_inmuebles` y sus tarjetas (una propiedad por
 * mensaje, con dedup entre turnos). Salio de `chat/tools.rs`; ninguna
 * funcion cambio de cuerpo, solo de sitio. */

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{OPERACIONES, TIPOS};
use crate::repositories::chat::tools::{tarjetas_inmuebles, FiltrosTarjetas, Tarjeta};
use crate::repositories::ClienteRepository;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, Encolado,
};
use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;

use super::espejo::{cuerpos_ai_recientes, espejar_en_hilo};

pub(super) async fn buscar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
    hub: Option<&ChatHub>,
) -> Result<Value, AgentError> {
    let texto = args
        .get("texto")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let tipo = args
        .get("tipo")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(t) = tipo {
        if !TIPOS.contains(&t) {
            return Ok(json!({"error": "tipo invalido"}));
        }
    }
    let operacion = args
        .get("operacion")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(o) = operacion {
        if !OPERACIONES.contains(&o) {
            return Ok(json!({"error": "operacion invalida"}));
        }
    }
    let precio_max = args
        .get("precio_max")
        .and_then(Value::as_f64)
        .filter(|p| *p > 0.0);
    /* [Fase3-H3] Habitaciones exactas en BD (antes el modelo filtraba a ojo
     * y colaba 3 hab cuando pedían 2). Solo enteros positivos; NULL/0 = sin
     * filtro. Ojo: filas con `habitaciones` NULL quedan fuera si se filtra. */
    let habitaciones = args
        .get("habitaciones")
        .and_then(Value::as_i64)
        .filter(|h| *h > 0);
    /* [Fase3-H4] Zona aparte de `texto`: el ILIKE sobre `ubicacion` no cubre
     * zonas que no aparecen literales ("norte" vs "Guayana Country Club").
     * Filtra lo que sí coincide y el prompt ordena reconocer el vacío. */
    let zona = args
        .get("zona")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let limite = args
        .get("limite")
        .and_then(Value::as_i64)
        .unwrap_or(5)
        .clamp(1, 10);
    /* [229A-1] Struct en vez de tupla de 9 (mismo patrón que `Ficha`):
     * legible y evita el lint de tipos complejos.
     * [Fase3-v2] `texto`/`zona` pasan por `sencilla()` (migración
     * `20260929000018`: minúsculas sin tildes): el visitante escribe
     * "Caroni" y la BD guarda "Caroní" (el ILIKE directo daba 0 filas y
     * la IA negaba oferta existente). */
    let filas: Vec<Tarjeta> = tarjetas_inmuebles(
        pool,
        FiltrosTarjetas {
            texto,
            tipo,
            operacion,
            precio_max,
            habitaciones,
            zona,
        },
        limite,
    )
    .await
    .map_err(|e| AgentError::Db(e.to_string()))?;
    let total = filas.len();
    /* [E-fluido F2] Tarjetas 1-propiedad-por-mensaje: el modelo tiende a soltar
     * la lista entera en un bloque (molesto de leer en WhatsApp), asi que el
     * backend encola hasta `MAX_TARJETAS` con formato fijo y devuelve
     * `tarjetas_enviadas` para que NO las repita: solo intro + cierre. Sin
     * telefono o sin canal WhatsApp no hay a donde enviarlas: `0` y el modelo
     * lista como antes (widget web).
     * Ojo: `filas` se consume abajo para `items`; las tarjetas van primero. */
    let (enviadas, repetidas) = encolar_tarjetas(pool, session_id, &filas, hub).await;
    let items: Vec<Value> = filas
        .into_iter()
        .map(|t| {
            json!({"id": t.id, "titulo": t.titulo, "tipo": t.tipo, "operacion": t.operacion,
                   "precio": t.precio, "ubicacion": t.ubicacion, "slug": t.slug,
                   "puestos": t.puestos, "residencia": t.residencia, "habitaciones": t.habitaciones})
        })
        .collect();
    Ok(
        json!({"inmuebles": items, "total": total, "tarjetas_enviadas": enviadas, "repetidas": repetidas}),
    )
}

/// Tope de tarjetas por turno (regla usuaria: con mas de 5 resultados se
/// muestran 5 + cierre "tengo N mas").
const MAX_TARJETAS: usize = 5;

/// Tarjeta breve de texto (una propiedad por mensaje, menos de 300 chars).
fn tarjeta_texto(t: &Tarjeta) -> String {
    let titulo: String = t.titulo.trim().chars().take(120).collect();
    let ubicacion: String = t.ubicacion.trim().chars().take(80).collect();
    format!(
        "🏠 {titulo}\n{} en {} · {}\n📍 {ubicacion}",
        t.tipo,
        t.operacion,
        formato_precio(t.precio)
    )
}

/// Precio en dolares con miles (`$150.000`): legible en una tarjeta breve.
/// Sin casts (el redondeo va por formato): `abs` + `{:.0}` + agrupar.
fn formato_precio(precio: f64) -> String {
    let entero: String = format!("{:.0}", precio.abs())
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let entero = if entero.is_empty() {
        "0".to_string()
    } else {
        entero
    };
    let digitos: Vec<char> = entero.chars().collect();
    let mut grupos: Vec<String> = Vec::new();
    let mut resto = digitos.as_slice();
    while resto.len() > 3 {
        let (cabeza, cola) = resto.split_at(resto.len() - 3);
        grupos.push(cola.iter().collect());
        resto = cabeza;
    }
    grupos.push(resto.iter().collect());
    grupos.reverse();
    format!("${}", grupos.join("."))
}

/// Encola una tarjeta por propiedad (hasta `MAX_TARJETAS`) en outbox
/// `whatsapp`. Devuelve `(nuevas, repetidas)`: `0` nuevas = el modelo lista
/// a mano o ya estaban encoladas.
/// [309A-1] Dedup entre turnos: el modelo re-llama `buscar` en el turno
/// siguiente ("mándame las fotos") y antes re-encolaba la misma tarjeta
/// (el visitante la recibía 2 veces). Se compara el texto exacto contra los
/// últimos 30 mensajes `ai` del hilo (incluye los espejos 299A-4): lo ya
/// enviado se salta y se cuenta en `repetidas`. Best-effort: si la lectura
/// falla se envía como antes (fail-open, nunca se bloquea un envío por un
/// fallo de lectura) y se avisa con WARN.
async fn encolar_tarjetas(
    pool: &PgPool,
    session_id: Uuid,
    filas: &[Tarjeta],
    hub: Option<&ChatHub>,
) -> (usize, usize) {
    if filas.is_empty() {
        return (0, 0);
    }
    let telefono = match ClienteRepository::ficha_para_aviso(pool, session_id).await {
        Ok(f) => f
            .telefono
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty()),
        Err(e) => {
            tracing::warn!("tarjetas sesion={session_id}: sin ficha ({e}), no se encolan");
            return (0, 0);
        }
    };
    let Some(destino) = telefono else {
        return (0, 0);
    };
    let via = match ClienteRepository::canal_de(pool, session_id).await {
        Ok(Some(v)) if v == "wa_a" || v == "wa_b" => v,
        _ => return (0, 0),
    };
    let mut ya_enviadas = cuerpos_ai_recientes(pool, session_id).await;
    let mut enviadas = 0;
    let mut repetidas = 0;
    for t in filas.iter().take(MAX_TARJETAS) {
        let texto = tarjeta_texto(t);
        if ya_enviadas.contains(&texto) {
            repetidas += 1;
            continue;
        }
        /* [011A-5 Fase1] Clave idempotente bajo corte: un reintento con el
         * mismo texto no duplica (el gemelo en vuelo ya espejó en hilo).
         * Se calcula antes del `json!` porque este mueve `via`. */
        let clave_tarjeta;
        let clave_tarjeta_ref = if debe_usar_clave("tarjeta") && corte_cubre(pool, &via).await {
            clave_tarjeta = clave_idempotencia(&session_id.to_string(), "tarjeta", &texto);
            Some(clave_tarjeta.as_str())
        } else {
            None
        };
        let tarjeta = json!({
            "session_id": session_id.to_string(),
            "destino": destino,
            "texto": texto.clone(),
            "via": via,
            "motivo": "tarjeta",
        });
        match encolar_outbox_idem(pool, "whatsapp", tarjeta, clave_tarjeta_ref).await {
            Ok(Encolado::Nuevo(_)) => {
                enviadas += 1;
                /* Se registra en el set para que dos filas con el mismo
                 * texto en el mismo lote tampoco se dupliquen. */
                ya_enviadas.insert(texto.clone());
                espejar_en_hilo(pool, hub, session_id, &texto).await;
            }
            /* [011A-5 Fase3] Revivido = el gemelo estaba `failed` y vuelve
             * a `pending` con este payload: cuenta como enviada y entra al
             * set del lote, pero NO se espeja (el primer encolado ya dejó
             * el texto en el hilo). */
            Ok(Encolado::Revivido(_)) => {
                enviadas += 1;
                ya_enviadas.insert(texto.clone());
            }
            Ok(Encolado::Duplicado) => {
                repetidas += 1;
            }
            Err(e) => {
                tracing::warn!("tarjetas sesion={session_id}: no se pudo encolar ({e})");
                break;
            }
        }
    }
    (enviadas, repetidas)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use super::super::pruebas::{limpiar_sesion, pool_si_hay};
    use super::super::Herramientas;
    use glory_agent::tools::{ToolCtx, ToolExecutor};

    /* [E-fluido F2] Tarjeta breve: una propiedad por mensaje, menos de 300
     * chars, precio con miles. */
    #[test]
    fn tarjeta_breve_con_precio_legible() {
        assert_eq!(formato_precio(150_000.0), "$150.000");
        assert_eq!(formato_precio(2_500.5), "$2.500");
        assert_eq!(formato_precio(900.0), "$900");
        let t = Tarjeta {
            id: Uuid::new_v4(),
            titulo: "Apartamento Residencias Caroní Plaza".to_string(),
            tipo: "apartamento".to_string(),
            operacion: "venta".to_string(),
            precio: 85_000.0,
            ubicacion: "Puerto Ordaz".to_string(),
            slug: "x".to_string(),
            puestos: 3,
            residencia: "2".to_string(),
            habitaciones: 2,
        };
        let texto = tarjeta_texto(&t);
        assert!(texto.contains("Apartamento Residencias Caroní Plaza"));
        assert!(texto.contains("apartamento en venta"));
        assert!(texto.contains("$85.000"));
        assert!(texto.chars().count() < 300);
        let larga = Tarjeta {
            titulo: "x".repeat(500),
            ubicacion: "y".repeat(500),
            ..Tarjeta {
                id: Uuid::new_v4(),
                titulo: String::new(),
                tipo: "casa".to_string(),
                operacion: "alquiler".to_string(),
                precio: 1_200_000.0,
                ubicacion: String::new(),
                slug: String::new(),
                puestos: 1,
                residencia: String::new(),
                habitaciones: 1,
            }
        };
        assert!(tarjeta_texto(&larga).chars().count() < 300);
    }

    /* [E-fluido F2] Sin teléfono del visitante no hay a donde enviar tarjetas:
     * `tarjetas_enviadas` 0, nada en outbox y el modelo lista a mano. */
    #[tokio::test]
    async fn buscar_sin_telefono_no_encola_tarjetas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(
            salida.get("tarjetas_enviadas").and_then(Value::as_u64),
            Some(0)
        );
        let encoladas: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(encoladas, 0);
        limpiar_sesion(&pool, sesion).await;
    }

    /* [Fase3-H3] El filtro de habitaciones es exacto en BD: pedir 2 nunca
     * trae 3 (antes el modelo filtraba a ojo y colaba de más). */
    #[tokio::test]
    async fn buscar_filtra_habitaciones_exactas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute("buscar_inmuebles", &json!({"habitaciones": 2}), &ctx)
            .await
            .unwrap();
        let items = salida
            .get("inmuebles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());
        assert!(items
            .iter()
            .all(|i| i.get("habitaciones").and_then(Value::as_i64) == Some(2)));
        limpiar_sesion(&pool, sesion).await;
    }

    /* [Fase3-v2] Sin tildes también encuentra: "caroni" localiza el local
     * de "Riberas del Caroní" (antes 0 filas y la IA negaba la oferta). */
    #[tokio::test]
    async fn buscar_sin_tilde_encuentra_igual() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute("buscar_inmuebles", &json!({"zona": "caroni"}), &ctx)
            .await
            .unwrap();
        let items = salida
            .get("inmuebles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i
            .get("ubicacion")
            .and_then(Value::as_str)
            .is_some_and(|u| u.contains("Caron"))));
        limpiar_sesion(&pool, sesion).await;
    }

    /* [309A-1] Segunda vuelta de `buscar` con el mismo filtro no re-encola
     * tarjetas ya enviadas (antes el visitante recibía la tarjeta 2 veces:
     * turno 1 `buscar` + turno 2 `buscar` de nuevo al pedir fotos). */
    #[tokio::test]
    async fn buscar_no_repite_tarjetas_ya_enviadas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string())
            .with_hub(ChatHub::default());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Dedup Test", "telefono": "+34644444444"}),
            &ctx,
        )
        .await
        .unwrap();
        /* `registrar` no vincula canal (Fase3-H1): se fija `wa_b` directo
         * para que `encolar_tarjetas` tenga vía de salida. */
        sqlx::query(
            "INSERT INTO canal_sesiones (session_id, cliente_id, canal, telefono, modo) \
             VALUES ($1, NULL, 'wa_b', '34644444444', 'completo')",
        )
        .bind(sesion)
        .execute(&pool)
        .await
        .unwrap();

        let primera = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert!(
            primera
                .get("tarjetas_enviadas")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        );
        let outbox_1: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        let segunda = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(
            segunda.get("tarjetas_enviadas").and_then(Value::as_u64),
            Some(0)
        );
        assert!(
            segunda
                .get("repetidas")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        );
        let outbox_2: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(outbox_1, outbox_2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34644444444'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }
}
