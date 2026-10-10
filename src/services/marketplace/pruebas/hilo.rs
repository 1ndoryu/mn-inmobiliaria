#![cfg(test)]
//! Normalización del hilo, clave y aviso de Facebook.

use super::*;

#[test]
fn nombre_de_thread_saluda_por_nombre() {
    assert_eq!(
        nombre_de_thread("alejandro|casa en venta en riberas"),
        Some("Alejandro".to_string())
    );
    assert_eq!(
        nombre_de_thread("jean carlos|apto amoblado"),
        Some("Jean Carlos".to_string())
    );
    assert_eq!(nombre_de_thread("sin-hilo"), None);
    assert_eq!(nombre_de_thread("solo-sin-barra"), None);
    assert_eq!(nombre_de_thread("|aviso sin nombre"), None);
}

#[test]
fn normalizar_excerpt_quita_ruido_y_duplicados_conservando_roles() {
    /* Literales del HTML real de ella (hilo Riberas del Caroní,
     * `Agente/documentacion/usuario/conversacion-html-facebook.md`):
     * cada mensaje sale dos veces (visible + aria-label) y FB inyecta
     * inicio de chat, tip de seguridad y chrome. */
    let crudo = "Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
        Jorge inició este chat.\n\
        Cliente: Hola. ¿Sigue estando disponible?\n\
        Cliente: Hola. ¿Sigue estando disponible?\n\
        Cliente: Si te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas.\n\
        Dueña: Sí, sigue disponible en $43.000 negociable.\n\
        View buyer\n\
        Sí. ¿Te interesa?\n\
        Cliente: Precio..??";
    assert_eq!(
        normalizar_excerpt(crudo),
        "Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
        Cliente: Hola. ¿Sigue estando disponible?\n\
        Dueña: Sí, sigue disponible en $43.000 negociable.\n\
        Cliente: Precio..??"
    );
}

#[test]
fn normalizar_excerpt_conserva_rapida_si_la_escribe_el_cliente() {
    let crudo = "Cliente: Sí. ¿Te interesa?\nDueña: Sí, dime qué buscas.";
    assert_eq!(normalizar_excerpt(crudo), crudo);
}

#[test]
fn normalizar_excerpt_vacio_si_todo_es_ruido() {
    assert!(normalizar_excerpt("View buyer\nMore options\nAa").is_empty());
    assert!(normalizar_excerpt("   \n  ").is_empty());
}

#[test]
fn normalizar_hilo_kerley_deja_solo_la_pregunta() {
    /* [08AA-16] Testigo exacto en BD
     * (`kerley|VEF0 apartamento residencias rio aro plaza puerto
     * ordaz`): el visor repite cabeceras (eco del título recortado,
     * `Mensajes`, `Kerley · Apartamento ...`, `Kerley` suelto) e
     * inyecta la instrucción + las dos sugeridas ES. Solo la
     * pregunta del cliente sobrevive. */
    let hilo = "kerley|VEF0 apartamento residencias rio aro plaza puerto ordaz";
    let crudo = "amento Residencias Rio Aro Plaza Puerto Ordaz\n\
        Mensajes\n\
        Kerley · Apartamento Residencias Rio Aro Plaza Puerto Ordaz\n\
        Kerley\n\
        ¿Sigue disponible?\n\
        Toca una respuesta para enviársela al comprador.\n\
        Lo estoy mirando. Te avisaré.\n\
        Lo siento, no está disponible.";
    assert_eq!(normalizar_excerpt_hilo(hilo, crudo), "¿Sigue disponible?");
}

#[test]
fn normalizar_hilo_edickson_pela_chrome_nuevo() {
    /* [09AA-16] Testigo exacto en BD (`edickson|VEF0 casa en venta en
     * riberas del caroní, puerto ordaz`, `length(excerpt_texto)=137`):
     * chrome nuevo del visor — eco del título con cabeza cortada
     * (`n · Casa...`), `Se unió a Facebook en 2010`, cola huérfana
     * `del comprador` y etiqueta `Comprador`. Solo la pregunta del
     * cliente sobrevive. */
    let hilo = "edickson|VEF0 casa en venta en riberas del caroní, puerto ordaz";
    let crudo = "n · Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
        Se unió a Facebook en 2010\n\
        del comprador\n\
        Comprador\n\
        Hola. ¿Sigue estando disponible?";
    assert_eq!(
        normalizar_excerpt_hilo(hilo, crudo),
        "Hola. ¿Sigue estando disponible?"
    );
}

