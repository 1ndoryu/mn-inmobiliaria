/// Versión del strip aceptada (`strip_vN` del plan).
/// [08AA-25] v2 suma `operacion` al allowlist: sin ella la IA presentaba
/// los alquileres como ventas (testigo: Townhouse Arivana, hilo cristo).
/// [18AA-5] v3 suma `nombre_corto`: el sujeto de la frase lo monta Rust en
/// vez de la IA, que lo componía con el título entero.
pub const STRIP_VERSION: &str = "v3";
/// Fallback exacto cuando no hay ficha o falla la IA.
/// [09AA-2] Sin «confirmo»: el prompt prohíbe anunciar confirmaciones y el
/// texto anterior («te confirmo precio/entrega») minaba ese veto.
pub const FALLBACK_BORRADOR: &str = "Lo reviso y te escribo el precio por aquí";
/// [07AA-8] Contacto fijo de los borradores (decisión de ella 2026-10-07):
/// la IA no lo inventa, el prompt lo exige literal y `asegurar_contacto`
/// lo agrega si falta. [08AA-14] Sin matriz negativa por decisión de ella
/// 2026-10-08: el texto (propio o de la IA) pasa tal cual.
pub const CONTACTO_TEL: &str = "04249208855";
pub const CONTACTO_WA: &str = "https://wa.me/584249208855";

/// [21AA-2] Cierre del borrador, decidido por ella el 2026-10-21: el teléfono
/// y el enlace van pegados en el MISMO párrafo. Sin «Somos inmobiliaria»
/// (lo quitó ese mismo día: sobra en un chat): arranca en «Puede llamarnos».
pub const CIERRE_BORRADOR: &str = "Puede llamarnos al 04249208855 para cualquier consulta, o escribirnos al whatsapp: https://wa.me/584249208855";

/// [09AA-2] Invitación canónica del final; [21AA-2] ya NO se emite (ella la
/// quitó el 2026-10-21: el mensaje va apertura + cierre). Se conserva para
/// reconocer el eco propio y las filas de caché viejas que aún la traen.
pub const CTA_FIJO: &str = "Cuéntame qué estás buscando y con gusto te ayudo.";

/* [09AA-2] La invariante de forma la impone Rust, no el wording: el prompt
 * mismo ORDENABA el relleno (±08AA-36/37/38) y esa orden positiva siempre le
 * ganó al veto. Cualquier párrafo intermedio con pregunta, oferta de fotos o
 * reafirmación se poda (lo hace `imponer_forma_borrador`). No toca texto
 * manual de la dueña ni la rama `reserva`. El `CTA_FIJO` de arriba queda solo
 * como reconocimiento (eco propio y filas de caché viejas). */
