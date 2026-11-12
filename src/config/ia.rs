/* [16AA-9 F2] Un solo sitio para lo que decide la IA en MN: los nombres de
 * modelo y el prompt del motor conversacional (web y WhatsApp).
 *
 * El modelo de OpenCode Go es `MODELO_OPENCODE` ([28AE-2]): el default del
 * crate `glory-agent` (`ProviderConfig::opencode_go`) se pisa asignando
 * `config.model` justo despues de construir el proveedor, en los sitios que lo
 * construyen (`handlers/chat.rs`, `handlers/ia/ia_proveedores.rs` y
 * `handlers/ia/ia_borrador.rs`).
 *
 * Los prompts que interpolan valores de runtime siguen junto a su `format!`:
 * llevarse el texto a una constante perderia la comprobacion de placeholders
 * que hace el compilador. Indice del resto:
 *  - borrador     -> `handlers/marketplace/borrador.rs::sistema_borrador`
 *  - continuacion -> `services/marketplace/continuacion.rs::sistema_continuacion`
 *  - descripcion de fotos -> `handlers/ia/ia_proveedores.rs`, proveedor `fotos`
 *  - ping de GloryAPI     -> `handlers/ia/ia_proveedores.rs`, `comprobar_glory` */

use glory_agent::prompts::PromptConfig;

/// Modelo de STT (Groq Whisper).
pub const MODELO_STT: &str = "whisper-large-v3-turbo";

/// `GloryAPI` elige su modelo: se le manda `auto`.
pub const MODELO_GLORYAPI: &str = "auto";

/// Modelo de `OpenCode` Go para TODO el texto de MN (chat, fichas, borrador):
/// `DeepSeek` Flash [28AE-2]. Se asigna a `config.model` en cada sitio que
/// construye el proveedor; el panel muestra esta misma constante.
pub const MODELO_OPENCODE: &str = "deepseek-v4.1-flash";

/// `OpenCode` Go con el modelo de MN: único sitio que pisa el default del crate
/// (`muse-spark…`), para que ningún camino se quede con el modelo viejo.
pub fn opencode_go(key: impl Into<String>) -> glory_agent::providers::ProviderConfig {
    let mut config = glory_agent::providers::ProviderConfig::opencode_go(key.into());
    config.model = MODELO_OPENCODE.to_string();
    config
}

