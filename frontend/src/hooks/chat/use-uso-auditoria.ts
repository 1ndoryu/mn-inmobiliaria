import { useEffect, useState } from 'react';
import { leerAuditoria, leerUso, type AuditoriaFila, type UsoDia } from '../../data/chat/cliente-duena';
import { ErrorApi } from '../../data/inmuebles/api';

function mensajeError(e: unknown): string {
  return e instanceof ErrorApi ? e.message : 'Fallo inesperado.';
}

const pedirDatos = () => Promise.all([leerUso(7), leerAuditoria(50)]);

/* Carga inicial de uso y auditoría. Si el componente se desmonta antes de que
 * llegue la respuesta, se descarta. `recargar` no se descarta: lo dispara el
 * usuario y su resultado siempre se quiere ver. */
export function useUsoAuditoria() {
  const [uso, setUso] = useState<UsoDia[]>([]);
  const [auditoria, setAuditoria] = useState<AuditoriaFila[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [cargando, setCargando] = useState(true);

  useEffect(() => {
    let viva = true;
    pedirDatos()
      .then(([u, a]) => {
        if (!viva) return;
        setUso(u);
        setAuditoria(a);
        setError(null);
      })
      .catch((e: unknown) => viva && setError(mensajeError(e)))
      .finally(() => viva && setCargando(false));
    return () => {
      viva = false;
    };
  }, []);

  const recargar = () => {
    setCargando(true);
    pedirDatos()
      .then(([u, a]) => {
        setUso(u);
        setAuditoria(a);
        setError(null);
      })
      .catch((e: unknown) => setError(mensajeError(e)))
      .finally(() => setCargando(false));
  };

  return { uso, auditoria, error, cargando, recargar };
}
