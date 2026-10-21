//! Generación del borrador (`POST /borrador`): tipos de respuesta, caché, idempotencia y prompt.

// Mismos imports que la cabecera del hub (marketplace.rs); el glob es intencional.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::services::marketplace::continuacion;
use chrono::Timelike as _;

/* [16AA-6] Era 30/min: las burbujas de la lab piden en ráfagas de ~90/min y
 * a partir de ahí caían a su plantilla local, que da peor borrador sin avisar.
 * El tope sigue existiendo para que nadie abuse del gasto de IA. */
pub(super) const TOPE_BORRADOR_MINUTO: i64 = 90;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BorradorResponse {
    pub borrador: String,
    pub fuente: String,
    pub aviso_conocido: bool,
    pub firma_version: String,
    /// `true` si el texto es corrección de la dueña (vía `corregir`).
    pub corregida: bool,
    /// [09AA-30] Coste de la pasada IA de este texto; vacío en `cache` (un
    /// hit no gasta IA, el coste original vive en la fila).
    pub coste: crate::services::marketplace::Coste,
}

/* F0 estructuradas/idempotencia → `marketplace_estructuradas.rs`
 * (`FuenteBorrador`, `resolver_fuente`, `fuente_v1/v2`,
 * `clave_idempotencia`, `con_idempotencia`): aquí solo wiring. */

