/* Modal de contacto/escalado del chat (169A-6): cuando el visitante
 * pide un humano (o toca el enlace), se ofrece el telefono y WhatsApp
 * configurados en el backend mas un mini-formulario que deja su contacto
 * para que staff lo llame (se ve en la bandeja Mensajes). Misma receta
 * que el modal de filtros (fondo, borde, inputs con etiqueta, boton
 * primario; cierra con overlay o Escape, sin X). Solo presentacion +
 * envio; la decision de mostrarla la toma el padre con `abierto`. */

import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useContactoChat } from '../../../hooks/publica/chat/use-contacto-chat';
import type { InfoAgente } from '../../../data/chat/cliente-chat';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_FONDO, CLASE_TEXTO, CLASE_TINTA } from '../disenno';

export function ContactoChat({
  info,
  abierto,
  alCerrar,
}: {
  info: InfoAgente;
  abierto: boolean;
  alCerrar: () => void;
}) {
  const { nombre, setNombre, telefono, setTelefono, envio, fallo, guardar } = useContactoChat();

  return (
    <Dialog open={abierto} onOpenChange={(a) => !a && alCerrar()}>
      <DialogContent
        showCloseButton={false}
        className={`max-h-[92dvh] overflow-y-auto rounded-none border ${CLASE_BORDE} ${CLASE_FONDO} ${CLASE_TINTA} p-6 font-soehne font-normal sm:max-w-md`}
      >
        <DialogTitle className="sr-only">Contacto</DialogTitle>
        <div className="mt-4 flex flex-col gap-4 font-normal">
          {info.contactoTelefono && <p>Llámanos: {info.contactoTelefono}</p>}
          {info.whatsappUrl && (
            <a href={info.whatsappUrl} target="_blank" rel="noreferrer" className="underline">
              Abrir WhatsApp
            </a>
          )}
          {envio === 'ok' ? (
            <p>Listo: te llamamos en breve.</p>
          ) : (
            <form
              className="flex flex-col gap-4"
              onSubmit={(e) => {
                e.preventDefault();
                void guardar();
              }}
            >
              <label className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
                Tu nombre
                <input
                  value={nombre}
                  onChange={(e) => setNombre(e.target.value)}
                  placeholder="Tu nombre"
                  aria-label="Tu nombre"
                  className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none placeholder:text-black/40`}
                />
              </label>
              <label className={`flex flex-col gap-1 text-sm font-normal ${CLASE_TINTA}`}>
                Tu teléfono
                <input
                  value={telefono}
                  onChange={(e) => setTelefono(e.target.value)}
                  placeholder="Tu teléfono"
                  aria-label="Tu teléfono"
                  inputMode="tel"
                  className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm font-normal outline-none placeholder:text-black/40`}
                />
              </label>
              {fallo && <p className="text-center">{fallo}</p>}
              <Button
                type="submit"
                variant="ghost"
                disabled={envio === 'enviando'}
                className={`w-full cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO}`}
              >
                {envio === 'enviando' ? 'Guardando…' : 'Que me llamen'}
              </Button>
            </form>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
