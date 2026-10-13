use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::models::url_publica_de_foto;
use crate::repositories::InmuebleRepository;

use crate::services::marketplace::{
    aviso_fb_de_thread, clave_hilo, mejor_puntaje_con_alias, Coste,
};

/// Fila del dashboard M2: conteos por día y evento. Sin PII: el HMAC del hilo
/// jamás sale, solo día + evento + conteo.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UsoDia {
    pub dia: String,
    pub hit: i64,
    pub miss: i64,
    pub copiar: i64,
    pub regenerar: i64,
    pub emision: i64,
}

/// Agrega `mp_auditoria` en una sola consulta (`GROUP BY` día+evento, sin
/// N+1). `dias` se acota a 1..=90; la ventana es día calendario local del
/// servidor (misma base que `ts_hora` truncada a la hora).
pub async fn resumen_uso(pool: &sqlx::PgPool, dias: i32) -> Result<Vec<UsoDia>, AppError> {
    let dias = dias.clamp(1, 90);
    // `GROUP BY`/`ORDER BY` repiten la expresión: con el alias `"dia!"` el
    // nombre `dia` ya no existe en el SQL. Misma agrupación que antes.
    let filas = sqlx::query!(
        "SELECT ts_hora::date AS \"dia!\", evento, COUNT(*) AS \"n!\" \
         FROM mp_auditoria \
         WHERE ts_hora >= date_trunc('day', now()) - make_interval(days => $1) \
         GROUP BY ts_hora::date, evento ORDER BY ts_hora::date",
        dias,
    )
    .fetch_all(pool)
    .await?;
    let mut orden: Vec<String> = Vec::new();
    let mut por_dia: std::collections::HashMap<String, UsoDia> = std::collections::HashMap::new();
    for f in filas {
        let dia = f.dia;
        let evento = f.evento;
        let n = f.n;
        let clave = dia.format("%Y-%m-%d").to_string();
        let entrada = por_dia.entry(clave.clone()).or_insert_with(|| {
            orden.push(clave.clone());
            UsoDia {
                dia: String::new(),
                hit: 0,
                miss: 0,
                copiar: 0,
                regenerar: 0,
                emision: 0,
            }
        });
        entrada.dia.clone_from(&clave);
        match evento.as_str() {
            "hit" => entrada.hit = n,
            "miss" => entrada.miss = n,
            "copiar" => entrada.copiar = n,
            "regenerar" => entrada.regenerar = n,
            "emision" => entrada.emision = n,
            otro => tracing::warn!("resumen_uso: evento desconocido {otro}"),
        }
    }
    Ok(orden
        .into_iter()
        .filter_map(|d| por_dia.remove(&d))
        .collect())
}

/// [07AA-7] Panel por chat: un chat = un `thread_id` (= clave de ventana
/// del puente, trae nombre+aviso: PII solo-admin por decisión de ella
/// 2026-10-07, misma retención 90d + purga).
/// [09AA-21] `aviso_conocido`: el aviso del hilo empareja con una ficha
/// (ID exacto o título); el front lo usa para la vista de huérfanos.
/// [09AA-23] `inmueble_vinculado`: título de la ficha emparejada (`None` =
/// huérfano); el front lo muestra junto a su miniatura, o «Sin ficha».
/// [09AA-28] `inmueble_foto`: URL pública (`/uploads/…`) de la portada del
/// inmueble vinculado; `None` si no hay vínculo o la ficha no tiene fotos.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatResumen {
    pub thread_id: String,
    pub borradores: i64,
    pub usos: i64,
    pub corregidas: i64,
    pub ultimo: String,
    pub aviso_conocido: bool,
    pub inmueble_vinculado: Option<String>,
    pub inmueble_foto: Option<String>,
    /// [09AA-30 F4] Hilos que comparten esta respuesta (mismo inmueble, precio y
    /// `mensaje_clave`), contando el propio. 1 = no compartida.
    pub compartida_por: i64,
    /// [09AA-30 F4] Fecha del último borrador (`creada_en`). `None` si todas sus
    /// filas son anteriores a la columna. No es la hora del último mensaje.
    pub ultimo_borrador_en: Option<String>,
}

