import { useCallback, useEffect, useState } from 'react';
import { FolderOpen, HardDriveDownload, Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';

/* [30AA-1] Sincronización a disco desde la app de escritorio.
 *
 * Solo aparece dentro de MN-Escritorio: ese binario expone `/__exportar` y
 * `/__sincronizacion`. En el navegador esos endpoints no existen (404 del
 * backend) y el bloque no se pinta.
 *
 * La sincronización es AUTOMÁTICA (el exe la corre cada diez minutos y solo baja
 * lo que cambió); el botón es solo para forzarla ahora. El destino es
 * `%USERPROFILE%\Downloads\MN Inmobiliaria\version 1|2\<slug>\`, con `01.jpg`,
 * `02.jpg`… y una `ficha.txt` que lleva título y descripción juntos. */

interface Resumen {
  ok: boolean;
  cuenta?: string;
  inmuebles?: number;
  fotos?: number;
  bajadas?: number;
  ruta?: string;
  detalle?: string;
}

interface EstadoSincro {
  activa: boolean;
  enCurso?: boolean;
  ultimaVez?: string | null;
  detalle?: string | null;
}

const SONDA = '/__carpeta-export';
const ESTADO = '/__sincronizacion';

/* La sonda es local y tiene que responder al instante; la sincronización cruza
 * el túnel y baja fotos, así que su tope es amplio: cortarla antes solo dejaría
 * carpetas a medias. */
const TOPE_SONDA_MS = 2000;
const TOPE_SINCRO_MS = 30 * 60 * 1000;

export function ExportarEscritorio() {
  const [disponible, setDisponible] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  const [resumen, setResumen] = useState<Resumen | null>(null);
  const [estado, setEstado] = useState<EstadoSincro | null>(null);

  const leerEstado = useCallback(async () => {
    try {
      const r = await fetch(ESTADO, { signal: AbortSignal.timeout(TOPE_SONDA_MS) });
      if (r.ok) setEstado((await r.json()) as EstadoSincro);
    } catch {
      /* Sin servidor local (navegador): no hay nada que enseñar. */
    }
  }, []);

  useEffect(() => {
    let vivo = true;
    fetch(SONDA, { signal: AbortSignal.timeout(TOPE_SONDA_MS) })
      .then((r) => (r.ok ? r.json() : null))
      .then((d: { ruta?: string | null } | null) => {
        if (!vivo || !d) return;
        setDisponible(true);
        void leerEstado();
      })
      .catch(() => {
        /* Navegador: el endpoint no existe. Sin bloque y sin ruido. */
      });
    return () => {
      vivo = false;
    };
  }, [leerEstado]);

  if (!disponible) return null;

  async function sincronizar() {
    setOcupado(true);
    setResumen(null);
    try {
      const r = await fetch('/__exportar', { signal: AbortSignal.timeout(TOPE_SINCRO_MS) });
      setResumen((await r.json()) as Resumen);
    } catch (e) {
      setResumen({ ok: false, detalle: e instanceof Error ? e.message : 'No se pudo sincronizar.' });
    } finally {
      setOcupado(false);
      void leerEstado();
    }
  }

  return (
    <div className="grid gap-2 rounded-lg border p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" disabled={ocupado} onClick={() => void sincronizar()}>
          {ocupado ? <Loader2 className="animate-spin" /> : <HardDriveDownload />}
          {ocupado ? 'Sincronizando…' : 'Sincronizar ahora'}
        </Button>
        <span className="flex items-center gap-1 text-xs text-muted-foreground">
          <RefreshCw className="h-3 w-3" />
          {estado?.activa
            ? 'Automático: cada 10 min, y solo baja lo que cambió.'
            : 'Automático desactivado (sin túnel al arrancar).'}
        </span>
      </div>
      {estado?.detalle && (
        <p className="text-xs text-muted-foreground">
          Última: {estado.ultimaVez} · {estado.detalle}
        </p>
      )}
      {resumen && (
        <p
          role="status"
          className={
            resumen.ok
              ? 'flex items-center gap-1 text-xs text-muted-foreground'
              : 'text-xs text-destructive'
          }
        >
          {resumen.ok ? (
            <>
              <FolderOpen className="h-3.5 w-3.5" /> {resumen.inmuebles} inmuebles y {resumen.fotos} fotos de «
              {resumen.cuenta}» en {resumen.ruta}
              {resumen.bajadas !== undefined && ` (${resumen.bajadas} fotos bajadas)`}
            </>
          ) : (
            resumen.detalle
          )}
        </p>
      )}
    </div>
  );
}
