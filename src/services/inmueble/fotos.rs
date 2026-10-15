//! Fotos y archivos del inmueble: alta, borrado en cascada por `orden`,
//! subida a disco, miniaturas y validación de magic-bytes. Vive aparte de
//! `inmueble.rs` para respetar el tope del gate (Sentinel `god-object-rs`).

use std::path::{Path, PathBuf};

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AddFotoRequest, FotoPublica, EXTENSIONES_FOTO, MAX_FOTO_BYTES, ORIGENES_FOTO};
use crate::repositories::InmuebleRepository;
use crate::services::sync::huella::registrar_huella_subida;
use crate::services::sync::vigilancia::notificar;

use super::InmuebleService;

impl InmuebleService {
    pub async fn add_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        req: AddFotoRequest,
    ) -> Result<FotoPublica, AppError> {
        if InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        let origen = req
            .origen
            .as_deref()
            .map(|v| Self::normalizar(v, ORIGENES_FOTO, "origen"))
            .transpose()?
            .unwrap_or_else(|| "original".to_string());
        let foto =
            InmuebleRepository::add_foto(pool, inmueble_id, &req.storage_key, req.orden, &origen)
                .await?;
        notificar();
        Ok(FotoPublica::from(foto))
    }

    /// [15AA-8] Reordena las fotos del inmueble sin borrar ni subir nada.
    /// `ids` son las `original` del inmueble en el orden deseado; cada
    /// mejorada sigue a su hermana por `orden`. Es la operación correcta
    /// para cambiar la principal: reordenar nunca destruye fotos.
    pub async fn reordenar_fotos(
        pool: &PgPool,
        inmueble_id: Uuid,
        ids: &[Uuid],
    ) -> Result<(), AppError> {
        if InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        if !InmuebleRepository::reordenar_fotos(pool, inmueble_id, ids).await? {
            return Err(AppError::Validation(
                "La lista de fotos no coincide con las del inmueble".into(),
            ));
        }
        notificar();
        Ok(())
    }

    pub async fn delete_foto(
        pool: &PgPool,
        upload_dir: &Path,
        foto_id: Uuid,
    ) -> Result<(), AppError> {
        /* [08AA-4] La cascada a la hermana + renumerado + `tocar_inmueble`
         * ocurren en una transacción; aquí solo limpieza de disco best-effort. */
        let Some(claves) = InmuebleRepository::delete_foto_en_cascada(pool, foto_id).await? else {
            return Err(AppError::NotFound("Foto no encontrada".into()));
        };
        notificar();
        for clave in &claves {
            Self::borrar_archivo(upload_dir, clave).await;
            /* El thumb muere con su original (si no existe, no pasa nada):
             * formato actual más legado `thumb-` de 320 px ([249A-4]). */
            if let Some(clave_thumb) = Self::clave_miniatura(clave) {
                Self::borrar_archivo(upload_dir, &clave_thumb).await;
            }
            if let Some(clave_legada) = Self::clave_miniatura_legada(clave) {
                Self::borrar_archivo(upload_dir, &clave_legada).await;
            }
        }
        Ok(())
    }

    /* [249A-1] Miniatura de tabla: JPEG del lado mayor, calidad 70. Puro
     * Rust (crate `image`, sin libs del sistema). Devuelve `None` si los
     * bytes no decodifican: la subida principal no debe caer por el thumb.
     * [249A-4] 320→160 px: la tabla pública muestra 64-80 px (DPR 2 cubierto
     * con 160) y el admin usa originales; PageSpeed pedía ~113 KiB menos en
     * las 8 portadas visibles. La clave cambia a `min160-` para regenerar. */
    pub(crate) fn miniatura(bytes: &[u8]) -> Option<Vec<u8>> {
        let img = image::load_from_memory(bytes).ok()?;
        let reducida = img.thumbnail(160, 160);
        let mut salida = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut salida, 70)
            .encode_image(&reducida)
            .ok()?;
        Some(salida)
    }

    /* Clave del thumb junto al original: `<inmueble>/min160-<uuid>.jpg`. */
    fn clave_miniatura(storage_key: &str) -> Option<String> {
        let (carpeta, archivo) = storage_key.split_once('/')?;
        let punto = archivo.rfind('.')?;
        if archivo.starts_with("min160-") || archivo.starts_with("thumb-") {
            return None;
        }
        Some(format!("{carpeta}/min160-{}.jpg", &archivo[..punto]))
    }

    /* Legado `thumb-<uuid>.jpg` de 320 px ([249A-1]): solo para borrar al
     * regenerar o eliminar la foto; ya no se genera ni se pide. */
    fn clave_miniatura_legada(storage_key: &str) -> Option<String> {
        let (carpeta, archivo) = storage_key.split_once('/')?;
        let punto = archivo.rfind('.')?;
        if archivo.starts_with("min160-") || archivo.starts_with("thumb-") {
            return None;
        }
        Some(format!("{carpeta}/thumb-{}.jpg", &archivo[..punto]))
    }

    /// Guarda bytes subidos en `UPLOAD_DIR/<inmueble>/<uuid>.<ext>` y registra la foto.
    /// Valida extensión + magic-bytes (jpeg/png/webp); el tope lo impone el extractor.
    pub async fn subir_foto(
        pool: &PgPool,
        upload_dir: &Path,
        inmueble_id: Uuid,
        filename: &str,
        origen: Option<&str>,
        orden: Option<i32>,
        bytes: &[u8],
    ) -> Result<FotoPublica, AppError> {
        if InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        let origen = origen
            .map(|v| Self::normalizar(v, ORIGENES_FOTO, "origen"))
            .transpose()?
            .unwrap_or_else(|| "original".to_string());

        let storage_key =
            Self::guardar_archivo(upload_dir, &inmueble_id.to_string(), filename, bytes).await?;

        match InmuebleRepository::add_foto(pool, inmueble_id, &storage_key, orden, &origen).await {
            Ok(foto) => {
                /* [249A-1] Invalida la caché de fotos (`?v=<updated_at>`). */
                InmuebleRepository::tocar_inmueble(pool, inmueble_id).await?;
                /* [13AA-12] Huella al subir (los bytes ya están en memoria). */
                registrar_huella_subida(pool, &storage_key, bytes).await;
                notificar();
                /* Thumb best-effort: si falla, la tabla usa el original. */
                if let Some(clave_thumb) = Self::clave_miniatura(&storage_key) {
                    if let Some(mini) = Self::miniatura(bytes) {
                        if let Err(e) = tokio::fs::write(upload_dir.join(&clave_thumb), mini).await
                        {
                            tracing::warn!("No se pudo guardar {clave_thumb}: {e}");
                        }
                    } else {
                        tracing::warn!("No se pudo generar miniatura de {storage_key}");
                    }
                }
                Ok(FotoPublica::from(foto))
            }
            Err(e) => {
                /* Sin fila no hay foto: se retira el archivo huérfano */
                Self::borrar_archivo(upload_dir, &storage_key).await;
                Err(e.into())
            }
        }
    }

    /* [169A-2] Núcleo reutilizable: valida y guarda bytes en
     * `UPLOAD_DIR/<carpeta>/<uuid>.<ext>`, devuelve la clave. Lo usan las
     * fotos de inmueble y las de solicitud (carpeta `solicitudes/<uuid>`). */
    pub async fn guardar_archivo(
        upload_dir: &Path,
        carpeta: &str,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, AppError> {
        if bytes.is_empty() {
            return Err(AppError::BadRequest("Archivo vacío".into()));
        }
        if bytes.len() > MAX_FOTO_BYTES {
            return Err(AppError::PayloadMuyGrande);
        }
        let extension = Self::extension_valida(filename)?;
        Self::magia_valida(bytes, extension)?;

        let dir = upload_dir.join(carpeta);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(AppError::from)?;
        let nombre = format!("{}.{}", Uuid::new_v4(), &extension[1..]);
        tokio::fs::write(dir.join(&nombre), bytes)
            .await
            .map_err(AppError::from)?;
        Ok(format!("{carpeta}/{nombre}"))
    }

    /* [299A-1 E12] Las notas de voz se archivan igual que las fotos pero
     * con su propia validación: `guardar_archivo` solo acepta imágenes y
     * cualquier audio caía al fallback `[media]` (la IA pedía el texto sin que el
     * staff pudiera oír nada). Misma carpeta y mismo tope de tamaño. */
    pub async fn guardar_audio(
        upload_dir: &Path,
        carpeta: &str,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, AppError> {
        if bytes.is_empty() {
            return Err(AppError::BadRequest("Archivo vacío".into()));
        }
        if bytes.len() > MAX_FOTO_BYTES {
            return Err(AppError::PayloadMuyGrande);
        }
        let extension = Self::extension_audio_valida(filename)?;
        Self::magia_audio_valida(bytes, extension)?;

        let dir = upload_dir.join(carpeta);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(AppError::from)?;
        let nombre = format!("{}.{}", Uuid::new_v4(), &extension[1..]);
        tokio::fs::write(dir.join(&nombre), bytes)
            .await
            .map_err(AppError::from)?;
        Ok(format!("{carpeta}/{nombre}"))
    }

    pub(super) fn extension_audio_valida(filename: &str) -> Result<&'static str, AppError> {
        let minusculas = filename.to_lowercase();
        let punto = minusculas.rfind('.').ok_or_else(|| {
            AppError::BadRequest("El audio necesita extensión (.ogg, .mp3, .m4a)".into())
        })?;
        let extension = &minusculas[punto..];
        [".ogg", ".mp3", ".m4a"]
            .iter()
            .find(|e| ***e == *extension)
            .copied()
            .ok_or_else(|| {
                AppError::BadRequest("Extensión no permitida (solo .ogg, .mp3, .m4a)".into())
            })
    }

    /* Firmas mínimas: `OggS` (ogg), `ID3` o sync de trama `0xFF 0xE_` (mp3),
     * `....ftyp` (m4a/mp4). No es un parser completo: solo evita guardar
     * HTML de error o texto con extensión de audio. */
    pub(super) fn magia_audio_valida(bytes: &[u8], extension: &str) -> Result<(), AppError> {
        let es_ogg = bytes.starts_with(b"OggS");
        let es_mp3 = bytes.starts_with(b"ID3")
            || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0);
        let es_m4a = bytes.len() >= 8 && bytes[4..8] == *b"ftyp";
        let valido = match extension {
            ".ogg" => es_ogg,
            ".mp3" => es_mp3,
            ".m4a" => es_m4a,
            _ => false,
        };
        if valido {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "El contenido no coincide con un audio válido".into(),
            ))
        }
    }

    /// Borra un archivo del volumen; los fallos se registran pero no rompen la operación
    pub(super) async fn borrar_archivo(upload_dir: &Path, storage_key: &str) {
        let ruta = upload_dir.join(storage_key);
        if let Err(e) = tokio::fs::remove_file(&ruta).await {
            tracing::warn!("No se pudo borrar {ruta:?}: {e}");
        }
    }

    pub(super) fn extension_valida(filename: &str) -> Result<&'static str, AppError> {
        let minusculas = filename.to_lowercase();
        let punto = minusculas.rfind('.').ok_or_else(|| {
            AppError::BadRequest("El archivo necesita extensión (.jpg, .png, .webp)".into())
        })?;
        let extension = &minusculas[punto..];
        EXTENSIONES_FOTO
            .iter()
            .find(|e| **e == extension)
            .copied()
            .ok_or_else(|| {
                AppError::BadRequest("Extensión no permitida (solo .jpg, .png, .webp)".into())
            })
    }

    pub(super) fn magia_valida(bytes: &[u8], extension: &str) -> Result<(), AppError> {
        let es_jpeg = bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF;
        let es_png = bytes.len() >= 8
            && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let es_webp = bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP";
        let valido = match extension {
            ".jpg" | ".jpeg" => es_jpeg,
            ".png" => es_png,
            ".webp" => es_webp,
            _ => false,
        };
        if valido {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "El contenido no coincide con una imagen válida".into(),
            ))
        }
    }

    /// Ruta absoluta de una clave dentro del volumen (para servir archivos)
    #[must_use]
    pub fn ruta_archivo(upload_dir: &Path, storage_key: &str) -> PathBuf {
        upload_dir.join(storage_key)
    }
}
