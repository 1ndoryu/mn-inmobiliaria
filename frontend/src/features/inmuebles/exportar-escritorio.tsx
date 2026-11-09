import { useEffect, useState } from 'react';
import { FolderOpen, HardDriveDownload, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';

/* [30AA-1] Exportación a disco desde la app de escritorio.
 *
 * Solo aparece dentro de MN-Escritorio: ese binario expone `/__exportar`, que
 * baja el manifiesto del servidor y escribe fotos + textos en
 * `%LOCALAPPDATA%\MN-Escritorio\inmuebles\<cuenta>\version 1|2\<slug>\`. En el
 * navegador esos endpoints no existen (404 del backend) y el botón no se pinta.
 *
 * El orden importa: cada foto va como `01.jpg`, `02.jpg`… y su texto en
 * `titulo.txt`/`descripcion.txt`, así que la carpeta se puede subir tal cual. */

interface Resumen {
  ok: boolean;
  cuenta?: string;
  inmuebles?: number;
  fotos?: number;
  ruta?: string;
  detalle?: string;
}

const ES_ESCRITORIO = '/__carpeta-export';

/* La sonda es local y tiene que responder al instante; la exportación cruza el
 * túnel y baja todas las fotos, así que su tope es amplio: cortarla antes solo
 * dejaría carpetas a medias. */
const TOPE_SONDA_MS = 2000;
const TOPE_EXPORT_MS = 30 * 60 * 1000;

export function ExportarEscritorio() {
  const [disponible, setDisponible] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  const [resumen, setResumen] = useState<Resumen | null>(null);

  useEffect(() => {
    let vivo = true;
    fetch(ES_ESCRITORIO, { signal: AbortSignal.timeout(TOPE_SONDA_MS) })
      .then((r) => (r.ok ? r.json() : null))
      .then((d: { ruta?: string | null } | null) => {
        if (vivo) setDisponible(Boolean(d));
      })
      .catch(() => {
        /* Navegador: el endpoint no existe. Sin botón y sin ruido. */
      });
    return () => {
      vivo = false;
    };
  }, []);

  if (!disponible) return null;

  async function exportar() {
    setOcupado(true);
    setResumen(null);
    try {
      const r = await fetch('/__exportar', { signal: AbortSignal.timeout(TOPE_EXPORT_MS) });
      const d = (await r.json()) as Resumen;
      setResumen(d);
    } catch (e) {
      setResumen({ ok: false, detalle: e instanceof Error ? e.message : 'No se pudo exportar.' });
    } finally {
      setOcupado(false);
    }
  }

  return (
    <div className="grid gap-2 rounded-lg border p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" disabled={ocupado} onClick={() => void exportar()}>
          {ocupado ? <Loader2 className="animate-spin" /> : <HardDriveDownload />}
          {ocupado ? 'Exportando…' : 'Exportar a carpetas (v1 y v2)'}
        </Button>
        <span className="text-xs text-muted-foreground">
          Fotos y textos, ordenados, en «version 1» y «version 2». Va despacio: los bytes cruzan el túnel.
        </span>
      </div>
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
            </>
          ) : (
            resumen.detalle
          )}
        </p>
      )}
    </div>
  );
}
