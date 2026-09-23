import { useEffect, useRef, useState } from 'react';
import { Download, Loader2, Pencil } from 'lucide-react';
import {
  lineaTitulo1De,
  recetaVigenteDe,
  resolverReceta,
  tituloZonaDe,
  type ComposicionPublicidad,
  type RecetaPublicidad,
} from '@/domain/plantilla-publicidad';
import { fotosVisiblesDe, type Inmueble } from '@/domain/inmueble';
import { exportarPublicidad, renderizarPublicidad } from '@/platform/canvas-publicidad';
import { usePublicidades } from '@/hooks/publicidad/use-publicidades';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ModalEditorPublicidad } from './modal-editor-publicidad';

/* Vista previa: renderiza la composición al ancho pedido. */
export function VistaPrevia({
  inmueble,
  comp,
  ancho = 394,
}: {
  inmueble: Inmueble;
  comp: ComposicionPublicidad;
  ancho?: number;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const generacion = useRef(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    generacion.current += 1;
    const mia = generacion.current;
    setError(null);
    renderizarPublicidad(ref.current!, inmueble, comp, ancho, () => generacion.current === mia).catch(() => {
      if (generacion.current === mia) setError('No se pudo dibujar (revisa las fotos).');
    });
    return () => {
      // Al cambiar formato/receta la generación avanza y el render
      // anterior queda invalidado: no pinta ni reporta error.
      if (generacion.current === mia) generacion.current += 1;
    };
  }, [inmueble, comp, ancho]);

  if (error) return <p className="p-4 text-xs text-destructive">{error}</p>;
  return <canvas ref={ref} className="h-auto w-full" />;
}

/* Botones JPG/PNG con estado de descarga. */
function BotonesExportar({ inmueble, comp }: { inmueble: Inmueble; comp: ComposicionPublicidad }) {
  const [exportando, setExportando] = useState<'png' | 'jpeg' | null>(null);
  const [fallo, setFallo] = useState<string | null>(null);

  const exportar = async (tipo: 'png' | 'jpeg') => {
    setExportando(tipo);
    setFallo(null);
    try {
      await exportarPublicidad(inmueble, comp, tipo);
    } catch {
      setFallo('Falló la exportación.');
    } finally {
      setExportando(null);
    }
  };

  return (
    <div className="space-y-1.5">
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          className="h-7 text-xs"
          disabled={exportando !== null}
          onClick={() => void exportar('jpeg')}
        >
          {exportando === 'jpeg' ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />}
          JPG
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="h-7 text-xs"
          disabled={exportando !== null}
          onClick={() => void exportar('png')}
        >
          {exportando === 'png' ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />}
          PNG
        </Button>
      </div>
      {fallo ? <p className="text-xs text-destructive">{fallo}</p> : null}
    </div>
  );
}

/* Tarjeta de un inmueble con su imagen publicitaria + acciones. */
function TarjetaPublicidad({
  inmueble,
  comp,
  alEditar,
  alAmpliar,
}: {
  inmueble: Inmueble;
  comp: ComposicionPublicidad;
  alEditar: () => void;
  alAmpliar: () => void;
}) {
  return (
    <article className="overflow-hidden rounded-lg border bg-card">
      <button
        type="button"
        onClick={alAmpliar}
        title="Ver completa"
        className="block w-full cursor-zoom-in"
      >
        <VistaPrevia inmueble={inmueble} comp={comp} />
      </button>
      <div className="space-y-2 p-3">
        <div>
          <p className="truncate text-sm font-medium">{lineaTitulo1De(inmueble)}</p>
          <p className="truncate text-xs text-muted-foreground">en {tituloZonaDe(inmueble)}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={alEditar}>
            <Pencil className="h-3.5 w-3.5" /> Editar
          </Button>
        </div>
        <BotonesExportar inmueble={inmueble} comp={comp} />
      </div>
    </article>
  );
}

/* Página Publicidad: galería de imágenes generadas, una por inmueble con
 * fotos. Cada tarjeta permite verla completa, cambiar las 3 fotos y
 * descargar JPG/PNG. */
