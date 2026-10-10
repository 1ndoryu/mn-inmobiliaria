//! Regeneración explícita del borrador de un hilo (`regenerar`, `regenerar_uno`).

// Mismos imports que la cabecera del hub (marketplace.rs); el glob es intencional.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Regenerar explícito de la dueña: borra las filas no-corregidas del hilo +
/// bypass de lectura (nueva IA siempre) + reemplazo (pisa incluso
/// correcciones de la MISMA firma: lo pidió ella). Si la IA cae no se guarda
/// nada ([09AA-4]: antes se conservaba el viejo y el panel lo seguía
/// mostrando al abrir). Sin tope por minuto por decisión 2026-10-05 (freno
/// = ritmo humano); el resto del flujo (schema 422, no cachear fallback)
/// es idéntico al `borrador`. [08AA-14] Sin matriz negativa por decisión de
/// ella 2026-10-08: el texto de la IA pasa por `imponer_forma_borrador`
/// (09AA-2) y conserva la regla de no inventar contacto.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/regenerar",
    request_body = BorradorRequest,
    responses(
        (status = 200, description = "Borrador regenerado (ia o reserva)", body = BorradorResponse),
        (status = 422, description = "Schema inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn regenerar(
    State(state): State<AppState>,
    _auth: MpAuth,
    headers: HeaderMap,
    r: Result<Json<BorradorRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* [09AA-20] La llave se valida y se devuelve igual que en `borrador`
     * (aquí no hay vuelo: gesto explícito, la última que escribe gana).
     * [09AA-22] F3: además se verifica contra hint+firma dentro de
     * `regenerar_uno` (422 si es de otro hilo). */
    let clave_idem = clave_idempotencia(&headers)?;
    let resp = regenerar_uno(&state.pool, r.0, clave_idem.as_ref()).await?;
    let resp = (StatusCode::OK, Json(resp)).into_response();
    Ok(con_idempotencia(resp, clave_idem.as_ref()))
}

/// [09AA-3] Núcleo compartido de Regenerar (uno y en lote): normaliza el
/// excerpt, genera directo a la IA (bypass, gesto explícito) y reemplaza
/// la fila si es `ia`.
/// [09AA-4] Borra PRIMERO las filas no-corregidas del hilo y NO conserva
/// nada si la IA cae: 09AA-3 conservaba el viejo en `reserva` y el panel
/// seguía mostrando el texto viejo al abrir (reporte de ella 2026-10-09).
/// Las correcciones de la dueña (`corregida`) jamás se tocan.
/// Sin validar schema: las filas masivas ya se validaron al ingresar.
pub(super) async fn regenerar_uno(
    pool: &sqlx::PgPool,
    mut r: BorradorRequest,
    clave: Option<&String>,
) -> Result<BorradorResponse, AppError> {
    /* [09AA-20] Fuente compartida con `borrador`: estructurada (firma v2 +
     * excerpt renderizado) o texto plano con su limpieza de siempre.
     * [08AA-16/18] Con contexto del hilo y `clave_hilo()` (ver `fuente_v1`).
     * [08AA-21] Crudo capturado antes de limpiar (ver `fuente_v1`).
     * [09AA-22] F3: la llave se verifica contra hint+firma (la masiva pasa
     * `None`: sus filas son plano sin conversacion y no hay nada que atar). */
    let fuente = resolver_fuente(&mut r)?;
    verificar_idempotencia_conversacion(&r, &fuente, clave)?;
    let FuenteBorrador {
        firma_cache,
        firma_version,
        crudo,
        firma_legacy: _legacy,
        mensaje_clave,
    } = fuente;
    let titulo_fb = aviso_fb_de_thread(r.thread_id.trim());
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(pool, r.aviso_id.as_deref(), titulo_fb.as_deref()).await?;
    let nombre = nombre_de_thread(r.thread_id.trim());
    let clave_compartida = mensaje_clave
        .as_deref()
        .filter(|_| conocido && nombre.is_some())
        .map(|mensaje_clave| ClaveCompartida {
            catalog_hash: &catalog_hash,
            precio_hash: &precio_hash,
            mensaje_clave,
        });
    /* [09AA-4] Borrar primero, generar después: cada excerpt nuevo es una
     * firma nueva y las filas viejas del hilo viven 90 días; sin esto el
     * panel lista lo viejo junto a lo fresco y parece «cacheado».
     * `clave_hilo()` es idempotente (ver `guardar_cache`), así que vale
     * tanto el `thread_id` crudo del flotante como el ya guardado que
     * está en caché (p. ej. tras `releer`). Las correcciones de la dueña quedan. */
    let borradas = borrar_hilo_no_corregidas(pool, &clave_hilo(r.thread_id.trim())).await?;
    /* Bypass: directo a la IA, sin vuelo (Regenerar es gesto explícito; si
     * dos llegan juntas, la última que escribe gana por `reemplazar`). */
    let gen = generar_borrador(&r, seguro.as_ref(), pool).await;
    /* [09AA-5] Evento para la tab de Logs: si esto dice `ia` y el panel
     * sigue mostrando lo viejo, el problema está del otro lado (caché del
     * front o el puente local). */
    mp_log(
        LogNivel::Info,
        "regenerar",
        gen.fuente.as_str(),
        format!(
            "regeneración directa: {borradas} filas viejas borradas (aviso conocido: {conocido})"
        ),
        &[
            ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
            ("borradas", serde_json::json!(borradas)),
            ("aviso_conocido", serde_json::json!(conocido)),
        ],
    );
    if gen.fuente == "ia" {
        let foto = FotoHilo {
            thread_id: r.thread_id.trim(),
            excerpt: &r.excerpt.texto,
            excerpt_crudo: &crudo,
        };
        reemplazar_cache(
            pool,
            &firma_cache,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            &foto,
            gen.coste,
        )
        .await?;
        /* [09AA-30 F2] Regenerar es gesto explícito: pisa la respuesta compartida
         * del inmueble (también una corrección previa). */
        if let (Some(clave), Some(n)) = (&clave_compartida, nombre.as_deref()) {
            if ocurrencias_nombre(&gen.texto, n) <= 1 {
                enlazar_compartida(
                    pool,
                    &firma_cache,
                    clave,
                    &plantilla_de_nombre(&gen.texto, n),
                    gen.coste,
                    true,
                )
                .await?;
            }
        }
    }
    Ok(BorradorResponse {
        borrador: gen.texto,
        fuente: gen.fuente,
        aviso_conocido: conocido,
        firma_version,
        corregida: false,
        coste: gen.coste,
    })
}

