import { useState } from 'react';
import { ErrorApi, entrar, registrarPropietario } from '@/data/inmuebles/api';

/* Estado del formulario de entrada (1 solo useState con objeto). */

// Puerta del panel: login contra la API. Si aún no hay usuarios, el mismo
// formulario crea la cuenta propietaria inicial (el backend cierra el
// registro con 403 en cuanto existe una).
interface EstadoLogin {
  email: string;
  clave: string;
  ocupado: boolean;
  error: string | null;
}

export function useLogin(alEntrar: (email: string) => void) {
  const [estado, setEstado] = useState<EstadoLogin>({ email: '', clave: '', ocupado: false, error: null });

  const setEmail = (email: string) => setEstado((e) => ({ ...e, email }));
  const setClave = (clave: string) => setEstado((e) => ({ ...e, clave }));

  async function ejecutar(accion: 'entrar' | 'registrar') {
    if (estado.ocupado) return;
    setEstado((e) => ({ ...e, error: null }));
    if (!estado.email.trim() || !estado.clave) {
      setEstado((e) => ({ ...e, error: 'Escribe el correo y la contraseña.' }));
      return;
    }
    setEstado((e) => ({ ...e, ocupado: true }));
    try {
      const quien =
        accion === 'entrar'
          ? await entrar(estado.email, estado.clave)
          : await registrarPropietario(estado.email, estado.clave);
      alEntrar(quien);
    } catch (e) {
      setEstado((s) => ({ ...s, error: e instanceof ErrorApi ? e.message : 'No se pudo entrar.' }));
    } finally {
      setEstado((s) => ({ ...s, ocupado: false }));
    }
  }

  return {
    email: estado.email,
    clave: estado.clave,
    ocupado: estado.ocupado,
    error: estado.error,
    setEmail,
    setClave,
    ejecutar,
  };
}