export function VistaPublicidad({
  inmuebles,
  alEditarPropiedad,
  alActualizarInmueble,
}: {
  inmuebles: Inmueble[];
  alEditarPropiedad?: (inmueble: Inmueble) => void;
  /* El guardado de la receta hace su propio PUT: con esto la lista se
   * refresca con el inmueble ya persistido (sin releer todo). */
  alActualizarInmueble?: (inmueble: Inmueble) => void;
}) {
  const { recetaDe, guardar } = usePublicidades();
  const [editando, setEditando] = useState<Inmueble | null>(null);
  const [fallo, setFallo] = useState<string | null>(null);
  const [ampliando, setAmpliando] = useState<{ inmueble: Inmueble; comp: ComposicionPublicidad } | null>(null);

  const lista = inmuebles.filter((i) => fotosVisiblesDe(i).length > 0);
  /* El editor trabaja con el objeto fresco de la lista: si la propiedad se
   * editó fuera (título, ubicación, fotos), la vista previa recarga sola. */
  const editandoFresco = editando ? (inmuebles.find((i) => i.id === editando.id) ?? editando) : null;
  /* Receta vigente (guardada/migrada o automática por índices 0-1-2) y
   * composición resuelta a URLs para dibujar. Manda la del servidor (la
   * que ve el frente público); la local queda de reserva. */
  const recetaDeInmueble = (inmueble: Inmueble): RecetaPublicidad | null =>
    recetaVigenteDe(inmueble, inmueble.receta ?? recetaDe(inmueble.id));
  const compDe = (inmueble: Inmueble): ComposicionPublicidad | null => {
    const receta = recetaDeInmueble(inmueble);
    return receta ? resolverReceta(inmueble, receta) : null;
  };

  return (
    <div className="space-y-4">
      {fallo && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {fallo}
        </p>
      )}
      {lista.length === 0 ? (
        <p className="rounded-lg border bg-card p-6 text-center text-sm text-muted-foreground">
          Aún no hay inmuebles con fotos para generar publicidad.
        </p>
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {lista.map((inmueble) => {
            const comp = compDe(inmueble);
            if (!comp) return null;
            return (
              <TarjetaPublicidad
                key={inmueble.id}
                inmueble={inmueble}
                comp={comp}
                alEditar={() => setEditando(inmueble)}
                alAmpliar={() => setAmpliando({ inmueble, comp })}
              />
            );
          })}
        </div>
      )}
      <ModalEditorPublicidad
        key={editandoFresco?.id ?? 'cerrado'}
        inmueble={editandoFresco}
        receta={editandoFresco ? recetaDeInmueble(editandoFresco) : null}
        alGuardar={async (receta) => {
          if (!editandoFresco) return;
          setFallo(null);
          try {
            const actualizado = await guardar(editandoFresco, receta);
            alActualizarInmueble?.(actualizado);
            setEditando(null);
          } catch (e) {
            // La copia local quedó; se avisa para reintentar con red.
            setFallo(e instanceof Error ? e.message : 'No se pudo guardar la publicidad en el servidor.');
          }
        }}
        alCerrar={() => setEditando(null)}
        alEditarPropiedad={
          alEditarPropiedad
            ? (inmueble) => {
                setEditando(null);
                alEditarPropiedad(inmueble);
              }
            : undefined
        }
      />
      <Dialog open={ampliando !== null} onOpenChange={(abierto) => !abierto && setAmpliando(null)}>
        <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {ampliando ? lineaTitulo1De(ampliando.inmueble) : 'Publicidad'}
            </DialogTitle>
          </DialogHeader>
          {ampliando ? (
            <div className="space-y-3">
              <div className="overflow-hidden rounded-lg border">
                <VistaPrevia inmueble={ampliando.inmueble} comp={ampliando.comp} ancho={700} />
              </div>
              <BotonesExportar inmueble={ampliando.inmueble} comp={ampliando.comp} />
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}
