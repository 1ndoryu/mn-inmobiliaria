/* [10AA-27] Marcas de lado del excerpt. Lo escriben el lab (`Dueña:` = la
 * dueña, `Cliente:`) y el backend (`Tú:`/`Tu:`/`You:` en 08AA-29). Solo estas
 * palabras se quitan: un `Hora: 5pm` dentro del mensaje queda intacto. */
const MARCA_LADO = /^(tú|tu|you|dueña|dueno|cliente)\s*:\s*/i;
const MARCAS_PROPIAS = new Set(['tú', 'tu', 'you', 'dueña', 'dueno']);

export type MensajeExcerpt = { propia: boolean; texto: string };

/* [10AA-27] Una línea con marca abre mensaje; una sin marca continúa el
 * anterior (mensaje de varias líneas). Sin marca al principio: del cliente.
 * Antes, cada línea sin marca salía como «Cliente» con el texto duplicado. */
export function mensajesDelExcerpt(excerpt: string): MensajeExcerpt[] {
  const salida: MensajeExcerpt[] = [];
  for (const linea of excerpt.split('\n')) {
    const texto = linea.trim();
    if (texto === '') continue;
    const marca = texto.match(MARCA_LADO);
    if (marca) {
      salida.push({
        propia: MARCAS_PROPIAS.has(marca[1].toLowerCase()),
        texto: texto.slice(marca[0].length).trim(),
      });
    } else if (salida.length > 0) {
      const anterior = salida[salida.length - 1];
      anterior.texto = `${anterior.texto} ${texto}`.trim();
    } else {
      salida.push({ propia: false, texto });
    }
  }
  return salida;
}
