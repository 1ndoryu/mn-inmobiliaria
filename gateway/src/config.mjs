import { fileURLToPath } from "node:url";

// Config del gateway Baileys (289A-1). Por entorno; los secretos pueden venir de gateway/.env (gitignored).
/* Carga gateway/.env sin pisar lo que ya trae el entorno. El guardián
 * (mantener.ps1) no inyecta variables: sin esto, el webhook queda abierto. */
try {
  process.loadEnvFile(fileURLToPath(new URL("../.env", import.meta.url)));
} catch {
  /* Sin .env: solo entorno, como antes. */
}

const num = (v, def) => {
  const n = Number.parseInt(v ?? "", 10);
  return Number.isFinite(n) ? n : def;
};

const flag = (v, def) => {
  const s = (v ?? String(def)).trim().toLowerCase();
  return !(s === "0" || s === "no" || s === "false");
};

export const config = {
  puerto: num(process.env.GATEWAY_PORT, 3102),
  backendWebhook: process.env.BACKEND_WEBHOOK_URL || "http://127.0.0.1:3110/api/agent/whatsapp/webhook", // [03AA-1] local en 3110: el 3000 lo ocupa glory-pulse.
  webhookSecreto: process.env.WA_WEBHOOK_SECRETO || "",
  sendSecreto: process.env.GATEWAY_SEND_SECRET || "",
  sesiones: {
    /* [289A-1] `respondeIA`: solo las vías en true reenvían inbound al
     * webhook. B es el teléfono manual de la dueña: mudo por defecto
     * (SESSION_B_RESPONDE_IA=1 lo reactiva sin tocar código). */
    wa_a: { nombre: "A", numero: (process.env.SESSION_A_NUMBER || "584120825234").replace(/\D/g, ""), respondeIA: flag(process.env.SESSION_A_RESPONDE_IA, 1) },
    wa_b: { nombre: "B", numero: (process.env.SESSION_B_NUMBER || "18149575416").replace(/\D/g, ""), respondeIA: flag(process.env.SESSION_B_RESPONDE_IA, 0) },
    /* Tercer número: mudo (no responde solo a clientes). El backend no lo
     * conoce; se controla por /send con via=wa_c. Su número sale al vincular. */
    wa_c: { nombre: "C", numero: (process.env.SESSION_C_NUMBER || "").replace(/\D/g, ""), respondeIA: flag(process.env.SESSION_C_RESPONDE_IA, 0) },
  },
  mediaDir: new URL("../media/", import.meta.url),
  mediaTtlMs: 30 * 60 * 1000,
};