/// Genera el borrador: valida schema (422), resuelve la ficha por UUID y la
/// stripea (sin ficha → no se afirma precio), consulta la caché (hit → sin
/// gastar IA) y si es miss genera con singleflight (un doble clic = una IA)
/// y guarda. Solo se cachea `fuente=ia`; el fallback nunca (ver M4).
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/borrador",
    request_body = BorradorRequest,
    responses(
        (status = 200, description = "Borrador listo (cache, ia o reserva)", body = BorradorResponse),
        (status = 422, description = "Schema inválido", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope por sub", body = crate::errors::ErrorResponse)
    )
)]
pub async fn borrador(
    State(state): State<AppState>,
    auth: MpAuth,
    headers: HeaderMap,
    r: Result<Json<BorradorRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    if !sub_exento(&auth.sub)
        && !consumir_minuto(
            &state.pool,
            &format!("bor:{}", auth.sub),
            TOPE_BORRADOR_MINUTO,
        )
        .await?
    {
        return Ok(limite(60));
    }
    let mut r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* [09AA-20] `Idempotency-Key` opcional (422 si es basura) + F0: la
     * estructurada deja `excerpt.texto` renderizado y su firma v2; el texto
     * plano sigue el camino de siempre (`fuente_v1`: limpia excerpt,
     * conserva el original si solo había ruido, crudo para calibrar).
     * [09AA-22] F3: la llave se verifica contra hint+firma (422 si es de
     * otro hilo). */
    let clave_idem = clave_idempotencia(&headers)?;
    let fuente = resolver_fuente(&mut r)?;
    verificar_idempotencia_conversacion(&r, &fuente, clave_idem.as_ref())?;
    let FuenteBorrador {
        firma,
        firma_version,
        crudo,
        mensaje_clave,
    } = fuente;
    let titulo_fb = aviso_fb_de_thread(r.thread_id.trim());
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), titulo_fb.as_deref()).await?;
    /* [09AA-30 F2] Caché compartida por inmueble: solo con los 2 primeros
     * mensajes del cliente y aviso conocido; sin nombre del hilo no se puede
     * rellenar `{{nombre}}`, así que ese hilo va por su camino de siempre.
     * [10AA-28] Además solo para apertura: con respuesta de la dueña en el
     * hilo, la plantilla de otro hilo con el mismo primer mensaje no encaja
     * (y la continuación no debe publicarse como plantilla). */
    let nombre = nombre_de_thread(r.thread_id.trim());
    /* [11AA] Caché por firma, sin nombre: la fila guarda solo el cuerpo y el
     * saludo (con el nombre de este hilo) se añade al servir (`servir_cuerpo`). */
    let clave_firma = clave_cache_hilo(&firma);
    let clave_compartida = mensaje_clave
        .as_deref()
        .filter(|_| conocido && nombre.is_some() && continuacion::es_apertura(&r.excerpt.texto))
        .map(|mensaje_clave| ClaveCompartida {
            catalog_hash: &catalog_hash,
            precio_hash: &precio_hash,
            mensaje_clave,
        });
    let compartida = clave_compartida.as_ref().zip(nombre.as_deref());
    let foto = FotoHilo {
        thread_id: r.thread_id.trim(),
        excerpt: &r.excerpt.texto,
        excerpt_crudo: &crudo,
    };
    let cl = ClavesBorrador {
        firma_cache: &clave_firma,
        precio_hash: &precio_hash,
        catalog_hash: &catalog_hash,
    };
    /* [10AA-17] Regenerar (`force`): solo una corrección de la dueña corta el
     * paso; cualquier otro hit se descarta y el texto se regenera de verdad. */
    let hit = aceptar_hit(
        buscar_hit(&state.pool, &cl, &foto, compartida).await?,
        r.force,
    );
    if let Some((texto, corregida)) = hit {
        /* Hit: el plugin audita `hit`; aquí no se audita nada (el conteo de
         * usos ya subió en la misma sentencia del `UPDATE ... RETURNING`). */
        log_borrador_cache(r.thread_id.trim(), corregida);
        let resp = respuesta_hit(
            servir_cuerpo(&texto, &r),
            corregida,
            conocido,
            &firma_version,
        );
        return Ok(con_idempotencia(resp, clave_idem.as_ref()));
    }
    /* Miss (el plugin audita `miss`): una sola IA por clave en vuelo. La
     * llave de idempotencia entra al vuelo para que un reintento colapse
     * con el original en vez de disparar otra IA. */
    let huella_idem = clave_idem.as_deref().unwrap_or("sin-clave");
    let clave_vuelo = format!("{clave_firma}:{precio_hash}:{catalog_hash}:{huella_idem}");
    /* [09AA-5] Latencia real de la pasada para la tab de Logs. */
    let inicio = std::time::Instant::now();
    let gen = state
        .mp_vuelo
        .ejecutar(&clave_vuelo, || {
            generar_borrador(&r, seguro.as_ref(), &state.pool)
        })
        .await;
    /* `u128` sin cast: `serde_json` no lo representa; si algún día no
     * cupiera en `u64`, se satura en vez de envolver. */
    let latencia_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);
    log_borrador_ia(
        r.thread_id.trim(),
        gen.fuente.as_str(),
        latencia_ms,
        conocido,
    );
    if gen.fuente == "ia" {
        guardar_generado(&state.pool, &cl, &foto, &gen, compartida, r.force).await?;
    }
    let resp = (
        StatusCode::OK,
        Json(BorradorResponse {
            borrador: if gen.fuente == "ia" {
                servir_cuerpo(&gen.texto, &r)
            } else {
                gen.texto.clone()
            },
            fuente: gen.fuente.clone(),
            aviso_conocido: conocido,
            firma_version,
            corregida: false,
            coste: gen.coste,
        }),
    )
        .into_response();
    Ok(con_idempotencia(resp, clave_idem.as_ref()))
}

/// Respuesta de un hit de caché: `fuente=cache` y coste vacío (no gastó IA).
fn respuesta_hit(texto: String, corregida: bool, conocido: bool, firma_version: &str) -> Response {
    (
        StatusCode::OK,
        Json(BorradorResponse {
            borrador: texto,
            fuente: "cache".to_string(),
            aviso_conocido: conocido,
            firma_version: firma_version.to_string(),
            corregida,
            coste: crate::services::marketplace::Coste::default(),
        }),
    )
        .into_response()
}

/// Claves de caché de una petición ya resueltas: llave de la fila del hilo
/// (`clave_cache_hilo`: solo la firma) y hashes de precio y catálogo.
pub(super) struct ClavesBorrador<'a> {
    pub(super) firma_cache: &'a str,
    pub(super) precio_hash: &'a str,
    pub(super) catalog_hash: &'a str,
}