#[test]
fn normalizar_hilo_edgarluis_pela_eco_propio_y_despega_url() {
    /* [09AA-17] Testigo exacto en BD (`edgarluis|VEF0 casa en venta en
     * urbanización villa icabarú, puerto ordaz`,
     * `length(excerpt_texto)=353`): la burbuja de las 12:59am trae
     * `por Edgarluis:` + tip + NUESTRO borrador sin marca (el pelado
     * de la atribución lo dejaba como Cliente: CTA + teléfono + wa
     * con `wa.me` pegado por el aria `...855wa.mewa.me`), y el eco
     * `por Tú:` repite el borrador completo. Sobreviven el fragmento
     * de corte (`nión.`, irrecuperable) y el saludo propio etiquetado;
     * los cierres caen en ambas copias (son boilerplate que
     * `imponer_forma` re-agrega). */
    let hilo = "edgarluis|VEF0 casa en venta en urbanización villa icabarú, puerto ordaz";
    let crudo = "nión. Ver más consejos de seguridadPresionar Enter, Mensaje enviado 12:59 am por Edgarluis: Si te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas. Usa la función de compartir la ubicación en tiempo real directamente con un amigo o familiar durante la reunión.Hola, Edgarluis, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.\n\nCualquier cosa escríbeme al 04249208855.\n\nhttps://wa.me/584249208855wa.mewa.meEnviado hace 3 hPresionar Enter, Mensaje enviado 1:10 am por Tú: Hola, Edgarluis, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.\n\nCualquier cosa escríbeme al 04249208855.\n\nhttps://wa.me/584249208855Meta podría usar tecnología para revisar los mensajes de Marketplace con el fin de detectar y reducir las estafas y el fraude.Presionar Enter, Mensaje enviado 1:10 am por Tú: Meta podría usar tecnología para revisar los mensajes de Marketplace con el fin de detectar y reducir las estafas y el fraude.Escribir mensajeEscribe en Edgarluis · Casa en venta en Urbanización Villa Icabarú, Puerto Ordaz.Aa";
    assert_eq!(
        normalizar_excerpt_hilo(hilo, crudo),
        "nión.\nTú: Hola, Edgarluis, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable."
    );
}

#[test]
fn normalizar_hilo_tina_limpia_cola_y_marcas() {
    /* [08AA-16] Testigo exacto en BD (`tina|VEF0 casa en venta en
     * riberas del caroní, puerto ordaz`): el float cortó a mitad de
     * palabra (`ponible?`), el visor mete hora (`2:43 am`) y marca
     * de enviado. El resto (aunque sea mensaje propio sin marca)
     * se conserva como contexto. */
    let hilo = "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz";
    let crudo = "ponible?\n\
        2:43 am\n\
        Hola, disponible.\n\
        $43.000 negociable\n\
        04249208855\n\
        Enviado";
    assert_eq!(
        normalizar_excerpt_hilo(hilo, crudo),
        "Hola, disponible.\n$43.000 negociable\n04249208855"
    );
}

#[test]
fn normalizar_hilo_conserva_mensaje_corto_con_mayuscula() {
    /* La cola truncada no se come saludos completos: empiezan en
     * mayúscula aunque vayan en primera línea y sin espacios. */
    let hilo = "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz";
    assert_eq!(normalizar_excerpt_hilo(hilo, "Hola"), "Hola");
    assert_eq!(normalizar_excerpt_hilo(hilo, "Sí"), "Sí");
}

#[test]
fn normalizar_hilo_cola_de_cabecera_cortada_del_titulo() {
    /* [09AA-29] Testigo exacto del retest (`الله معي|VEF0 casa en venta
     * en urbanización villa icabarú, puerto ordaz.`): el float corta el
     * texto plano por carácter y sobrevive la cola de la cabecera
     * (`erto Ordaz.`) antes de la pregunta del cliente. Solo la pregunta
     * sobrevive: un único mensaje Cliente. */
    let hilo = "الله معي|VEF0 casa en venta en urbanización villa icabarú, puerto ordaz.";
    let crudo = "erto Ordaz.\n¿Sigue disponible?";
    assert_eq!(normalizar_excerpt_hilo(hilo, crudo), "¿Sigue disponible?");
}

#[test]
fn normalizar_hilo_tina_cargando_devuelve_vacio() {
    /* [08AA-17] Reporte de ella 2026-10-08 (panel, hilo Tina): el
     * hilo aún cargaba (`Cargando...`) y el visor repetía cabeceras
     * (`Tina · Casa ...`, `Marketplace`, `VEF0 - Casa ...` con guion,
     * `Escribe en Tina · ...`) sin ningún mensaje real: es
     * ruido → vacío (el handler conserva el original en ese caso). */
    let hilo = "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz";
    let crudo = "Tina · Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
        Marketplace\n\
        VEF0 - Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
        View buyer\n\
        More options\n\
        Mensajes\n\
        Cargando...\n\
        Escribir mensaje\n\
        Escribe en Tina · Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
        Aa";
    assert!(normalizar_excerpt_hilo(hilo, crudo).is_empty());
}