/// [09AA-28] Vínculos de aviso: `marketplace_id` → (id, título, alias).
pub(in crate::services::marketplace) type VinculosAviso =
    std::collections::HashMap<String, (uuid::Uuid, String, Vec<String>)>;

/// [07AA-7] Una fila del chat: foto de la conversación + texto guardado.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatFila {
    pub excerpt_texto: String,
    pub respuesta: String,
    pub usos: i64,
    pub corregida: bool,
    pub valida_hasta: String,
    /// [09AA-30] `ia` = texto generado; `releer` = solo foto; `None` = fila
    /// anterior a la migración 09AA-30 (origen desconocido).
    pub origen: Option<String>,
    pub coste: Coste,
}

/// [09AA-31] Una página del panel: `hay_mas` indica si queda otra tras
/// `chats`; `total` cuenta todos los hilos con borradores (sin archivados),
/// o solo los huérfanos cuando se pide `solo_huerfanos` (10AA-4).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginaResumen {
    pub chats: Vec<ChatResumen>,
    pub total: i64,
    pub hay_mas: bool,
}

/// [09AA-31] Cursor keyset: la página siguiente empieza tras el hilo
/// `(ultimo, thread_id)` de la anterior (orden: más reciente primero).
#[derive(Debug, Clone)]
pub struct CursorChats {
    pub ultimo: chrono::DateTime<chrono::Utc>,
    pub thread_id: String,
}

/// Títulos publicados y vínculos aviso→inmueble, una vez por página. Si una
/// consulta falla, se usa vacío y se registra: el panel jamás se bloquea.
async fn cargar_vinculos(
    pool: &sqlx::PgPool,
) -> (Vec<(uuid::Uuid, String, Vec<String>)>, VinculosAviso) {
    let candidatos: Vec<(uuid::Uuid, String, Vec<String>)> =
        match InmuebleRepository::titulos_alias_publicados(pool).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("resumen_chats: sin títulos publicados ({e}), avisos no conocidos");
                Vec::new()
            }
        };
    let vinculos: VinculosAviso = match InmuebleRepository::vinculos_publicados(pool).await {
        Ok(v) => v
            .into_iter()
            .map(|(aviso, id, titulo, alias)| (aviso, (id, titulo, alias)))
            .collect(),
        Err(e) => {
            tracing::warn!("resumen_chats: sin vínculos de aviso ({e}), solo título");
            std::collections::HashMap::new()
        }
    };
    (candidatos, vinculos)
}

