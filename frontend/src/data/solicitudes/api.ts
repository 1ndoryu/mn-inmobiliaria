/* [169A-2] Cliente admin de solicitudes "Publicar mi inmueble": listado y
 * cambio de estado de revisión con JWT. `apiFetch` gestiona sesión y 401. */
import type { EstadoSolicitud, PaginaSolicitudes, Solicitud } from '../../domain/solicitud';
import { apiFetch } from '../inmuebles/api';

interface SolicitudApi {
  id: string;
  nombre: string;
  telefono: string;
  email: string;
  descripcion: string;
  ubicacion: string;
  puestos: number;
  residencia: string;
  precio_estimado: number | null;
  operacion: string;
  estado: string;
  fotos: { storage_key: string; url: string }[];
  origen_contacto: string;
  created_at: string;
}

interface PaginaApi {
  items: SolicitudApi[];
  total: number;
  page: number;
  per_page: number;
}

function aSolicitud(s: SolicitudApi): Solicitud {
  return {
    id: s.id,
    nombre: s.nombre,
    telefono: s.telefono,
    email: s.email,
    descripcion: s.descripcion,
    ubicacion: s.ubicacion,
    puestos: s.puestos,
    residencia: s.residencia,
    precioEstimado: s.precio_estimado,
    operacion: s.operacion,
    // El servidor solo guarda los cuatro estados (CHECK en BD): el cast es seguro.
    estado: s.estado as EstadoSolicitud,
    fotos: s.fotos.map((f) => f.url),
    origenContacto: s.origen_contacto,
    creadaEn: s.created_at,
  };
}

export async function listarSolicitudes(opciones: {
  estado?: EstadoSolicitud;
  page: number;
  perPage: number;
}): Promise<PaginaSolicitudes> {
  const query = new URLSearchParams({ page: String(opciones.page), per_page: String(opciones.perPage) });
  if (opciones.estado) query.set('estado', opciones.estado);
  const r = await apiFetch<PaginaApi>(`/api/admin/solicitudes?${query.toString()}`);
  return { items: r.items.map(aSolicitud), total: r.total, page: r.page, perPage: r.per_page };
}

export async function cambiarEstadoSolicitud(id: string, estado: EstadoSolicitud): Promise<void> {
  await apiFetch<SolicitudApi>(`/api/admin/solicitudes/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify({ estado }),
  });
}
