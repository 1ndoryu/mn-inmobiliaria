// Utilidades HTTP del mini-backend: CORS para los orígenes que sirven el panel
// y lectura del cuerpo acotada en tamaño.
/* [15AA-12] El panel ya no vive solo en el dev de Vite (127.0.0.1:5199): en
 * `build` lo sirve la 89, así que también llaman el puerto 3110 (túnel) y el dev
 * público. La lista se amplía con `MEJORA_ORIGENES` (separados por comas) sin
 * tocar código. */
const ORIGENES = new Set(
  [
    'http://127.0.0.1:5199',
    'http://127.0.0.1:3110',
    'http://localhost:3110',
    'https://mn-dev.wandori.us',
    ...(process.env.MEJORA_ORIGENES || '').split(','),
  ]
    .map((o) => o.trim())
    .filter(Boolean),
);

/* Cabeceras de CORS del origen que llama. Un origen no listado recibe el del dev
 * de siempre: el navegador lo rechaza, que es justo lo que se busca. */
export function cabecerasCors(origen) {
  return {
    'Access-Control-Allow-Origin': origen && ORIGENES.has(origen) ? origen : 'http://127.0.0.1:5199',
    'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
    Vary: 'Origin',
    /* Chrome lo exige cuando la página es pública (https) y llama a 127.0.0.1:
     * es el permiso de «acceso a red local». */
    'Access-Control-Allow-Private-Network': 'true',
  };
}

export function json(res, codigo, cuerpo) {
  const texto = JSON.stringify(cuerpo);
  /* Solo el tipo: el CORS lo fija el servidor antes de despachar, que es quien
   * conoce el origen de la petición. */
  res.writeHead(codigo, { 'Content-Type': 'application/json; charset=utf-8' });
  res.end(texto);
}

/* Lee el cuerpo como texto; si supera el límite (20 MB por foto), corta la conexión. */
export function leerCuerpo(req, limiteBytes = 20 * 1024 * 1024) {
  return new Promise((resolve, reject) => {
    const partes = [];
    let total = 0;
    req.on('data', (c) => {
      total += c.length;
      if (total > limiteBytes) {
        reject(new Error('La foto supera los 20MB.'));
        req.destroy();
        return;
      }
      partes.push(c);
    });
    req.on('end', () => resolve(Buffer.concat(partes).toString('utf8')));
    req.on('error', reject);
  });
}