/// Chats con borradores, ordenados por el más reciente, de `limite` en
/// `limite` (keyset, sin OFFSET). Una consulta para el total y dos para el
/// vínculo (títulos e IDs publicados, una vez, sin N+1): cada hilo resuelve en
/// memoria si su aviso es conocido.
/// [10AA-4] `solo_huerfanos`: solo hilos sin ficha conocida. El vínculo no vive
/// en SQL, así que se recorre por lotes hasta reunir `limite + 1` huérfanos.
/// Si las auxiliares fallan, todo queda `false` (el panel jamás se bloquea).
pub async fn resumen_chats(
    pool: &sqlx::PgPool,
    limite: i64,
    antes: Option<&CursorChats>,
    solo_huerfanos: bool,
) -> Result<PaginaResumen, AppError> {
    let tope = usize::try_from(limite).unwrap_or(usize::MAX);
    let (candidatos, vinculos) = cargar_vinculos(pool).await;
    /* Sin filtro: una consulta de `limite + 1` filas; la sobrante prueba que hay
     * otra página. Con filtro: lotes hasta llenar la página o agotar la BD. */
    let mut salida: Vec<(FilaChat, Option<(uuid::Uuid, String)>)> = Vec::new();
    let mut hay_mas = false;
    let mut tras: Option<(chrono::DateTime<chrono::Utc>, String)> =
        antes.map(|c| (c.ultimo, c.thread_id.clone()));
    'lotes: loop {
        let lote = if solo_huerfanos {
            LOTE_HUERFANOS
        } else {
            limite.saturating_add(1)
        };
        let leidas = filas_chats_tras(pool, lote, tras.as_ref()).await?;
        let agotado = i64::try_from(leidas.len()).unwrap_or(i64::MAX) < lote;
        for fila in leidas {
            tras = Some((fila.4, fila.0.clone()));
            let vinculado = titulo_vinculado_del_hilo(&fila.0, &candidatos, &vinculos);
            if solo_huerfanos && vinculado.is_some() {
                continue;
            }
            if salida.len() == tope {
                hay_mas = true;
                break 'lotes;
            }
            salida.push((fila, vinculado));
        }
        if agotado {
            break;
        }
    }
    /* [10AA-4] Con `solo_huerfanos` el total cuenta solo los huérfanos: el
     * vínculo se resuelve en memoria igual que en la lista. */
    let total = total_chats(pool, solo_huerfanos, &candidatos, &vinculos).await?;
    /* [09AA-28] Portadas de los inmuebles vinculados en una sola query
     * (únicos); si falla, el panel sigue sin miniaturas (jamás se bloquea). */
    let mut ids_vinculados: Vec<uuid::Uuid> = salida
        .iter()
        .filter_map(|(_, vinculado)| vinculado.as_ref().map(|(id, _)| *id))
        .collect();
    ids_vinculados.sort_unstable();
    ids_vinculados.dedup();
    let portadas = match InmuebleRepository::portadas_por_inmuebles(pool, &ids_vinculados).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("resumen_chats: sin portadas ({e}), chats sin miniatura");
            std::collections::HashMap::new()
        }
    };
    /* [09AA-30 F4] Fecha del último borrador y hilos que comparten respuesta, en
     * una sola query para la página; si falla, sin esos datos (jamás se bloquea). */
    let hilos_pagina: Vec<String> = salida.iter().map(|(fila, _)| fila.0.clone()).collect();
    let extras = match datos_extra_hilos(pool, &hilos_pagina).await {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("resumen_chats: sin fecha ni compartición ({e}), chats sin esos datos");
            std::collections::HashMap::new()
        }
    };
    let chats = salida
        .into_iter()
        .map(
            |((thread_id, borradores, usos, corregidas, ultimo), vinculado)| {
                let inmueble_foto = vinculado
                    .as_ref()
                    .and_then(|(id, _)| portadas.get(id))
                    .map(|clave| url_publica_de_foto(clave));
                let (ultimo_borrador, compartida_por) =
                    extras.get(&thread_id).copied().unwrap_or((None, 1));
                ChatResumen {
                    thread_id,
                    borradores,
                    usos,
                    corregidas,
                    ultimo: ultimo.to_rfc3339(),
                    aviso_conocido: vinculado.is_some(),
                    inmueble_vinculado: vinculado.map(|(_, titulo)| titulo),
                    inmueble_foto,
                    compartida_por,
                    ultimo_borrador_en: ultimo_borrador.map(|f| f.to_rfc3339()),
                }
            },
        )
        .collect();
    Ok(PaginaResumen {
        chats,
        total,
        hay_mas,
    })
}

/// Total de chats con borradores (sin archivados). Con `solo_huerfanos` cuenta
/// solo los hilos sin ficha conocida, con el mismo vínculo que la lista.
pub(in crate::services::marketplace) async fn total_chats(
    pool: &sqlx::PgPool,
    solo_huerfanos: bool,
    candidatos: &[(uuid::Uuid, String, Vec<String>)],
    vinculos: &VinculosAviso,
) -> Result<i64, AppError> {
    if solo_huerfanos {
        let hilos: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT thread_id FROM mp_respuestas_cache \
             WHERE origen IS DISTINCT FROM 'releer' \
             AND thread_id NOT IN (SELECT thread_id FROM mp_chats_archivados)",
        )
        .fetch_all(pool)
        .await?;
        let sin_ficha = hilos
            .iter()
            .filter(|h| titulo_vinculado_del_hilo(h, candidatos, vinculos).is_none())
            .count();
        return Ok(i64::try_from(sin_ficha).unwrap_or(i64::MAX));
    }
    /* [09AA-30 F4] Cuenta solo hilos con borrador: los «releer» (solo foto) no. */
    let total = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT thread_id)::BIGINT FROM mp_respuestas_cache \
         WHERE origen IS DISTINCT FROM 'releer' \
         AND thread_id NOT IN (SELECT thread_id FROM mp_chats_archivados)",
    )
    .fetch_one(pool)
    .await?;
    Ok(total)
}

