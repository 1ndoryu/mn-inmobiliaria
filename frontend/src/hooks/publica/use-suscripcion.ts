import { useState, type FormEvent } from 'react';
import { ErrorApi } from '../../data/inmuebles/api';
import { suscribir } from '../../data/suscriptores/api';

/* Suscripción del pie: correo + fase del envío con mensaje siempre visible
 * (nunca fallos silenciosos). El pie solo renderiza este estado. */

type FaseSuscripcion = 'reposo' | 'enviando' | 'ok' | 'error';

export function useSuscripcion() {
  const [correo, setCorreo] = useState('');
  const [fase, setFase] = useState<FaseSuscripcion>('reposo');
  const [mensaje, setMensaje] = useState('');

  async function enviar(evento: FormEvent) {
    evento.preventDefault();
    if (fase === 'enviando') return;
    setFase('enviando');
    setMensaje('');
    try {
      await suscribir(correo.trim());
      setFase('ok');
      setMensaje('¡Suscripción lista! Te avisaremos de las novedades.');
      setCorreo('');
    } catch (e: unknown) {
      setFase('error');
      setMensaje(e instanceof ErrorApi ? e.message : 'No se pudo completar la suscripción.');
    }
  }

  return { correo, setCorreo, fase, mensaje, enviar };
}