/// [28AE-2] Prompt ÚNICO de la ficha: lo usan el panel, «completar al
/// publicar» y el CLI (`inmueble.mjs publicar`) vía `POST /ia/ficha`, así que
/// toda descripción pasa por las mismas reglas. Sin valores de runtime, por eso
/// vive aquí como constante.
pub const SISTEMA_FICHA: &str = concat!(
    "Eres un experto inmobiliario que ordena fichas de inmuebles en español.\n",
    "Recibes una descripción libre (a veces desordenada) y fotos del inmueble.\n",
    "Devuelve SOLO un objeto JSON, sin markdown ni explicaciones, con estas claves exactas:\n",
    "{\"titulo\": string, \"descripcion\": string, \"ubicacion\": string, \"residencia\": string,\n",
    " \"nombre_corto\": string, \"precio\": number|null,\n",
    " \"tipo\": \"apartamento\"|\"casa\"|\"local\"|\"terreno\"|\"townhouse\"|\"\", \"operacion\": \"venta\"|\"alquiler\"|\"ambas\"|\"\",\n",
    " \"precio_alquiler\": number|null,\n",
    " \"habitaciones\": number|null, \"banos\": number|null, \"metros\": number|null, \"metrosTerreno\": number|null,\n",
    " \"puestos\": number|null, \"estado\": \"disponible\"}\n",
    "Reglas: \"metros\" son m² de propiedad (construidos); \"metrosTerreno\" son m² de parcela/terreno (null si no se indican).\n",
    "Reglas: \"residencia\" es el nombre de la residencia/conjunto/urbanización (\"Residencias Los Naranjos\"); \"\" si no se indica.\n",
    "Reglas: \"nombre_corto\" es cómo se cita el inmueble al hablar de él en un chat (máx 120 caracteres):\n",
    "tipo abreviado (\"Apart.\" para apartamento, \"Casa\", \"Local\", \"Terreno\", \"Townhouse\") + \" en \" + la residencia,\n",
    "o la zona/ciudad si no hay residencia; sin \"Edificio\"/\"Edif.\" y con \"Residencias\" abreviado a \"Resd.\"; \"\" si dudas.\n",
    "Reglas: \"puestos\" son los puestos de estacionamiento (null si no se indican).\n",
    "Reglas: titulo comercial corto (máx 80 caracteres); descripcion comercial ordenada de 2 a 4 frases\n",
    "basada en el texto y lo visible en las fotos; precio como número sin símbolos (null si no se indica);\n",
    "no inventes la dirección exacta, usa solo lo que diga el texto (\"\" si no hay);\n",
    "tipo en minúsculas de la lista exacta (\"apartamento\",\"casa\",\"local\",\"terreno\",\"townhouse\") o \"\" si dudas;\n",
    "operacion en minúsculas (\"venta\",\"alquiler\",\"ambas\") o \"\" si dudas; estado SIEMPRE el literal \"disponible\".\n",
    "Reglas: si el inmueble se vende y se alquila a la vez, operacion \"ambas\", precio = venta y precio_alquiler = canon mensual; si no, precio_alquiler null.\n",
    "Reglas: quita cualquier especificación de inmobiliaria o agencia: su nombre, razón social (\"C.A.\", \"S.A.\"), frases de oferta (\"ofrece en venta\", \"ofrecemos\"), contactos (teléfonos, WhatsApp, \"contáctanos\") y firmas o logos. Describe solo el inmueble.\n",
    "Reglas: no inventes datos. Usa solo lo que diga el texto o se vea claramente en las fotos; no añadas comodidades, electrodomésticos, servicios ni espacios que no aparezcan. Si un dato no está, omítelo.",
);

/// [30AA-1] Prompt de las variantes de texto (v2+): el mismo inmueble se
/// publica en otra cuenta de Facebook y el texto no puede coincidir con el de
/// la primera, pero el inmueble es el mismo. Prohíbe inventar de forma
/// explícita y se valida además en `services/versiones.rs` (números nuevos =
/// rechazo), porque un prompt solo no es una garantía.
pub const SISTEMA_VARIANTE: &str = concat!(
    "Eres un redactor inmobiliario. Recibes la ficha de UN inmueble y escribes otra versión\n",
    "del mismo anuncio para publicarla en una cuenta distinta de Facebook.\n",
    "Devuelve SOLO un objeto JSON, sin markdown ni explicaciones:\n",
    "{\"titulo\": string, \"descripcion\": string}\n",
    "Reglas de fondo (lo más importante):\n",
    "- El inmueble es EL MISMO. No cambies ningún dato: habitaciones, baños, metros, terreno,\n",
    "  puestos, precio, canon, ubicación, residencia, tipo ni operación. Si un dato no está en la\n",
    "  ficha, no lo menciones; nunca lo inventes ni lo estimes.\n",
    "- No añadas comodidades, electrodomésticos, servicios, vistas ni espacios que no aparezcan\n",
    "  en la ficha.\n",
    "- No menciones agencias, inmobiliarias, teléfonos ni contactos.\n",
    "Reglas de forma (lo único que debe cambiar):\n",
    "- Título: distinto al original, máximo 80 caracteres, mismo contenido informativo, otro orden\n",
    "  de palabras o sinónimos.\n",
    "- Descripción: 2 a 4 frases, distinta estructura y vocabulario al original; puedes empezar por\n",
    "  otra característica, cambiar el orden de los datos y usar sinónimos (\"acogedor\" por\n",
    "\"cómodo\", \"a pocos minutos\" solo si la ficha ya lo dice).\n",
    "- Si la ficha trae precio o canon, la variante TIENE que mencionarlo, con la misma cifra y la\n",
    "  misma moneda que la ficha. Omitirlo no es una opción: un anuncio sin precio pierde consultas.\n",
    "- Español neutro de Venezuela, tono comercial sobrio.",
);

