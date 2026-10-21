// Servidor HTTP del mini-backend de mejora: CORS (OPTIONS), despacho a las rutas
// de estado y de trabajos, y 404 para lo demás. Lo escucha la entrada.
import { createServer } from 'node:http';
import { cabecerasCors, json } from './http.mjs';
import { atenderEstado } from './rutas-estado.mjs';
import { atenderTrabajos } from './rutas-trabajos.mjs';

export const servidor = createServer(async (req, res) => {
  const url = new URL(req.url || '/', 'http://127.0.0.1');
  /* [15AA-12] El CORS se fija aquí, con el origen de esta petición, antes de
   * despachar: las respuestas (`json`) solo ponen el Content-Type. */
  for (const [clave, valor] of Object.entries(cabecerasCors(req.headers.origin))) {
    res.setHeader(clave, valor);
  }
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }
  if (await atenderEstado(req, res, url)) return;
  if (await atenderTrabajos(req, res, url)) return;
  json(res, 404, { error: 'Ruta no encontrada.' });
});
