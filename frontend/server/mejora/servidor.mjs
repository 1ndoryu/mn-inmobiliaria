// Servidor HTTP del mini-backend de mejora: CORS (OPTIONS), despacho a las rutas
// de estado y de trabajos, y 404 para lo demás. Lo escucha la entrada.
import { createServer } from 'node:http';
import { ORIGEN_FRONTAL, json } from './http.mjs';
import { atenderEstado } from './rutas-estado.mjs';
import { atenderTrabajos } from './rutas-trabajos.mjs';

export const servidor = createServer(async (req, res) => {
  const url = new URL(req.url || '/', 'http://127.0.0.1');
  if (req.method === 'OPTIONS') {
    res.writeHead(204, {
      'Access-Control-Allow-Origin': ORIGEN_FRONTAL,
      'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type',
    });
    res.end();
    return;
  }
  if (await atenderEstado(req, res, url)) return;
  if (await atenderTrabajos(req, res, url)) return;
  json(res, 404, { error: 'Ruta no encontrada.' });
});
