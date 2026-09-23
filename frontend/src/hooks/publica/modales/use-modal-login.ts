import { useCallback, useState } from 'react';
import { leerToken } from '../../../data/inmuebles/api';
import { useLogin } from '../../sesion/use-login';
import { irA } from '../../../platform/ventana';

/* Modal "Entrar al panel" desde la web pública: reutiliza el login contra
 * la API y al entrar navega a `/admin`. Si ya hay sesión, el botón lleva
 * directo sin abrir el modal. */
export function useModalLogin() {
  const [abierto, setAbierto] = useState(false);
  const login = useLogin(() => {
    setAbierto(false);
    irA('/admin');
  });

  const abrir = useCallback(() => {
    if (leerToken()) {
      irA('/admin');
      return;
    }
    setAbierto(true);
  }, []);

  const cerrar = useCallback(() => setAbierto(false), []);

  return { abierto, abrir, cerrar, login };
}
