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
/// Enlace de WhatsApp con el número en formato internacional (sin el `0`
/// inicial). [GW-19] Ella lo devolvió al cierre el 2026-11-28 y pidió
/// verificar que funcionara: responde 200 y abre `whatsapp.com/send` con el
/// número cargado.
pub const CONTACTO_WA: &str = "https://wa.me/584249208855";

/// [21AA-2] Cierre del borrador de un inmueble disponible. [GW-19] Decisión
/// de ella 2026-11-28: vuelve el enlace de WhatsApp y sale la invitación
/// final. Teléfono a la vista y el enlace, nada más.
pub const CIERRE_BORRADOR: &str = "Puede llamarnos al 04249208855 para cualquier consulta, o escribirnos al whatsapp: https://wa.me/584249208855";

/// [GW-16] Cierre que se emitió entre el 2026-11-21 y el 2026-11-28, con la
/// invitación pegada al teléfono. Ya no se escribe: sirve para reconocer el
/// eco propio en los hilos y las filas de caché que aún lo traen.
pub const CIERRE_BORRADOR_CON_INVITACION: &str = "Puede llamarnos al 04249208855 para cualquier consulta, o escribirnos al whatsapp. Cuéntenos qué está buscando y con gusto le ayudamos.";

/// [GW-15] Cierre recortado que se emitió entre el 2026-11-21 y el 2026-11-28
/// (sin enlace ni invitación). Ya no se escribe; se reconoce.
pub const CIERRE_BORRADOR_SIN_CTA: &str = "Puede llamarnos al 04249208855 para cualquier consulta, o escribirnos al whatsapp.";

/// [GW-19] Cierre propio del aviso «no disponible» (decisión de ella
/// 2026-11-28): no repite el teléfono del cierre de los disponibles, ofrece
/// mandar la ficha y pregunta el monto. En usted, como el resto del
/// borrador; sin enlace, porque el comprador ya está en el hilo.
pub const CIERRE_AVISO: &str = "Si le interesa se la envío por aquí o a su whatsapp, o me comenta hasta qué monto está buscando.";

/// [GW-15] Arranque común de los dos cierres (`CIERRE_BORRADOR` y el que
/// llevaba enlace). Es la marca que separa el cierre en su propio párrafo
/// cuando la IA lo pega a la frase anterior, sin depender de que el texto
/// termine igual que el cierre vigente.
pub const ARRANQUE_CIERRE: &str = "Puede llamarnos al";

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