/// [10AA-17] Un hit cuenta si no hay Regenerar o si es corrección de la dueña
/// (Regenerar nunca la pisa). Sin `force` devuelve el hit tal cual.
pub(super) fn aceptar_hit(hit: Option<(String, bool)>, force: bool) -> Option<(String, bool)> {
    hit.filter(|(_, corregida)| *corregida || !force)
}

/// Lookup de caché de `borrador`: primero la respuesta compartida del inmueble
/// (con `{{nombre}}` rellenado), luego la fila del hilo.
pub(super) async fn buscar_hit(
    pool: &sqlx::PgPool,
    cl: &ClavesBorrador<'_>,
    foto: &FotoHilo<'_>,
    compartida: Option<(&ClaveCompartida<'_>, &str)>,
) -> Result<Option<(String, bool)>, AppError> {
    if let Some((clave, nombre)) = compartida {
        if let Some((plantilla, corregida_compartida)) = buscar_compartida(pool, clave).await? {
            if let Some(texto) = rellenar_nombre(&plantilla, Some(nombre)) {
                vincular_hilo_compartido(
                    pool,
                    cl.firma_cache,
                    clave,
                    &texto,
                    corregida_compartida,
                    foto,
                )
                .await?;
                return Ok(Some((texto, corregida_compartida)));
            }
        }
    }
    buscar_cache(pool, cl.firma_cache, cl.precio_hash, cl.catalog_hash).await
}

/// Persiste un borrador de IA: caché del hilo y, si el nombre aparece a lo
/// sumo una vez, plantilla compartida del inmueble. `pisar` (Regenerar) pisa
/// la fila y la plantilla en vez de respetar la que ya hubiera.
pub(super) async fn guardar_generado(
    pool: &sqlx::PgPool,
    cl: &ClavesBorrador<'_>,
    foto: &FotoHilo<'_>,
    gen: &crate::services::marketplace::Generado,
    compartida: Option<(&ClaveCompartida<'_>, &str)>,
    pisar: bool,
) -> Result<(), AppError> {
    if pisar {
        reemplazar_cache(
            pool,
            cl.firma_cache,
            cl.precio_hash,
            cl.catalog_hash,
            &gen.texto,
            foto,
            gen.coste,
        )
        .await?;
    } else {
        guardar_cache(
            pool,
            cl.firma_cache,
            cl.precio_hash,
            cl.catalog_hash,
            &gen.texto,
            foto,
            gen.coste,
        )
        .await?;
    }
    /* [09AA-30 F2] Publica la respuesta para el inmueble solo si el nombre
     * aparece a lo sumo una vez (`{{nombre}}` sustituye una sola ocurrencia).
     * [10AA-17] Con `pisar` también se pisa la plantilla: si no, el siguiente
     * borrador del hilo (sin force) la reenlazaría con el texto viejo. */
    if let Some((clave, n)) = compartida {
        if ocurrencias_nombre(&gen.texto, n) <= 1 {
            enlazar_compartida(
                pool,
                cl.firma_cache,
                clave,
                &plantilla_de_nombre(&gen.texto, n),
                gen.coste,
                pisar,
            )
            .await?;
        }
    }
    Ok(())
}

/// [11AA] Franja del saludo en el instante dado, en Venezuela (UTC-4).
fn franja_en(instante: chrono::DateTime<chrono::Utc>) -> &'static str {
    let ven = instante - chrono::Duration::hours(4);
    crate::handlers::chat::periodo_venezuela(ven.hour()).1
}

/// [11AA] Franja del saludo según la hora de Venezuela AHORA. No se usa la hora
/// del mensaje del cliente: un borrador cacheado de noche se leía de día como
/// «buenas noches» (la hora del mensaje, no la de envío).
fn franja_actual() -> &'static str {
    franja_en(chrono::Utc::now())
}