#[test]
fn normalizar_hilo_wilmery_pegado_deja_solo_la_pregunta() {
    /* [08AA-8] Testigo exacto en BD
     * (`wilmery|apartamento amoblado 3 hab. en vista hermosa, puerto
     * ordaz.`, `length(excerpt_texto)=554`): el puente aplana el DOM a
     * una sola línea pegada (`OrdazDetalles`, `WilmeryHola.`,
     * `disponible?Presionar`, `mensajeEscribe`) y el filtro por líneas
     * no tocaba nada. Solo la pregunta del cliente sobrevive (la
     * duplicada se colapsa). */
    let hilo = "wilmery|apartamento amoblado 3 hab. en vista hermosa, puerto ordaz.";
    let crudo = "También es miembro de CASAS y APARTAMENTOS en Puerto OrdazDetalles del compradorPresionar Enter, Mensaje enviado: 3:18 pm por: WilmeryHola. ¿Sigue estando disponible?Presionar Enter, Mensaje enviado 3:18 pm por Wilmery: Hola. ¿Sigue estando disponible?Envía una respuesta rápidaToca una respuesta para enviársela al comprador.Sí. ¿Te interesa?Lo estoy mirando. Te avisaré.Lo siento, no está disponible.Presionar Enter, Mensaje enviado: 3:18 pm por: WilmeryEscribir mensajeEscribe en Wilmery · Apartamento amoblado 3 hab. en Vista Hermosa, Puerto Ordaz.Aa";
    assert_eq!(
        normalizar_excerpt_hilo(hilo, crudo),
        "Hola. ¿Sigue estando disponible?"
    );
}

#[test]
fn normalizar_hilo_cristo_dia_y_truncado_deja_solo_preguntas() {
    /* [08AA-24] Crudo exacto del hilo cristo (`excerpt_crudo` 1200,
     * con saltos): día de semana ante la hora (`lunes 22:48 por
     * Cristo:`), duplicado con dos puntos (`lunes 22:48 por:
     * Cristo`), inicio truncado por el `slice(-1200)` del float
     * (`sionar Enter,`) y marca `3:53 pm` separando mensajes. Solo
     * las 2 preguntas sobreviven. */
    let hilo = "cristo|VEF0 alquiler townhouse 2 niveles en arivana, puerto ordaz.";
    let crudo = "sionar Enter, Mensaje enviado lunes 22:48 por Cristo: Hola. ¿Sigue estando disponible?\nEnvía una respuesta rápida\nToca una respuesta para enviársela al comprador.\nSí. ¿Te interesa?\nLo estoy mirando. Te avisaré.\nLo siento, no está disponible.\nPresionar Enter, Mensaje enviado: lunes 22:48 por: Cristo\nSi te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas. Usa la función de compartir la ubicación en tiempo real directamente con un amigo o familiar durante la reunión. Ver más consejos de seguridad\nPresionar Enter, Mensaje enviado lunes 22:48 por Cristo: Si te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas. Usa la función de compartir la ubicación en tiempo real directamente con un amigo o familiar durante la reunión.\n3:53 pm\nCristo\n¿Sigue disponible?\nPresionar Enter, Mensaje enviado 3:53 pm por Cristo: ¿Sigue disponible?\nEnvía una respuesta rápida\nToca una respuesta para enviársela al comprador.\nSí. ¿Te interesa?\nLo estoy mirando. Te avisaré.\nLo siento, no está disponible.\nPresionar Enter, Mensaje enviado: 3:53 pm por: Cristo\nEscribir mensaje\nEscribe en Cristo · Alquiler Townhouse 2 niveles en Arivana, Puerto Ordaz.\n\n\n\n\nAa";
    assert_eq!(
        normalizar_excerpt_hilo(hilo, crudo),
        "Hola. ¿Sigue estando disponible?\n¿Sigue disponible?"
    );
}