/// [10AA-4] Hilos por lote al filtrar huérfanos: cuántos hilos se leen de la
/// caché por vuelta antes de volver a mirar el vínculo en memoria.
pub(in crate::services::marketplace) const LOTE_HUERFANOS: i64 = 100;

/// [10AA-4] Fila agregada por hilo: (`thread_id`, borradores, usos, corregidas,
/// último `valida_hasta`).
pub(in crate::services::marketplace) type FilaChat =
    (String, i64, i64, i64, chrono::DateTime<chrono::Utc>);

/// [10AA-4] Hasta `lote` hilos tras el cursor `(ultimo, thread_id)`, en el mismo
/// orden que el panel. Excluye los archivados, igual que `total`.
pub(in crate::services::marketplace) async fn filas_chats_tras(
    pool: &sqlx::PgPool,
    lote: i64,
    tras: Option<&(chrono::DateTime<chrono::Utc>, String)>,
) -> Result<Vec<FilaChat>, AppError> {
    let (ultimo, hilo) = match tras {
        Some((u, h)) => (Some(*u), Some(h.as_str())),
        None => (None, None),
    };
    let filas: Vec<FilaChat> = sqlx::query!(
        "SELECT thread_id, COUNT(*) FILTER (WHERE origen IS DISTINCT FROM 'releer')::BIGINT AS \"borradores!\", COALESCE(SUM(usos), 0)::BIGINT AS \"usos!\", \
         SUM(CASE WHEN corregida THEN 1 ELSE 0 END)::BIGINT AS \"corregidas!\", MAX(valida_hasta) AS \"ultimo!\" \
         FROM mp_respuestas_cache \
         WHERE thread_id NOT IN (SELECT thread_id FROM mp_chats_archivados) \
         GROUP BY thread_id \
         HAVING $1::TIMESTAMPTZ IS NULL OR (MAX(valida_hasta), thread_id) < ($1, $2) \
         ORDER BY MAX(valida_hasta) DESC, thread_id DESC \
         LIMIT $3",
        ultimo,
        hilo,
        lote,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.thread_id, r.borradores, r.usos, r.corregidas, r.ultimo))
    .collect();
    Ok(filas)
}

/// [09AA-30 F4] Por hilo de la página: fecha del último borrador (`creada_en`) y
/// hilos que comparten su respuesta (mismo inmueble, precio y `mensaje_clave`).
/// Una query para todos, comprobada en compilación (`.sqlx`); las filas
/// «releer» no cuentan. Sin `mensaje_clave` la compartición es 1.
pub(in crate::services::marketplace) async fn datos_extra_hilos(
    pool: &sqlx::PgPool,
    hilos: &[String],
) -> Result<
    std::collections::HashMap<String, (Option<chrono::DateTime<chrono::Utc>>, i64)>,
    sqlx::Error,
> {
    if hilos.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let filas = sqlx::query!(
        "SELECT thread_id AS \"thread_id!\", MAX(creada_en) AS \"ultimo\", MAX(compartida)::BIGINT AS \"compartida\" FROM ( \
           SELECT c.thread_id, c.creada_en, \
             CASE WHEN c.mensaje_clave IS NULL THEN 1 ELSE ( \
               SELECT COUNT(DISTINCT o.thread_id) FROM mp_respuestas_cache o \
               WHERE o.catalog_hash = c.catalog_hash AND o.precio_hash = c.precio_hash \
                 AND o.mensaje_clave = c.mensaje_clave AND o.origen IS DISTINCT FROM 'releer' \
             ) END AS compartida \
           FROM mp_respuestas_cache c \
           WHERE c.thread_id = ANY($1) AND c.origen IS DISTINCT FROM 'releer' \
         ) t GROUP BY thread_id",
        hilos,
    )
    .fetch_all(pool)
    .await?;
    Ok(filas
        .into_iter()
        .map(|r| (r.thread_id, (r.ultimo, r.compartida.unwrap_or(1))))
        .collect())
}