/// [11AA] Antepone «Hola, <nombre>, <franja>.» al cuerpo. El saludo no pasa por
/// el modelo: depende del hilo (nombre) y de la hora de servir, así que no
/// puede vivir en la caché compartida por firma.
pub(super) fn con_saludo(cuerpo: &str, nombre: Option<&str>, franja: &str) -> String {
    let mut saludo = String::from("Hola");
    if let Some(n) = nombre.map(str::trim).filter(|n| !n.is_empty()) {
        saludo.push_str(", ");
        saludo.push_str(n);
    }
    saludo.push_str(", ");
    saludo.push_str(franja);
    format!("{saludo}. {cuerpo}")
}

/// [11AA] Lo que recibe la dueña: el cuerpo cacheado con el saludo de este
/// hilo y la franja de la hora actual. Quita antes cualquier saludo que traiga
/// (filas viejas lo llevaban). Las continuaciones nunca llevan saludo.
pub(super) fn servir_cuerpo(cuerpo: &str, r: &BorradorRequest) -> String {
    if continuacion::es_continuacion(&r.excerpt.texto) {
        return cuerpo.to_string();
    }
    let nombre = nombre_de_thread(r.thread_id.trim());
    let sin_saludo = crate::services::marketplace::quitar_saludo(cuerpo, nombre.as_deref());
    con_saludo(&sin_saludo, nombre.as_deref(), franja_actual())
}

/// Ficha o precio del aviso y lo ya dicho en el hilo: el contexto de datos del prompt.
pub(super) async fn datos_y_ya_dicho(
    r: &BorradorRequest,
    seguro: Option<&crate::services::marketplace::PromptSeguro>,
    aviso: &str,
    pool: &sqlx::PgPool,
) -> (String, String) {
    use crate::services::marketplace::{hilo_previo, precio_del_aviso};
    /* [07AA-9] En el piloto no hay ficha, pero el título del aviso sí puede
     * traer el precio publicado (`125.000$`): se extrae y se entrega como
     * dato conocido para que la IA lo dé directo en vez del fallback. */
    let precio_aviso = precio_del_aviso(aviso);
    let datos = match (&seguro, &precio_aviso) {
        (Some(s), _) => serde_json::to_string(s).unwrap_or_else(|_| "SIN FICHA".to_string()),
        (None, Some(p)) => format!(
            "Precio publicado en el aviso: {p}. Sin ficha: no afirmes medidas, ubicación exacta ni otros datos; el precio del aviso sí dalo directo."
        ),
        (None, None) => {
            "SIN FICHA: no conoces el inmueble; no afirmes precio ni medidas.".to_string()
        }
    };
    /* [07AA-8] Lo ya dicho en este hilo: la IA avanza, no repite. Si la BD
     * falla aquí, se genera sin contexto (nunca se bloquea el borrador). */
    let previas = hilo_previo(pool, r.thread_id.trim())
        .await
        .unwrap_or_default();
    let ya_dicho = if previas.is_empty() {
        "nada todavía".to_string()
    } else {
        previas.join(" / ")
    };
    (datos, ya_dicho)
}