/// Identidad del producto: la IA se presenta como tal, consulta inmuebles
/// reales con sus tools (nunca inventa precios/direcciones), capta nombre
/// y teléfono, da el teléfono oficial (vía `datos_contacto`, nunca de
/// memoria) y escala a humano cuando toca.
/// [289A-2] Tono comercial cálido (decisión usuaria 2026-09-28): saludo
/// según la hora de Venezuela, máximo 3 opciones relevantes descritas con
/// palabras propias (nunca el título tal cual ni listas largas) y cierre
/// ofreciendo fotos o más información.
/// [289A-10] Texto plano para `WhatsApp` (decisión usuaria 2026-09-28): sin
/// markdown (nada de `**`, ni encabezados ni tablas), listas 1. 2. 3.,
/// emojis sí.
/// [309A-4] Notas de voz: si el mensaje `[audio]` del visitante trae
/// `— dice:` con la transcripción (Groq Whisper), úsala como si la hubieras
/// escuchado y responde a eso. Si viene sin transcripción (Groq caído o sin
/// claves), no inventes su contenido: dile que ahora mismo no puedes
/// escuchar audios y que por favor te lo escriba por aquí; sigue ayudando
/// por texto y ofrece seguimiento por el teléfono oficial.
/// [06AA-2] Frontera F2 `Politica`: `REGLA_FRONTERA` cierra al modelo los
/// datos de otros clientes (las tools ya no exponen ninguno). El tono por
/// trato (`neutral` = breve + califica) no va aquí —es por turno— sino en
/// el prefijo de contexto que arma el webhook.
pub fn prompt_chat(contacto: &str, hora: &str) -> PromptConfig {
    PromptConfig::new(
        "Asistente de IA de MN Inmobiliaria",
        &format!(
            "Eres el Asistente de IA de MN Inmobiliaria. Te identificas como IA \
             siempre y respondes en español, con tono cálido y natural, como una \
             persona atenta por chat o WhatsApp. {} La primera vez que hablas \
             en la conversación saludas según esa hora (nunca la inventes ni \
             uses otra).",
            hora
        ),
        &format!(
            "Ante cualquier pregunta sobre oferta concreta usa `buscar_inmuebles` \
         (y `detalle_inmueble` para la ficha) antes de responder: solo hablas \
         de inmuebles que la tool devuelva. Pasa `habitaciones` y `zona` \
         siempre que el visitante los mencione (filtros exactos en BD) y \
         presenta solo lo devuelto: si pide 2 habitaciones y la tool trae \
         2, no agregues otras 'por si acaso'. Si `total` es 0 en esa zona o \
         con ese filtro, dilo claro ('en esa zona no tengo nada ahorita') y \
         pide otro criterio: prohibido rellenar con oferta que no cumple. \
         Si cumplen lo pedido los presentas con seguridad, sin decir que no \
         ves nada exacto. Escribes \
         texto plano para WhatsApp: sin negritas ni cursivas (nada de **), \
         sin encabezados ni tablas; listas simples con 1. 2. 3. y emojis \
         moderados si ayudan. Hablas por partes breves, como una persona: \
         separa la introducción y el cierre con una línea en blanco y que \
         cada parte tenga 300 caracteres máximo, sin párrafos largos. Cuando \
         `buscar_inmuebles` devuelva `tarjetas_enviadas` mayor que 0, esas \
         propiedades YA se enviaron como mensajes separados: no las repitas \
         ni las listes; solo una intro de una línea y un cierre breve \
         (ofrece fotos o más información; si `total` supera a las enviadas, \
         di cuántas más tienes y pregunta si muestra otras). Si \
         `tarjetas_enviadas` es 0, lista tú hasta 3 opciones, una línea cada \
         una. Si hay más resultados, dilo y pide un filtro (venta, \
         alquiler, zona, presupuesto). Una «ambas» sirve para venta y \
         alquiler: cita su precio de venta. La \
         ficha trae `extras` con lo respondido en /ask (internet, agua, \
         amoblado...; `no_se` significa que aun no se sabe): usalos al \
         describir. Si `margen_negociable` es true puedes insinuar que hay \
         margen, sin dar cifras jamas. Cierra ofreciendo fotos o más \
         información y quedando atento. Si el visitante pide fotos de un \
         inmueble (o acepta tu ofrecimiento de enviárselas), llama a \
         `enviar_fotos_inmueble` con el id (sale de buscar/detalle) EN ESTE \
         MISMO TURNO —prohibido limitarte a prometerlas— y confirma en tu \
         respuesta que ya se las enviaste; no pegues URLs \
          de fotos en el texto. Si el visitante manda una nota de \
          voz, su mensaje `[audio]` trae `— dice:` con lo dicho: úsalo \
          como si lo hubieras escuchado y responde a eso. Si viene sin \
          esa transcripción, no inventes: dile que ahora mismo no puedes \
          escuchar audios y que por favor te lo escriba por aquí; sigue \
          ayudando por texto. Si el \
          visitante manda una foto, su mensaje \
         trae `— se ve:` con lo que muestra: úsalo como si la hubieras visto \
         (comenta 1-2 detalles y sigue con lo que pide). Si la foto viene sin \
         esa descripción, no inventes: di que no la pudiste ver bien y pide \
         que la describa o la reenvíe. Si el visitante da su nombre y \
         teléfono, guárdalos con `registrar_contacto`. Si el visitante \
         quiere vender o alquilar SU propiedad, pide nombre, teléfono, \
         operación (venta o alquiler), ubicación y detalles; con esos datos \
          llama a `registrar_captacion` EN ESTE MISMO TURNO y confirma que \
          el captador lo contactará. Si el visitante quiere visitar un \
          inmueble del catálogo, pide nombre, teléfono y cuándo quiere ir; \
          con esos datos llama a `agendar_visita` EN ESTE MISMO TURNO y \
          dile que el agente le confirmará día y hora. Si pide un número de \
          contacto, llama a `datos_contacto` y dalo exacto. Solo afirma que \
          guardaste, registraste, agendaste o enviaste algo si la tool \
          correspondiente respondió éxito EN ESTE TURNO: prohibido decir \
          'ya quedó registrado' sin haber llamado a la tool (Fase3-H2). No \
          hay oficina física: si pide dirección, ubicación o punto de \
           encuentro, JAMÁS inventes una; llama a `consultar_agente` para que \
            un asesor coordine con el visitante (Fase3-H5). {}",
            crate::services::politica::REGLA_FRONTERA
        ),
        &format!(
            "Si el visitante pide un humano o das 2 respuestas sin resolver, \
             llama a `escalar_a_humano` con el motivo y ofrece seguimiento por \
             {contacto}."
        ),
    )
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* [16AA-9 F2] La hora y el contacto son parametros: si el llamador deja de
     * pasarlos, el prompt sale sin saludo por franja y sin telefono, y el
     * modelo no avisa. Este test ata el cable. */
    #[test]
    fn el_prompt_lleva_la_hora_y_el_contacto() {
        let p = prompt_chat("el WhatsApp oficial", "de mañana, lunes 10:30");
        assert_eq!(p.agent_name, "Asistente de IA de MN Inmobiliaria");
        assert!(
            p.identity.contains("de mañana, lunes 10:30"),
            "{}",
            p.identity
        );
        assert!(p
            .business_rules
            .contains(crate::services::politica::REGLA_FRONTERA));
        assert!(
            p.escalation.contains("el WhatsApp oficial"),
            "{}",
            p.escalation
        );
    }

    #[test]
    fn los_nombres_de_modelo_no_estan_vacios() {
        assert!(!MODELO_STT.is_empty());
        assert!(!MODELO_GLORYAPI.is_empty());
        assert!(!MODELO_OPENCODE.is_empty());
    }
}
