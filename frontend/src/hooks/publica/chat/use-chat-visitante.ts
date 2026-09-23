/* Estado del chat del visitante (169A-1, F5): mensajes, borrador,
 * ocupado y errores para pintar. El transporte (WS + REST + reconexion)
 * vive en `useCanalChat`; aqui solo estado de UI. La apertura la manda el
 * padre (`abierto`): al abrir se conecta, al cerrar se corta el socket. */

import { useEffect, useState } from 'react';
import type { InfoAgente, MensajeChat } from '../../../data/chat/cliente-chat';
import { pedirInfo } from '../../../data/chat/cliente-chat';
import { useCanalChat } from './use-canal-chat';

interface EstadoChat {
  mensajes: MensajeChat[];
  texto: string;
  ocupado: boolean;
  error: string | null;
  enLinea: boolean;
}

const ESTADO_INICIAL: EstadoChat = {
  mensajes: [],
  texto: '',
  ocupado: false,
  error: null,
  enLinea: false,
};

export function useChatVisitante(abierto: boolean) {
  const [estado, setEstado] = useState<EstadoChat>(ESTADO_INICIAL);
  const { conectar, enviarTexto, cerrar } = useCanalChat();
  /* Tarjeta de contacto/escalado + info del agente: viven aqui para que
   * el componente quede solo presentacion (regla componente-sin-hook). */
  const [info, setInfo] = useState<InfoAgente | null>(null);
  const [tarjeta, setTarjeta] = useState(false);

  /* La info de contacto se carga al abrir; si falla, el chat sigue
   * funcionando y simplemente no se ofrece la tarjeta. */
  useEffect(() => {
    if (!abierto) return;
    let viva = true;
    pedirInfo()
      .then((i) => {
        if (viva) setInfo(i);
      })
      .catch(() => {
        /* Intencional: sin info no se ofrece la tarjeta, sin romper el chat. */
      });
    return () => {
      viva = false;
    };
  }, [abierto]);

  function poner(cambio: Partial<EstadoChat>) {
    setEstado((e) => ({ ...e, ...cambio }));
  }

  useEffect(() => {
    if (!abierto) {
      cerrar();
      return;
    }
    void conectar(
      (m) => {
        setEstado((e) => ({ ...e, mensajes: [...e.mensajes, m], ocupado: false }));
      },
      (error) => setEstado((e) => ({ ...e, error })),
      (enLinea) =>
        setEstado((e) => ({ ...e, enLinea, error: enLinea ? null : e.error })),
    );
    return () => cerrar();
  }, [abierto, conectar, cerrar]);

  /* Por socket el eco y la IA llegan solos; por REST se pintan aqui. */
  async function enviar() {
    const texto = estado.texto.trim();
    if (!texto || estado.ocupado) return;
    poner({ ocupado: true, error: null, texto: '' });
    try {
      const res = await enviarTexto(texto);
      if (res.porSocket) return;
      const ahora = Date.now();
      setEstado((e) => ({
        ...e,
        mensajes: [...e.mensajes, { id: `temp-${ahora}`, propio: true, texto }],
      }));
      if (res.respuestaIa) {
        setEstado((e) => ({
          ...e,
          mensajes: [...e.mensajes, { id: `ia-${ahora}`, propio: false, texto: res.respuestaIa as string }],
        }));
      } else poner({ error: 'Sin respuesta automática: deja tu contacto y te llamamos.' });
    } catch (error) {
      poner({ error: error instanceof Error ? error.message : 'No se pudo enviar.' });
    } finally {
      poner({ ocupado: false });
    }
  }

  return { estado, poner, enviar, info, tarjeta, setTarjeta };
}