/// Prompt de sistema del borrador: tono, datos del inmueble y forma del mensaje (sin saludo).
pub(super) fn sistema_borrador(
    r: &BorradorRequest,
    aviso: &str,
    datos: &str,
    ya_dicho: &str,
) -> String {
    use crate::services::marketplace::{CONTACTO_TEL, CONTACTO_WA};
    let tono = crate::services::marketplace::continuacion::tono_de(r);
    /* [07AA-10] Saludo primero y por su nombre: el hilo trae `nombre|aviso`.
     * El precio según operación ([08AA-25]: un alquiler presentado como
     * venta rompe la confianza — testigo Townhouse Arivana, hilo cristo);
     * el cierre invita a contar qué busca (conocer intención, no solo
     * coordinar visita).
     * [08AA-31] Sin promesa de visita (hilo angelv: el borrador decía
     * "Sí, puedes visitarla y te coordinamos" sin saber la
     * disponibilidad real de la dueña — decir poco es mejor en el primer
     * mensaje): disponible ≠ visitable; la visita se confirma con ella.
     * [08AA-11] Párrafos separados por línea en blanco, no líneas sueltas:
     * el borrador se copia a WhatsApp y los saltos sueltos se ven rotos.
     * [08AA-15] Breve por pedido de ella: 3 párrafos cortos como máximo,
     * nombre corto del inmueble (tipo + residencia, sin dirección ni zona
     * duplicada) y sin párrafo de relleno ("sigue disponible y con gusto…"
     * ya va dicho en la apertura). */
    /* [08AA-36] Sin anuncio de confirmación con la dueña (mensaje de ella
     * 2026-10-08: el borrador decía «lo confirmo con la dueña» dos veces —
     * el prompt lo ORDENABA («di solo que está disponible y que lo
     * confirmas con ella», testigo fila olear). Ahora: el dato se da una
     * sola vez, prohibido «confirmo», «te confirmo su estatus» o cualquier
     * meta-comentario de coordinación; sin precio solo vale el FALLBACK
     * exacto.
     * [08AA-37] Nombre autoritativo + sin re-afirmar (mensajes de ella
     * 2026-10-08: saludó «Ordaz» con hilo `lidia|...` — tomó el apellido/
     * lugar del excerpt en vez del nombre del hilo; y el 2º párrafo repetía
     * lo del 1º: «Sí, se mantiene publicada en venta al momento»).
     * Ahora: el nombre del hilo es el único válido y el 2º párrafo jamás
     * reafirma disponibilidad/precio ni usa jerga interna.
     * [08AA-38] Regla estructural del 2º párrafo (mensaje de ella 2026-10-08:
     * «Sí, la publicación sigue vigente» burló la lista de frases de 08AA-37
     * con un sinónimo — fila sicilia v1, borrada en la limpieza 20:44).
     * Ahora: lógica condicional (pregunta ya respondida arriba → avanzar,
     * no responder) + veto por PALABRAS, no por frases.
     * [09AA-2] Raíz del bucle (4 subagentes 2026-10-09): el prompt mismo
     * ORDENABA el relleno («avanza la conversación: ofrece fotos o pregunta
     * qué busca») y esa orden positiva siempre le ganó al veto; además
     * «máximo 3» + 3 roles obligatorios se leía como «exactamente 3», y los
     * comentarios [08AA-*] nunca viajan al modelo. Ahora: formato de DOS
     * bloques + excepción de una línea, y la invariante la impone
     * `imponer_forma_borrador` en Rust aunque el modelo desobedezca. */
    format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Tono {tono}, BREVE: el mensaje son DOS párrafos (primero + final) \
         y, solo si aplica la excepción de abajo, UNA línea intermedia; \
         cada bloque va en su párrafo separado por una línea en blanco \
         (nada de líneas sueltas). \
         Datos del inmueble: {datos}. \
         Aviso en Facebook: {aviso}. \
         Sin saludo: no escribas «Hola», «buenos días», «buenas tardes» ni \
         «buenas noches», ni el nombre del cliente; el sistema antepone el \
         saludo. \
         La conversación trae marcas: `Cliente:` es el comprador, `Dueña:` \
          es la dueña (tú no eres la dueña: no repitas lo que ella ya dijo). \
           Formato obligatorio, en este orden exacto: primer párrafo = UNA sola \
          frase, con uno de estos dos moldes: «<nombre_corto> sigue disponible en \
          <precio>$ negociable.» si la cifra es la de venta, o «El canon mensual \
          es de <precio>$ para <nombre_corto>.» si es el canon de alquiler; nunca \
          el molde de venta para un canon, porque «sigue disponible en 2.800$» se \
          lee como si el inmueble valiera eso. El sujeto es `nombre_corto` tal \
          cual, copiado de los datos: no lo compongas ni le añadas nada del \
          título (ni «planta baja con patio» ni la dirección), que suena a ficha \
          y no a conversación. Sin «te hablo de», «sí \
          sigue» ni «te escribo»; sin visitas, coordinación ni confirmar nada con \
          la dueña. La cifra exacta de los datos o del aviso, con el $ al final \
          (35.000$). El mismo inmueble puede \
          estar en venta y en alquiler a la vez: `precio_publico` es la cifra de \
          `operacion`, y si la descripción de los datos trae otra (p. ej. \
          «Alquiler: $2.800») y el Cliente pregunta por esa operación, da esa \
          diciendo «canon mensual»; \
          segundo bloque = este texto literal, sin cambiar ni \
          una palabra: «Cuéntame qué estás buscando y con gusto te ayudo. \
          Cualquier cosa escríbeme al {CONTACTO_TEL}» y en línea aparte {CONTACTO_WA}. \
          Excepción: un párrafo intermedio de UNA sola frase (máximo 140 \
          caracteres, sin ? ni ¿) SOLO si el Cliente pide un dato concreto \
          no dicho arriba (baños, habitaciones, m2). Si el Cliente pregunta \
          la ubicación, ese párrafo es «El inmueble se ubica en <dirección o \
          zona exacta de los datos>.» con la dirección completa, nunca cortada \
          ni empezando por una abreviatura o fragmento suelto. Sin pregunta \
          concreta, OMITE el párrafo: nada de transiciones, fotos, preguntas \
          ni reafirmaciones con ninguna palabra. Prohibido fuera del bloque \
          final: ? ¿ fotos disponible precio visitas cifras contacto propio; \
          sin precio en datos ni aviso, no lo inventes; \
         si preguntan precio y no hay precio en los datos ni en el aviso, responde exactamente: {FALLBACK_BORRADOR} \
         (el sistema agrega el contacto y el enlace al final). \
          Ya le dijiste (no lo repitas igual): {ya_dicho}"
    )
}