#[test]
fn normalizar_hilo_yusmelis_etiqueta_lado_propio_y_pela_chrome() {
    /* [08AA-29] Crudo exacto del hilo yusmelis (`excerpt_crudo` 1200
     * en BD): el aviso de seguridad de Meta llega como texto suelto
     * (`fin de detectar...`) y como eco propio, la burbuja propia se
     * duplica en su eco (`por Tú:`), y el chrome trae `wa.me`,
     * `En medio de la conversación` y `Enviado hace 1 min`. Lo de
     * ella se etiqueta (`Tú:`) para separarlo del cliente, el eco
     * repetido no duplica y el ruido no sobrevive. */
    let hilo = "yusmelis|VEF0 casa en venta en urbanización villa icabarú, puerto ordaz";
    let crudo = "fin de detectar y reducir las estafas y el fraude.\n\
        Presionar Enter, Mensaje enviado 5:28 pm por Tú: Meta podría usar tecnología para revisar los mensajes y así garantizar la seguridad de todas las personas.\n\
        Yusmelis\n\
        Buenas tardes Mayerlin, gracias por la información y no ofrece algún plan de financiamiento para el pago de la casa?\n\
        Presionar Enter, Mensaje enviado 5:28 pm por Yusmelis: Buenas tardes Mayerlin, gracias por la información y no ofrece algún plan de financiamiento para el pago de la casa?\n\
        Estoy interesada en una casa en puerto Ordaz que esté en una zona céntrica sí tienes otras opciones que no superen los 60 mil $ me podrías informar por favor\n\
        Mensajes\n\
        Hola, por favor,\n\
        dejame un numero\n\
        para guardarte y pasarte\n\
        la información.\n\
        Presionar Enter, Mensaje enviado 5:51 pm por Tú: Hola, por favor, dejame un numero para guardarte y pasarte la información.\n\
        Escribir mensaje\n\
        Escribe en Yusmelis · Casa en venta en Urbanización Villa Icabarú, Puerto Ordaz.\n\
        wa.me\n\
        En medio de la conversación\n\
        Enviado hace 1 min\n\
        Aa\n\
        Presionar Enter, Mensaje enviado 5:51 pm por Tú: Hola, por favor, dejame un numero para guardarte y pasarte la información.";
    assert_eq!(
        normalizar_excerpt_hilo(hilo, crudo),
        "Buenas tardes Mayerlin, gracias por la información y no ofrece algún plan de financiamiento para el pago de la casa?\n\
        Estoy interesada en una casa en puerto Ordaz que esté en una zona céntrica sí tienes otras opciones que no superen los 60 mil $ me podrías informar por favor\n\
        Tú: Hola, por favor, dejame un numero para guardarte y pasarte la información."
    );
}

#[test]
fn clave_hilo_deshace_precio_inyectado_y_respeta_lo_demas() {
    /* [08AA-18] El puente (07AA-11) manda `tina|$43.000 vef0...` pero
     * la cifra parpadea entre llamadas: la BD solo ve la forma
     * canónica para que guardar y buscar emparejen siempre. */
    assert_eq!(
        clave_hilo("tina|$43.000 vef0 casa en venta"),
        "tina|vef0 casa en venta"
    );
    assert_eq!(
        clave_hilo("tina|US$ 43.000 vef0 casa en venta"),
        "tina|vef0 casa en venta"
    );
    assert_eq!(clave_hilo("  tina|$43.000 vef0 casa  "), "tina|vef0 casa");
    assert_eq!(
        clave_hilo("tina|vef0 casa en venta"),
        "tina|vef0 casa en venta"
    );
    assert_eq!(clave_hilo("sin-hilo"), "sin-hilo");
    assert_eq!(clave_hilo(""), "");
    /* Sin dígitos no es cifra; sin resto no hay aviso; `$` en otro
     * sitio no es inyección: queda intacto. */
    assert_eq!(clave_hilo("ana|$negociable casa"), "ana|$negociable casa");
    assert_eq!(clave_hilo("ana|$50"), "ana|$50");
    assert_eq!(clave_hilo("ana|casa $50 mil"), "ana|casa $50 mil");
}

#[test]
fn precio_del_aviso_extrae_moneda_antes_o_despues() {
    assert_eq!(
        precio_del_aviso("town house en venta en las peonías 125.000$"),
        Some("125.000$".to_string())
    );
    assert_eq!(
        precio_del_aviso("Casa $95.000 en Riberas"),
        Some("$ 95.000".to_string())
    );
    assert_eq!(
        precio_del_aviso("APTO USD 120.000"),
        Some("usd 120.000".to_string())
    );
    assert_eq!(precio_del_aviso("casa en venta, 3 habitaciones"), None);
    assert_eq!(precio_del_aviso("piso 2, año 2024"), None);
}

#[test]
fn asegurar_contacto_agrega_lo_que_falta_y_respeta_lo_presente() {
    let sin_nada = asegurar_contacto("Casa en Riberas.\nSí, aceptamos visita.");
    assert!(sin_nada.contains(CONTACTO_TEL));
    assert!(sin_nada.ends_with(CONTACTO_WA));
    let completo = asegurar_contacto(&format!(
        "Casa.\nCualquier cosa escríbeme al {CONTACTO_TEL}\n{CONTACTO_WA}"
    ));
    assert_eq!(completo.matches(CONTACTO_TEL).count(), 1);
    assert_eq!(completo.matches(CONTACTO_WA).count(), 1);
}

#[test]
fn aviso_fb_sale_del_hilo() {
    assert_eq!(
        aviso_fb_de_thread("javier|casa en venta en riberas del caroní, p..."),
        Some("casa en venta en riberas del caroní, p...".to_string())
    );
    assert_eq!(aviso_fb_de_thread("sin-hilo"), None);
    assert_eq!(aviso_fb_de_thread("solo|"), None);
}
