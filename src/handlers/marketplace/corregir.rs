//! Releer la foto del hilo y corregir el borrador ya generado (`releer`, `corregir`).

// Mismos imports que la cabecera del hub (marketplace.rs); el glob es intencional.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReleerRequest {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    /* Texto plano (no `ExcerptIn`: Releer no necesita remitente ni hora,
     * solo refresca la foto del hilo). */
    #[serde(default)]
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReleerResponse {
    pub actualizado: bool,
    /// [08AA-28] `true` si no había fila y se creó solo-foto (sin borrador).
    pub creado: bool,
}

/// Releer explícito de la dueña (botón separado de Regenerar, 08AA-9):
/// refresca la foto del hilo (`excerpt_texto`) sin generar ni tocar el
/// borrador guardado. Sin IA, sin invalidar caché: `UPDATE` por
/// `thread_id` (todas las firmas del hilo comparten la foto nueva).
/// [08AA-28] Sin fila del hilo → se crea solo-foto (`respuesta` vacía,
/// `creado:true`) para que el chat aparezca en el panel sin inventar
/// borrador. Mismo tope barato del borrador.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/releer",
    request_body = ReleerRequest,
    responses(
        (status = 200, description = "Foto del hilo refrescada o creada solo-foto", body = ReleerResponse),
        (status = 422, description = "Schema inválido", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope por sub", body = crate::errors::ErrorResponse)
    )
)]
pub async fn releer(
    State(state): State<AppState>,
    auth: MpAuth,
    r: Result<Json<ReleerRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    if !sub_exento(&auth.sub)
        && !consumir_minuto(
            &state.pool,
            &format!("releer:{}", auth.sub),
            TOPE_BORRADOR_MINUTO,
        )
        .await?
    {
        return Ok(limite(60));
    }
    let mut r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    if r.thread_id.trim().is_empty() {
        return Err(AppError::Validation("threadId requerido".to_string()));
    }
    let n = r.excerpt.chars().count();
    if n == 0 || n > 2000 {
        return Err(AppError::Validation(
            "excerpt 1..2000 caracteres".to_string(),
        ));
    }
    /* [08AA-5] Igual que en `borrador`: lo que se guarda es el excerpt
     * limpio, nunca el ruido crudo del DOM. [08AA-16] Con contexto.
     * [08AA-18] Guarda y busca con `clave_hilo()`: la cifra inyectada
     * por el puente (07AA-11) parpadea entre el `/borrador` y el
     * `/releer` y el `thread_id` literal no empareja (`actualizado=false`
     * en silencio).
     * [08AA-21] El crudo también se guarda (`excerpt_crudo`), para
     * calibrar el filtro (08AA-8): lo limpio al panel, lo crudo al
     * diagnóstico. */
    let hilo = clave_hilo(r.thread_id.trim());
    let crudo = r.excerpt.clone();
    let limpio = normalizar_excerpt_hilo(&hilo, &r.excerpt);
    if limpio.is_empty() {
        return Err(AppError::Validation(
            "excerpt sin contenido aprovechable".to_string(),
        ));
    }
    r.excerpt = limpio;
    // [08AA-28] Upsert atómico en el servicio: si no hay fila la crea
    // solo-foto para que el chat aparezca en el panel (caché borrada).
    let (actualizado, creado) = releer_foto(&state.pool, &hilo, &r.excerpt, &crudo).await?;
    Ok((
        StatusCode::OK,
        Json(ReleerResponse {
            actualizado,
            creado,
        }),
    )
        .into_response())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CorregirRequest {
    pub firma: String,
    pub firma_version: String,
    #[serde(rename = "avisoId")]
    pub aviso_id: Option<String>,
    pub texto: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CorregirResponse {
    pub corregida: bool,
}

/// Guarda la corrección de la dueña (`corregida=TRUE`, vigencia +90d). Cubo
/// propio 30/min (escritura barata pero no gratis); la matriz vale también
/// para su texto (422 con motivo si trae contacto).
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/corregir",
    request_body = CorregirRequest,
    responses(
        (status = 200, description = "Corrección guardada", body = CorregirResponse),
        (status = 422, description = "Texto inválido o con contacto", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope por sub", body = crate::errors::ErrorResponse)
    )
)]
pub async fn corregir(
    State(state): State<AppState>,
    auth: MpAuth,
    r: Result<Json<CorregirRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    if !sub_exento(&auth.sub)
        && !consumir_minuto(
            &state.pool,
            &format!("corr:{}", auth.sub),
            TOPE_BORRADOR_MINUTO,
        )
        .await?
    {
        return Ok(limite(60));
    }
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    if r.firma.len() != 64 || !r.firma.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Validation("firma debe ser hex64".to_string()));
    }
    if r.firma_version != "firma-v1" && r.firma_version != FIRMA_VERSION_V2 {
        return Err(AppError::Validation(
            "firma_version debe ser firma-v1 o firma-v2".to_string(),
        ));
    }
    let (_, precio_hash, catalog_hash, _) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), None).await?;
    corregir_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash, &r.texto).await?;
    /* [09AA-30 F2] La corrección de la dueña vale para todo el inmueble: se
     * aplica a la respuesta compartida con el nombre del hilo que la corrige. */
    if let Some((thread, mensaje)) =
        vinculo_compartido(&state.pool, &r.firma, &precio_hash, &catalog_hash).await?
    {
        if let Some(n) = nombre_de_thread(&thread) {
            let clave = ClaveCompartida {
                catalog_hash: &catalog_hash,
                precio_hash: &precio_hash,
                mensaje_clave: &mensaje,
            };
            corregir_compartida(&state.pool, &clave, &plantilla_de_nombre(&r.texto, &n)).await?;
        }
    }
    Ok((StatusCode::OK, Json(CorregirResponse { corregida: true })).into_response())
}

