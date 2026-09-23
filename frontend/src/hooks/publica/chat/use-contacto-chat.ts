/* Mini-formulario de contacto del chat (169A-6): nombre + telefono que
 * staff ve en la bandeja Mensajes. Extraido del componente para cumplir
 * la regla de maximo 3 useState por componente. */
import { useState } from 'react';
import { enviarContacto, leerSesion } from '../../../data/chat/cliente-chat';

export type EstadoEnvio = 'reposo' | 'enviando' | 'ok';

export function useContactoChat() {
  const [nombre, setNombre] = useState('');
  const [telefono, setTelefono] = useState('');
  const [envio, setEnvio] = useState<EstadoEnvio>('reposo');
  const [fallo, setFallo] = useState<string | null>(null);

  async function guardar() {
    const sesion = leerSesion();
    if (!sesion) {
      setFallo('Aún no hay sesión de chat: escribe primero un mensaje.');
      return;
    }
    /* Regla simétrica al backend: el teléfono debe tener dígitos reales. */
    if (telefono.replace(/\D/g, '').length < 6) {
      setFallo('Escribe un teléfono válido (mínimo 6 dígitos).');
      return;
    }
    setEnvio('enviando');
    setFallo(null);
    try {
      await enviarContacto(sesion, nombre.trim() || 'Visitante del chat', telefono.trim());
      setEnvio('ok');
    } catch (error) {
      setEnvio('reposo');
      setFallo(error instanceof Error ? error.message : 'No se pudo guardar.');
    }
  }

  return { nombre, setNombre, telefono, setTelefono, envio, fallo, guardar };
}
