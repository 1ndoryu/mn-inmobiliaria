/// Versión del strip aceptada (`strip_vN` del plan).
/// [08AA-25] v2 suma `operacion` al allowlist: sin ella la IA presentaba
/// los alquileres como ventas (testigo: Townhouse Arivana, hilo cristo).
pub const STRIP_VERSION: &str = "v2";
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

/// [09AA-2] Fin del bucle del párrafo de relleno (08AA-36/37/38 burlados con
/// sinónimos): el prompt mismo ORDENABA el relleno («avanza la conversación:
/// ofrece fotos o pregunta qué busca») y esa orden positiva siempre le ganó
/// al veto. Ahora la invariante la impone Rust, no el wording: el borrador
/// de IA sale como P1 + [una línea de dato útil] + CTA canónico + wa.
/// Cualquier párrafo intermedio con pregunta, oferta de fotos o reafirmación
/// se poda; el final se reconstruye literal (nunca se conserva el de la IA).
/// No toca texto manual de la dueña ni la rama `reserva`.
pub const CTA_FIJO: &str = "Cuéntame qué estás buscando y con gusto te ayudo.";
