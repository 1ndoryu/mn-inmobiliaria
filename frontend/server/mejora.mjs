// Mini-backend local de mejora (127.0.0.1:3122). Arranque: `npm run server:mejora`.
// Este fichero solo arranca el servicio; la lógica está en server/mejora/:
// estado (config y estado vivo), registro (logs y eventos), worker (Python),
// cola (bomba y reintentos), credenciales (.env.local), http, rutas y servidor.
import { promises as fs } from 'node:fs';
import { LOG_DIR, MAX_INTENTOS, PUERTO, TMP, ritmo } from './mejora/estado.mjs';
import { evento, informar } from './mejora/registro.mjs';
import { cargarEnvLocal, vigilarEnvLocal } from './mejora/credenciales.mjs';
import { bombear, listo } from './mejora/cola.mjs';
import { servidor } from './mejora/servidor.mjs';

await cargarEnvLocal();
vigilarEnvLocal(); // F22: renovar-cookies reescribe .env.local y se recarga solo.
await fs.mkdir(TMP, { recursive: true }).catch(() => {});
await fs.mkdir(LOG_DIR, { recursive: true }).catch(() => {});
evento('arranque', null, null, `Puerto ${PUERTO}, intervalo ${ritmo.intervaloSeg}s, jitter ${ritmo.jitterPct}%, maxDia ${ritmo.maxPorDia}, maxIntentos ${MAX_INTENTOS}.`);
servidor.listen(PUERTO, '127.0.0.1', () => {
  informar(`[mejora] http://127.0.0.1:${PUERTO} listo=${listo()} intervalo=${ritmo.intervaloSeg}s jitter=${ritmo.jitterPct}% maxDia=${ritmo.maxPorDia} maxIntentos=${MAX_INTENTOS}`);
});
/* Latido cada 30s: despierta reintentos vencidos, libera el tope tras la
 * medianoche y reanuda la cola aunque el último bombear no dejara cadena. */
setInterval(() => void bombear(), 30 * 1000).unref();