pub(super) async fn generar_borrador(
    r: &BorradorRequest,
    seguro: Option<&crate::services::marketplace::PromptSeguro>,
    pool: &sqlx::PgPool,
) -> crate::services::marketplace::Generado {
    use crate::services::marketplace::{aviso_fb_de_thread, Generado};
    /* [07AA-8] El aviso de Facebook viaja en el hilo (`comprador|aviso`):
     * contexto aproximado para abrir con la ficha breve en el piloto. */
    let aviso = aviso_fb_de_thread(r.thread_id.trim()).unwrap_or_else(|| "desconocido".to_string());
    let (datos, ya_dicho) = datos_y_ya_dicho(r, seguro, &aviso, pool).await;
    /* [10AA-25] La dueña ya contestó y el último mensaje es del cliente: se
     * responde solo a ese mensaje, sin molde de apertura ni contacto fijo. */
    let cont = continuacion::es_continuacion(&r.excerpt.texto);
    let sistema = if cont {
        continuacion::sistema_continuacion(continuacion::tono_de(r), &aviso, &datos, &ya_dicho)
    } else {
        sistema_borrador(r, &aviso, &datos, &ya_dicho)
    };
    /* [09AA-4] Sesión estable por hilo (hash, jamás PII en claro): el relay
     * exige `x-opencode-session` y premia la estabilidad con ruteo afin y
     * prompt caching. */
    let sesion_hilo = sha_hex(&clave_hilo(r.thread_id.trim()));
    /* [09AA-15] Vía rápida sin razonamiento (pedido de ella por los 75 s):
     * si no trae texto cae sola a la estándar; la forma la sigue
     * imponiendo `imponer_forma_borrador` abajo. */
    /* [09AA-30] Coste de la pasada: tiempo de la llamada IA (no del vuelo
     * ni del fallback) y tokens del `usage`; `None` si el relay no los trajo. */
    let inicio_ia = std::time::Instant::now();
    let (texto, uso) = match crate::handlers::ia::rutas::completar_opencode_rapido(
        &sistema,
        &r.excerpt.texto,
        &[],
        &sesion_hilo,
    )
    .await
    {
        Ok((t, _, uso)) => (t, uso),
        Err(e) => {
            tracing::warn!("borrador mp: IA caída ({e}), va fallback");
            let texto = if cont {
                continuacion::FALLBACK_CONTINUACION.to_string()
            } else {
                /* [20AA-1] La reserva pasa por la forma canónica: con la ficha
                 * en `datos`, un relay caído no puede llevarse el precio. */
                crate::services::marketplace::borrador_de_reserva(seguro)
            };
            return Generado {
                texto,
                fuente: "reserva".to_string(),
                coste: crate::services::marketplace::Coste::default(),
            };
        }
    };
    let coste = crate::services::marketplace::Coste {
        tokens_entrada: uso.map(|u| u.entrada),
        tokens_salida: uso.map(|u| u.salida),
        ms: Some(i64::try_from(inicio_ia.elapsed().as_millis()).unwrap_or(i64::MAX)),
    };
    /* [09AA-2] La invariante de forma la impone Rust: el texto de la IA
     * pasa por `imponer_forma_borrador` (poda de relleno + final canónico)
     * antes de garantizar el contacto. La continuación no lleva ninguno. */
    /* [11AA] La caché guarda solo el cuerpo: si la IA escribió igual un saludo,
     * se quita aquí (`servir_cuerpo` lo añade de nuevo, con el nombre del hilo). */
    let nombre = nombre_de_thread(r.thread_id.trim());
    let sin_saludo = crate::services::marketplace::quitar_saludo(&texto, nombre.as_deref());
    let texto = if cont {
        continuacion::dar_forma_continuacion(&sin_saludo)
    } else {
        formatear_parrafos(&crate::services::marketplace::asegurar_contacto(
            &crate::services::marketplace::imponer_forma_borrador(
                &formatear_parrafos(&sin_saludo),
                seguro,
            ),
        ))
    };
    Generado {
        texto,
        fuente: "ia".to_string(),
        coste,
    }
}

