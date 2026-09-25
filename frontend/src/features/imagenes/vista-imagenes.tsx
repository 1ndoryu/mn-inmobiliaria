import { Hand, Images, WandSparkles } from 'lucide-react';
import type { ConfigMejora, FotoMejora } from '@/domain/foto-mejora';
import type { Inmueble } from '@/domain/inmueble';
import type { useColaMejora } from '@/hooks/mejora/use-cola-mejora';
import type { useFotosMejora } from '@/hooks/mejora/use-fotos-mejora';
import { Button } from '@/components/ui/button';
import { HistorialMejoras } from './historial-mejoras';
import { PaginaImagenes } from './pagina-imagenes';
import { TiraDiagnostico } from './tira-diagnostico';

interface VistaImagenesProps {
  fotos: FotoMejora[];
  inmuebles: Inmueble[];
  config: ConfigMejora;
  alCambiarConfig: (config: ConfigMejora) => void;
  cola: ReturnType<typeof useColaMejora>;
  historial: ReturnType<typeof useFotosMejora>['historial'];
  alLimpiarHistorial: () => void;
  alRestaurar: (inmuebleId: string, orden: number) => Promise<void>;
}

/* Vista de imágenes del panel: modo manual/automático, diagnóstico,
 * fotos con reintento y su historial. */
export function VistaImagenes({
  fotos,
  inmuebles,
  config,
  alCambiarConfig,
  cola,
  historial,
  alLimpiarHistorial,
  alRestaurar,
}: VistaImagenesProps) {
  const manual = config.modo === 'manual';

  const tituloDe = (inmuebleId: string) =>
    inmuebles.find((i) => i.id === inmuebleId)?.titulo || 'Inmueble eliminado';

  return (
    <>
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <div className="flex gap-1 rounded-lg border p-1">
          <Button
            variant={manual ? 'default' : 'ghost'}
            size="sm"
            className="h-7 text-xs"
            onClick={() => alCambiarConfig({ ...config, modo: 'manual' })}
          >
            <Hand className="h-3.5 w-3.5" /> Manual
          </Button>
          <Button
            variant={manual ? 'ghost' : 'default'}
            size="sm"
            className="h-7 text-xs"
            onClick={() => alCambiarConfig({ ...config, modo: 'automatico' })}
          >
            <WandSparkles className="h-3.5 w-3.5" /> Automático
          </Button>
        </div>
        <span className="text-xs text-muted-foreground">
          {manual
            ? 'Modo manual: nada se procesa solo; usa Reintentar en cada foto.'
            : 'Modo automático: las pendientes se encolan solas de una en una.'}
        </span>
      </div>
      {fotos.some((f) => f.estado === 'pendiente') && (
        <p className="mb-4 rounded-md border px-3 py-2 text-sm text-muted-foreground">
          <Images className="mr-1 inline h-4 w-4" />
          {manual
            ? 'Hay fotos pendientes: pulsa Reintentar en cada una para mejorarla.'
            : 'La cola automática procesa de una en una con pausas.'}
        </p>
      )}
      <TiraDiagnostico fotos={fotos} alReiniciarTodo={() => cola.reiniciarTodo()} />
      <PaginaImagenes
        inmuebles={inmuebles}
        fotos={fotos}
        ocupado={cola.ocupado}
        reintentos={cola.reintentos}
        alReintentar={(foto) => void cola.reintentar(foto, config.prompt)}
        alCancelar={(foto) => void cola.cancelar(foto)}
        alRestaurar={alRestaurar}
      />
      <div className="mt-6">
        <HistorialMejoras eventos={historial} tituloDe={tituloDe} alLimpiar={alLimpiarHistorial} />
      </div>
    </>
  );
}
