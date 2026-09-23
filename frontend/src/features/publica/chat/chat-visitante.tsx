/* Ventana de chat del visitante (169A-1, F5): solo se pinta si el
 * padre la abre (`abierto`, desde el boton Mensaje del header). Sin
 * burbuja propia, sin redondeados ni sombras; colores y texto solo de
 * `disenno.ts` (receta publica: #e8e7e3/#050200/#dddbd5).
 * El estado vive en `useChatVisitante`; aqui solo presentacion.
 * [169A-6] Tarjeta de contacto/escalado: se abre sola si el visitante
 * pide un humano y tambien via enlace; ofrece telefono/WhatsApp de
 * `pedirInfo` + mini-formulario (`ContactoChat`). Sin info, no se ofrece. */

import { useEffect, useRef } from 'react';
import { Button } from '@/components/ui/button';
import { useChatVisitante } from '../../../hooks/publica/chat/use-chat-visitante';
import { ContactoChat } from './contacto-chat';
import {
  CLASE_ACTIVO,
  CLASE_BORDE,
  CLASE_FONDO,
  CLASE_RELLENO_SUAVE,
  CLASE_TEXTO,
} from '../disenno';

/* Pide un humano con palabras propias: la tarjeta se abre sola. */
const PIDE_HUMANO = /humano|persona|agente|llam|tel[eé]fono|whatsapp|contacto/i;

export function ChatVisitante({ abierto, alCerrar }: { abierto: boolean; alCerrar: () => void }) {
  const { estado, poner, enviar, info, tarjeta, setTarjeta } = useChatVisitante(abierto);
  const listaRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const lista = listaRef.current;
    if (lista) lista.scrollTop = lista.scrollHeight;
  }, [estado.mensajes.length, abierto]);

  /* Escalado visible sin IA: se evalua al enviar, no en efecto. */
  function alEnviar(e: React.FormEvent) {
    e.preventDefault();
    if (PIDE_HUMANO.test(estado.texto)) setTarjeta(true);
    void enviar();
  }

  if (!abierto) return null;

  return (
    <>
    <section
      aria-label="Chat de ayuda"
      className={`fixed bottom-4 right-4 z-50 flex h-[min(70dvh,520px)] w-[min(92vw,360px)] flex-col border ${CLASE_BORDE} ${CLASE_FONDO} ${CLASE_TEXTO}`}
    >
      <header className="flex items-center justify-between bg-[#050200] px-3 py-2 text-[#e8e7e3]">
        <p>
          Asistente · IA{estado.enLinea ? '' : ' (reconectando…)'}
        </p>
        <Button type="button" variant="ghost" onClick={alCerrar} aria-label="Cerrar chat" className="rounded-none px-2 py-1 text-[#e8e7e3]">
          Cerrar
        </Button>
      </header>
      <div ref={listaRef} className="flex-1 overflow-y-auto px-3 py-2">
        {estado.mensajes.length === 0 && (
          <p className="py-6 text-center">
            Hola, soy la IA de la inmobiliaria. Pregunta por compra, venta o alquiler.
          </p>
        )}
        {estado.mensajes.map((m) => (
          <p
            key={m.id}
            className={`my-1 break-words px-3 py-2 ${
              m.propio ? 'ml-8 bg-[#050200] text-[#e8e7e3]' : `mr-8 ${CLASE_RELLENO_SUAVE} text-black`
            }`}
          >
            {m.texto}
          </p>
        ))}
        {estado.ocupado && <p className="py-1 text-center">Escribiendo…</p>}
        {estado.error && <p className="border border-[#050200] px-3 py-2 text-center">{estado.error}</p>}
      </div>
      {info && !tarjeta && (
        <Button type="button" variant="ghost" onClick={() => setTarjeta(true)} className="rounded-none px-3 py-1 underline">
          ¿Hablamos por WhatsApp o te llamamos?
        </Button>
      )}
      <form
        className={`flex border-t ${CLASE_BORDE}`}
        onSubmit={alEnviar}
      >
        <input
          value={estado.texto}
          onChange={(e) => poner({ texto: e.target.value })}
          placeholder="Escribe tu mensaje…"
          aria-label="Escribe tu mensaje"
          className="min-w-0 flex-1 bg-transparent px-3 py-2 outline-none"
        />
        <Button type="submit" variant="ghost" disabled={estado.ocupado} className={`rounded-none px-4 py-2 ${CLASE_ACTIVO}`}>
          Enviar
        </Button>
      </form>
    </section>
    {info && <ContactoChat info={info} abierto={tarjeta} alCerrar={() => setTarjeta(false)} />}
    </>
  );
}