#[cfg(test)]
mod tests {
    use super::{con_saludo, franja_en};
    use crate::services::marketplace::quitar_saludo;

    fn utc(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .expect("fecha de prueba válida")
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn franja_sigue_la_hora_de_venezuela() {
        /* 12:00 UTC son las 08:00 en Venezuela (UTC-4). */
        assert_eq!(franja_en(utc("2026-10-11T12:00:00Z")), "buenos días");
        assert_eq!(franja_en(utc("2026-10-11T18:00:00Z")), "buenas tardes");
        /* 02:00 UTC del día siguiente son las 22:00 del día anterior en Venezuela. */
        assert_eq!(franja_en(utc("2026-10-12T02:00:00Z")), "buenas noches");
        /* 12:30 UTC son las 08:30 en Venezuela. */
        assert_eq!(franja_en(utc("2026-10-11T12:30:00Z")), "buenos días");
    }

    #[test]
    fn saludo_con_nombre_y_franja() {
        assert_eq!(
            con_saludo(
                "El apartamento sigue disponible.",
                Some("Ana"),
                "buenos días"
            ),
            "Hola, Ana, buenos días. El apartamento sigue disponible."
        );
    }

    #[test]
    fn saludo_sin_nombre() {
        assert_eq!(
            con_saludo("Cuerpo.", None, "buenas noches"),
            "Hola, buenas noches. Cuerpo."
        );
    }

    #[test]
    fn el_saludo_añadido_se_quita_al_releer() {
        let cuerpo = "El apartamento sigue disponible.";
        let servido = con_saludo(cuerpo, Some("Ana"), "buenos días");
        assert_eq!(quitar_saludo(&servido, Some("Ana")), cuerpo);
    }

    #[test]
    fn filas_viejas_con_saludo_se_limpian() {
        assert_eq!(
            quitar_saludo(
                "Hola, Ana, buenas tardes. El apartamento sigue disponible.",
                Some("Ana")
            ),
            "El apartamento sigue disponible."
        );
    }
}
