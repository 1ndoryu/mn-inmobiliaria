import { useEffect, useState } from 'react';
import { EVENTO_SESION_EXPIRADA, borrarSesion, leerEmailSesion, leerToken } from '../../data/inmuebles/api';
import { suscribirEvento } from '../../platform/ventana';

// Sesión del admin contra la API: el token vive en localStorage (dura 1
// año y sobrevive al cierre del navegador). `email === null` = hay que entrar.
export function useSesion() {
  const [email, setEmail] = useState<string | null>(() =>
    leerToken() ? (leerEmailSesion() ?? 'admin') : null,
  );

  useEffect(() => {
    const alExpirar = () => setEmail(null);
    return suscribirEvento(EVENTO_SESION_EXPIRADA, alExpirar);
  }, []);

  function salir() {
    borrarSesion();
    setEmail(null);
  }

  return { email, alEntrar: setEmail, salir };
}
