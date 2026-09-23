/* Canal de transporte del chat (169A-1, F5): WS realtime contra
 * /api/agent/ws con reconexion (max 5, backoff) + fallback REST cuando el
 * socket esta caido. Sin estado de UI: recibe callbacks y devuelve el
 * resultado del envio para que el hook de estado decida que pintar. */

import { useCallback, useEffect, useRef } from 'react';
import {
  aChat,
  crearSesion,
  enviarRest,
  leerSesion,
  pedirHistorial,
  urlWs,
  type MensajeChat,
  type MensajeServidor,
} from '../../../data/chat/cliente-chat';

/* Trama servidor→cliente: { type:'message', message, delivery }. */
interface EntranteWs {
  type?: unknown;
  message?: MensajeServidor;
}

export interface ResultadoEnvio {
  porSocket: boolean;
  respuestaIa: string | null;
}

type ConectarFn = (
  alRecibir: (m: MensajeChat) => void,
  alFallo: (error: string) => void,
  alLinea: (enLinea: boolean) => void,
) => Promise<void>;

export function useCanalChat() {
  const sesionRef = useRef<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const vistosRef = useRef<Set<string>>(new Set());
  const abiertoRef = useRef(false);
  const intentosRef = useRef(0);
  const temporizadorRef = useRef<number | null>(null);
  /* La reconexion se llama a si misma diferida: via ref para no leer
   * `conectar` durante su propia inicializacion (oxlint react). */
  const conectarRef = useRef<ConectarFn>(async () => {});
  /* Cuerpos enviados por REST (sin socket): el servidor los devuelve en el
   * proximo historial con id real; se consumen para no duplicar el eco. */
  const ecosRef = useRef<string[]>([]);

  const asegurarSesion = useCallback((): string => {
    const sesion = sesionRef.current ?? leerSesion() ?? crearSesion();
    sesionRef.current = sesion;
    return sesion;
  }, []);

  const conectar: ConectarFn = useCallback(
    async (alRecibir, alFallo, alLinea) => {
      if (wsRef.current && wsRef.current.readyState <= WebSocket.OPEN) return;
      abiertoRef.current = true;
      const sesion = asegurarSesion();
      const recibir = (m: MensajeChat) => {
        /* Eco de un envio REST: ya pintado con id temporal, se consume. */
        if (m.propio) {
          const eco = ecosRef.current.indexOf(m.texto);
          if (eco >= 0) {
            ecosRef.current.splice(eco, 1);
            vistosRef.current.add(m.id);
            return;
          }
        }
        alRecibir(m);
      };
      try {
        const historial = await pedirHistorial(sesion);
        for (const m of historial) {
          if (!vistosRef.current.has(m.id)) {
            vistosRef.current.add(m.id);
            recibir(aChat(m));
          }
        }
      } catch (error) {
        alFallo(error instanceof Error ? error.message : 'Sin historial.');
      }
      const ws = new WebSocket(urlWs(sesion));
      wsRef.current = ws;
      ws.onopen = () => {
        intentosRef.current = 0;
        alLinea(true);
      };
      ws.onmessage = (ev: MessageEvent<string>) => {
        try {
          const datos = JSON.parse(ev.data) as EntranteWs;
          if (datos.type === 'message' && datos.message && !vistosRef.current.has(datos.message.id)) {
            vistosRef.current.add(datos.message.id);
            recibir(aChat(datos.message));
          }
        } catch {
          // Trama ajena al protocolo: se ignora sin romper el socket.
        }
      };
      ws.onclose = () => {
        alLinea(false);
        if (!abiertoRef.current || intentosRef.current >= 5) return;
        const espera = 2 ** intentosRef.current * 1000;
        intentosRef.current += 1;
        temporizadorRef.current = window.setTimeout(
          () => void conectarRef.current(alRecibir, alFallo, alLinea),
          espera,
        );
      };
      ws.onerror = () => ws.close();
    },
    [asegurarSesion],
  );

  /* true = salio por socket (el eco y la IA llegan por `alRecibir`);
   * false = se envio por REST con la respuesta IA ya resuelta. */
  const enviarTexto = useCallback(
    async (texto: string): Promise<ResultadoEnvio> => {
      const sesion = asegurarSesion();
      const socket = wsRef.current;
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: 'send', body: texto }));
        return { porSocket: true, respuestaIa: null };
      }
      const res = await enviarRest(sesion, texto);
      ecosRef.current.push(texto);
      return { porSocket: false, respuestaIa: res.reply };
    },
    [asegurarSesion],
  );

  const cerrar = useCallback(() => {
    abiertoRef.current = false;
    if (temporizadorRef.current) window.clearTimeout(temporizadorRef.current);
    wsRef.current?.close();
    wsRef.current = null;
  }, []);

  useEffect(() => {
    conectarRef.current = conectar;
  }, [conectar]);

  useEffect(
    () => () => {
      if (temporizadorRef.current) window.clearTimeout(temporizadorRef.current);
      wsRef.current?.close();
    },
    [],
  );

  return { conectar, enviarTexto, cerrar };
}
