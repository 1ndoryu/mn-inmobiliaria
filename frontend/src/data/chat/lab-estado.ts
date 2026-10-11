// Estado del lab (10AA-23): `GET /api/admin/marketplace/lab/estado` clasifica
// el latido del token CLI (el lab consulta `borradores/version` cada 5 s).
// Reutiliza `apiFetch` (JWT admin + 401). El backend habla snake_case.

import type { ConexionLab, ResumenLab } from '@/domain/lab';
import { apiFetch } from '../inmuebles/api';

interface EstadoLabCrudo {
  estado: string;
  ultimo_ok_hace_s: number | null;
}

const CONEXIONES: Record<string, ConexionLab> = {
  conectado: 'conectado',
  sin_senal: 'sin-senal',
  rechazado: 'rechazado',
};

export async function leerEstadoLab(signal?: AbortSignal): Promise<ResumenLab> {
  const crudo = await apiFetch<EstadoLabCrudo>('/api/admin/marketplace/lab/estado', { signal });
  return { conexion: CONEXIONES[crudo.estado] ?? 'sin-leer', ultimoOkHaceS: crudo.ultimo_ok_hace_s };
}