/* [09AA-21] ¿El aviso del hilo empareja con una ficha? Rama exacta primero:
 * el texto tras `|` son dígitos 5–32 vinculados; si no, emparejado por título
 * (misma regla que `ficha_por_titulo`: directo o solape ≥3 con distintiva,
 * empate = no conocido). Pura en memoria (sin BD).
 * [09AA-23] Devuelve el título emparejado (rama exacta: el título del
 * vínculo; rama título: el título candidato). `resumen_chats` deriva
 * `aviso_conocido` como `vinculado.is_some()`.
 * [09AA-24] Las ramas por título puntúan título + alias: el hilo puede
 * nombrar cualquiera de los nombres del inmueble, pero el badge muestra
 * siempre el título canónico.
 * [09AA-28] Devuelve (id, título): el id resuelve la portada del panel.
 * [09AA-29] La rama por título compara contra TODOS los candidatos: aquí el
 * texto tras `|` es un título (no un ID numérico), así que el hilo ya lo
 * nombra y la ficha vinculada a su propio aviso sí debe verse.
 * [13AA-4] La rama de dígitos sin dueño tampoco cae al título, igual que el
 * borrador: el badge muestra «Sin ficha» y el borrador va sin precio. */
pub(in crate::services::marketplace) fn titulo_vinculado_del_hilo(
    thread_id: &str,
    candidatos: &[(uuid::Uuid, String, Vec<String>)],
    vinculos: &VinculosAviso,
) -> Option<(uuid::Uuid, String)> {
    let aviso = aviso_fb_de_thread(thread_id).unwrap_or_default();
    let recortado = aviso.trim();
    if recortado.is_empty() {
        return None;
    }
    if recortado.chars().all(|c| c.is_ascii_digit()) && (5..=32).contains(&recortado.len()) {
        return vinculos
            .get(recortado)
            .map(|(id, titulo, _)| (*id, titulo.clone()));
    }
    let mut mejor: Option<(usize, usize)> = None;
    let mut vinculo_mejor: Option<(uuid::Uuid, String)> = None;
    let mut empate = false;
    for (id, titulo, alias) in candidatos {
        let (directo, solape, distintivo) = mejor_puntaje_con_alias(recortado, titulo, alias);
        if !(directo || (solape >= 3 && distintivo >= 1)) {
            continue;
        }
        let clave = (distintivo, solape);
        match mejor {
            Some(m) if m == clave => empate = true,
            Some(m) if m > clave => {}
            _ => {
                mejor = Some(clave);
                vinculo_mejor = Some((*id, titulo.clone()));
                empate = false;
            }
        }
    }
    if mejor.is_some() && !empate {
        vinculo_mejor
    } else {
        None
    }
}

/// Filas de un chat (tope 200, recientes primero).
/// [08AA-18] Busca con `clave_hilo()` (ver `hilo_previo`).
pub async fn detalle_chat(pool: &sqlx::PgPool, thread: &str) -> Result<Vec<ChatFila>, AppError> {
    type FilaBd = (
        String,
        String,
        i64,
        bool,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    );
    let filas: Vec<FilaBd> = sqlx::query!(
        "SELECT excerpt_texto, respuesta, usos::BIGINT AS \"usos!\", corregida, valida_hasta, \
         origen, tokens_entrada, tokens_salida, ms_generacion \
         FROM mp_respuestas_cache WHERE thread_id = $1 \
         ORDER BY valida_hasta DESC LIMIT 200",
        clave_hilo(thread),
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        (
            r.excerpt_texto,
            r.respuesta,
            r.usos,
            r.corregida,
            r.valida_hasta,
            r.origen,
            r.tokens_entrada,
            r.tokens_salida,
            r.ms_generacion,
        )
    })
    .collect();
    Ok(filas
        .into_iter()
        .map(
            |(excerpt_texto, respuesta, usos, corregida, valida_hasta, origen, te, ts, ms)| {
                ChatFila {
                    excerpt_texto,
                    respuesta,
                    usos,
                    corregida,
                    valida_hasta: valida_hasta.to_rfc3339(),
                    origen,
                    coste: Coste {
                        tokens_entrada: te,
                        tokens_salida: ts,
                        ms,
                    },
                }
            },
        )
        .collect())
}
