/* Rescate puntual del historial WhatsApp (10AA-2): el gateway estuvo
 * apagado y los mensajes que llegaron en ese lapso nunca se archivaron.
 * Conecta la sesión con `syncFullHistory` y baja las fotos recientes del
 * historial del teléfono, guardándolas con la misma convención del backend
 * (`uploads/whatsapp/<tel>/<uuid>.<ext>`) + manifiesto JSON. NO toca el
 * webhook (sin turnos IA, sin sesiones, sin envíos): solo archivos.
 * Uso: node src/rescatar-historial.mjs --sesion=wa_a [--dias=7]
 * OJO: usa las mismas credenciales que el gateway vivo → apagarlo antes. */
import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import makeWASocket, {
  useMultiFileAuthState,
  fetchLatestBaileysVersion,
  downloadMediaMessage,
  DisconnectReason,
} from "@whiskeysockets/baileys";
import { config } from "./config.mjs";

const RAIZ = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, "").split("=");
    return [k, v ?? true];
  })
);
const VIA = args.sesion ?? "wa_a";
const DIAS = Number(args.dias ?? 7);
const SALIDA = args.salida ?? "C:\\tmp\\rescate-wa";
const CORTE = Math.floor(Date.now() / 1000) - DIAS * 86400;

const sesion = config.sesiones[VIA];
if (!sesion) {
  console.error(`sesión desconocida: ${VIA}`);
  process.exit(1);
}
const dirAuth = fileURLToPath(new URL(`../sesiones/${VIA}/`, import.meta.url));
const UPLOADS = join(RAIZ, "..", "uploads", "whatsapp");
const EXT = { "image/jpeg": ".jpg", "image/png": ".png", "image/webp": ".webp" };

const digitos = (s) => (s ?? "").replace(/\D/g, "");
const remitenteDe = (m) =>
  digitos(m.key.senderPn) || digitos((m.key.remoteJid ?? "").split("@")[0]);

const manifiesto = [];
let historicoRecibido = false;
let procesadas = 0;
let archivadas = 0;

async function archivar(msg) {
  const remitente = remitenteDe(msg);
  if (!remitente) return;
  const img = msg.message?.imageMessage;
  const ts = Number(msg.messageTimestamp ?? 0);
  const texto = img?.caption ?? msg.message?.conversation ?? msg.message?.extendedTextMessage?.text ?? "";
  const entrada = {
    remitente,
    fecha: new Date(ts * 1000).toISOString(),
    texto,
    archivo: null,
  };
  if (img) {
    try {
      const buf = await downloadMediaMessage(msg, "buffer", {});
      let mime = "image/jpeg";
      if (buf[0] === 0x89 && buf[1] === 0x50) mime = "image/png";
      else if (buf[0] === 0x52 && buf[1] === 0x49) mime = "image/webp";
      const nombre = `${randomUUID()}${EXT[mime]}`;
      const carpeta = join(UPLOADS, remitente);
      await mkdir(carpeta, { recursive: true });
      await writeFile(join(carpeta, nombre), buf);
      entrada.archivo = `whatsapp/${remitente}/${nombre}`;
      archivadas++;
    } catch (e) {
      entrada.error = `media no descargable: ${e.message}`;
    }
  }
  manifiesto.push(entrada);
}

const { state, saveCreds } = await useMultiFileAuthState(dirAuth);
const { version } = await fetchLatestBaileysVersion();
const sock = makeWASocket({
  version,
  auth: state,
  syncFullHistory: true,
  markOnlineOnConnect: false,
  browser: ["MN-Rescate", "Chrome", "1.0"],
});
sock.ev.on("creds.update", saveCreds);

sock.ev.on("messaging-history.set", async ({ messages }) => {
  historicoRecibido = true;
  console.log(`[rescate] historial: ${messages.length} mensajes`);
  for (const m of messages) {
    if (m.key.fromMe || !m.message) continue;
    if (m.key.remoteJid?.endsWith("@g.us")) continue;
    if (m.key.remoteJid?.endsWith("@lid") && !m.key.senderPn) continue;
    if (Number(m.messageTimestamp ?? 0) < CORTE) continue;
    const tieneMedia = !!m.message?.imageMessage;
    const tieneTexto = !!(m.message?.conversation ?? m.message?.extendedTextMessage?.text);
    if (!tieneMedia && !tieneTexto) continue;
    procesadas++;
    await archivar(m);
  }
  console.log(`[rescate] candidatas: ${procesadas}, fotos archivadas: ${archivadas}`);
  await terminar();
});

sock.ev.on("connection.update", async ({ connection, lastDisconnect }) => {
  if (connection === "close") {
    const codigo = lastDisconnect?.error?.output?.statusCode;
    if (codigo === DisconnectReason.loggedOut) {
      console.error("[rescate] desvinculado: re-escanea el QR del gateway normal");
      process.exit(2);
    }
    console.log("[rescate] reconecto en 5s");
    setTimeout(() => process.exit(3), 5000);
  }
});

let fin = false;
async function terminar() {
  if (fin) return;
  fin = true;
  await mkdir(SALIDA, { recursive: true });
  const ruta = join(SALIDA, `manifiesto-${VIA}.json`);
  await writeFile(ruta, JSON.stringify(manifiesto, null, 2));
  console.log(`[rescate] manifiesto en ${ruta} (${manifiesto.length} entradas)`);
  process.exit(0);
}
setTimeout(() => {
  console.error(historicoRecibido ? "[rescate] timeout tras historial" : "[rescate] timeout sin historial");
  terminar();
}, 8 * 60 * 1000);
