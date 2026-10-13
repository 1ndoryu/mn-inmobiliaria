import { Suspense, lazy, useState } from 'react';
import { useModalLogin } from '../../../hooks/publica/modales/use-modal-login';
import { useModalPublicar } from '../../../hooks/publica/modales/use-modal-publicar';
import { useMontarAlAbrir } from '../../../hooks/publica/use-montar-al-abrir';
import { abrirExterna } from '../../../platform/ventana';
import { enlaceWhatsApp, esEntornoLocal, mensajeGeneral } from '../../../platform/whatsapp';
import { CabeceraPublica } from '../cabecera-publica';

/* Mismos overlays `lazy` que la portada (fuera del JS inicial). */
const ModalPublicar = lazy(() => import('../modales/modal-publicar').then((m) => ({ default: m.ModalPublicar })));
const ModalLogin = lazy(() => import('../modales/modal-login').then((m) => ({ default: m.ModalLogin })));
const ChatVisitante = lazy(() => import('../chat/chat-visitante').then((m) => ({ default: m.ChatVisitante })));

/* Cabecera del sitio con sus acciones (Mensaje, Publicar, Entrar) para las
 * páginas que no son la portada: trae sus propios overlays, con la misma
 * lógica que `PaginaPublica`. */
export function CabeceraAcciones() {
  const [chatAbierto, setChatAbierto] = useState(false);
  const publicar = useModalPublicar();
  const acceso = useModalLogin();
  const publicarMontado = useMontarAlAbrir(publicar.abierto);
  const accesoMontado = useMontarAlAbrir(acceso.abierto);
  const chatMontado = useMontarAlAbrir(chatAbierto);

  /* El chat con IA solo funciona en local; en producción abre WhatsApp. */
  const pedirContacto = () => {
    if (esEntornoLocal()) {
      setChatAbierto(true);
      return;
    }
    abrirExterna(enlaceWhatsApp(mensajeGeneral()));
  };

  return (
    <>
      <CabeceraPublica alPedirChat={pedirContacto} alPedirPublicar={publicar.abrir} alPedirEntrar={acceso.abrir} />
      <Suspense fallback={null}>
        {publicarMontado && <ModalPublicar modal={publicar} />}
        {accesoMontado && <ModalLogin modal={acceso} />}
        {chatMontado && <ChatVisitante abierto={chatAbierto} alCerrar={() => setChatAbierto(false)} />}
      </Suspense>
    </>
  );
}
