// Utilidades HTTP del mini-backend: respuestas JSON con CORS solo para el frontal
// (127.0.0.1:5199) y lectura del cuerpo acotada en tamaño.
export const ORIGEN_FRONTAL = 'http://127.0.0.1:5199';

export function json(res, codigo, cuerpo) {
  const texto = JSON.stringify(cuerpo);
  res.writeHead(codigo, {
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': ORIGEN_FRONTAL,
    'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  });
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
