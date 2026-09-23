import { useState } from 'react';

const CLAVE_ASIDE = 'inmobiliaria:aside:v1';

function leerAsideGuardado(): boolean {
  try {
    return localStorage.getItem(CLAVE_ASIDE) === 'contraido';
  } catch {
    // Sin almacenamiento: arranca estirado.
    return false;
  }
}

function guardarAside(contraido: boolean): void {
  try {
    localStorage.setItem(CLAVE_ASIDE, contraido ? 'contraido' : 'estirado');
  } catch {
    // El estado en memoria sigue valiendo para la sesión.
  }
}

// Contracción del aside (admin): persiste entre vistas y recargas.
export function useAside() {
  const [contraido, setContraidoState] = useState<boolean>(leerAsideGuardado);

  const alternar = () => {
    setContraidoState((actual) => {
      guardarAside(!actual);
      return !actual;
    });
  };

  return { contraido, alternar };
}
