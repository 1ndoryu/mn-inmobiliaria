/* [13AA-12] Columna «Sync» y banner del estado de sincronización con el servidor.
 * Presentacional: el estado llega del hook (`useSyncInmuebles`); aquí solo se pinta. */
import { useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { etiquetaEstado } from '@/domain/sync/sync-inmueble';
import type { EntradaSync, EstadoRemotoSync, EstadoSync } from '@/data/inmuebles/sync';
import { Badge } from '@/components/ui/badge';
import { Button, ButtonPlano } from '@/components/ui/button';
import { ModalDiferenciasSync } from '@/features/inmuebles/modal-diferencias-sync';
import { cn } from '@/lib/utils';

const ETIQUETA_REMOTO: Record<EstadoRemotoSync['estado'], string> = {
  iniciando: 'Iniciando…',
  desactivado: 'Desactivado',
  sin_credenciales: 'Sin credenciales',
  sin_conexion: 'Sin conexión',
  sin_soporte: 'Sin soporte',
  activo: 'Activo',
};

const MAX_SOLO_SERVIDOR_VISIBLES = 5;

function descripcionRemoto(remoto: EstadoRemotoSync): string {
  switch (remoto.estado) {
    case 'iniciando':
      return 'Conectando con el servidor…';
    case 'desactivado':
      return 'La comparación con el servidor está desactivada en este entorno.';
    case 'sin_credenciales':
      return 'Faltan las credenciales del servidor: no se puede comparar.';
    case 'sin_conexion':
      return `No hay conexión con el servidor: ${remoto.motivo}`;
    case 'sin_soporte':
      return `El servidor no admite la comparación: ${remoto.motivo}`;
    case 'activo':
      return '';
  }
}

interface SyncBadgeProps {
  /* Ausente en borradores sin guardar. */
  slug?: string;
  titulo: string;
  estado: EstadoSync | null;
  entrada?: EntradaSync;
}

/* Estado de un inmueble frente al servidor. Sin servidor disponible muestra el
 * estado global (sin comparar) en lugar de un estado por inmueble falso. El modal
 * de diferencias se monta solo al abrirlo: cada celda guarda su propio estado. */
export function SyncBadge({ slug, titulo, estado, entrada }: SyncBadgeProps) {
  const [verDiferencias, setVerDiferencias] = useState(false);
  if (!slug) {
    return (
      <Badge variant="outline" className="text-muted-foreground" title="Borrador sin guardar: aún no existe en el servidor">
        Sin guardar
      </Badge>
    );
  }
  if (!estado) {
    return <span className="text-xs text-muted-foreground">…</span>;
  }
  if (estado.remoto.estado !== 'activo') {
    return (
      <Badge variant="outline" className="text-muted-foreground" title={descripcionRemoto(estado.remoto)}>
        {ETIQUETA_REMOTO[estado.remoto.estado]}
      </Badge>
    );
  }
  if (!entrada) {
    return (
      <Badge variant="outline" className="text-muted-foreground" title="Recién guardado: la comparación llega en segundos">
        Calculando…
      </Badge>
    );
  }
  const { resultado } = entrada;
  const { texto, clase } = etiquetaEstado(resultado.estado, resultado.estimado);
  const hayDiferencias = resultado.estado !== 'igual' && resultado.estado !== 'calculando';
  return (
    <span className="inline-flex flex-wrap items-center gap-1.5">
      <Badge variant="secondary" className={cn('whitespace-nowrap', clase)}>
        {texto}
      </Badge>
      {hayDiferencias && (
        <ButtonPlano
          type="button"
          onClick={() => setVerDiferencias(true)}
          className="rounded text-xs font-medium text-primary underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
        >
          Resolver
        </ButtonPlano>
      )}
      {verDiferencias && (
        <ModalDiferenciasSync
          slug={slug}
          titulo={titulo}
          revision={estado.revision}
          alCerrar={() => setVerDiferencias(false)}
        />
      )}
    </span>
  );
}

interface BannerSyncProps {
  estado: EstadoSync | null;
  error: string | null;
  alComprobar: () => Promise<void>;
}

/* Estado global: servidor, fotos aún sin huella, inmuebles solo en el servidor. */
export function BannerSync({ estado, error, alComprobar }: BannerSyncProps) {
  const [comprobando, setComprobando] = useState(false);

  async function comprobar() {
    setComprobando(true);
    try {
      await alComprobar();
    } finally {
      setComprobando(false);
    }
  }

  const remoto = estado?.remoto;
  const activo = remoto?.estado === 'activo';
  const soloServidor = estado?.comparacion?.solo_servidor ?? [];
  const titulosVisibles = soloServidor
    .slice(0, MAX_SOLO_SERVIDOR_VISIBLES)
    .map((s) => s.titulo || s.slug)
    .join(', ');
  const restantes = soloServidor.length - MAX_SOLO_SERVIDOR_VISIBLES;

  return (
    <section aria-label="Sincronización con el servidor" className="mb-3 space-y-2 rounded-lg border bg-card px-3 py-2 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="flex flex-wrap items-center gap-2 text-muted-foreground">
          <span className="font-medium text-foreground">Sincronización</span>
          {remoto && (
            <Badge variant={activo ? 'secondary' : 'outline'} className={cn(activo && 'border-transparent bg-emerald-100 text-emerald-900')}>
              {ETIQUETA_REMOTO[remoto.estado]}
            </Badge>
          )}
          {activo && estado && (
            <span className="text-xs">
              Actualizado {new Date(estado.generado_en).toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            </span>
          )}
        </p>
        <Button type="button" variant="outline" size="sm" onClick={() => void comprobar()} disabled={comprobando}>
          <RefreshCw className={cn('h-3.5 w-3.5', comprobando && 'animate-spin')} aria-hidden="true" />
          Comprobar ahora
        </Button>
      </div>
      {!estado && !error && <p className="text-muted-foreground">Cargando el estado de sincronización…</p>}
      {remoto && !activo && <p className="text-muted-foreground">{descripcionRemoto(remoto)}</p>}
      {error && <p className="text-destructive">{error}</p>}
      {activo && estado && estado.fotos_pendientes_local > 0 && (
        <p className="text-muted-foreground">
          Calculando la huella de {estado.fotos_pendientes_local} foto(s) local(es); la comparación de esos inmuebles puede tardar.
        </p>
      )}
      {soloServidor.length > 0 && (
        <p>
          <span className="font-medium">Solo en el servidor ({soloServidor.length}):</span> {titulosVisibles}
          {restantes > 0 && ` y ${restantes} más`}
        </p>
      )}
    </section>
  );
}
