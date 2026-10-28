/* [28AA-6] Editor de una publicación: descripción y etiquetas (con IA opcional),
 * destinos, piezas e historial. Guarda su propia copia para que un error aquí
 * se vea dentro del modal y no quede tapado por él. */
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import type { Inmueble } from '@/domain/inmueble';
import type { Publicacion } from '@/domain/redes/publicacion-redes';
import { useEditorPublicacion } from '@/hooks/publicaciones/use-editor-publicacion';
import { cn } from '@/lib/utils';
import { CLASE_ESTADO, ETIQUETA_ESTADO } from './calendario-util';
import { ChecklistDestinos } from './destinos-publicacion';
import { HistorialPublicacion } from './historial-publicacion';
import { PiezasPublicacion } from './piezas-publicacion';

interface EditorPublicacionProps {
  publicacion: Publicacion;
  inmueble: Inmueble | undefined;
  alCambiar: (publicacion: Publicacion) => void;
  alBorrar: () => void;
  alCerrar: () => void;
}

export function EditorPublicacion({ publicacion, inmueble, alCambiar, alBorrar, alCerrar }: EditorPublicacionProps) {
  const {
    pub,
    setPub,
    destinos,
    setDestinos,
    descripcion,
    setDescripcion,
    tagsTexto,
    setTagsTexto,
    generando,
    guardando,
    ocupado,
    publicada,
    confirmarBorrado,
    error,
    generar,
    guardar,
    borrar,
  } = useEditorPublicacion({ publicacion, inmueble, alCambiar, alBorrar });

  return (
    <Dialog
      open
      onOpenChange={(abierto: boolean) => {
        if (!abierto && !ocupado) alCerrar();
      }}
    >
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex flex-wrap items-center gap-2">
            {pub.tituloInmueble}
            <span className={cn('rounded border px-1.5 py-0.5 text-xs font-normal', CLASE_ESTADO[pub.estado])}>
              {ETIQUETA_ESTADO[pub.estado]}
            </span>
          </DialogTitle>
          <DialogDescription>
            Cuando los cuatro destinos estén marcados, la publicación pasa a publicada.
          </DialogDescription>
        </DialogHeader>

        {error && (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        )}

        <section className="flex flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between gap-2">
              <label htmlFor="descripcion-publicacion" className="text-sm font-medium">
                Descripción
              </label>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void generar()}
                disabled={ocupado || !inmueble}
              >
                {generando ? 'Generando…' : 'Generar con IA'}
              </Button>
            </div>
            <Textarea
              id="descripcion-publicacion"
              rows={6}
              maxLength={2200}
              value={descripcion}
              onChange={(e) => setDescripcion(e.target.value)}
              disabled={ocupado}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label htmlFor="tags-publicacion" className="text-sm font-medium">
              Etiquetas (separadas por coma)
            </label>
            <Textarea
              id="tags-publicacion"
              rows={2}
              value={tagsTexto}
              onChange={(e) => setTagsTexto(e.target.value)}
              disabled={ocupado}
            />
          </div>

          <ChecklistDestinos destinos={destinos} deshabilitado={ocupado} alCambiar={setDestinos} />
        </section>

        <section className="flex flex-col gap-2">
          <h3 className="text-sm font-medium">Piezas</h3>
          <PiezasPublicacion
            publicacion={pub}
            bloqueado={publicada || pub.estado === 'saltada'}
            alActualizar={(actualizada) => {
              setPub(actualizada);
              alCambiar(actualizada);
            }}
          />
        </section>

        <section className="flex flex-col gap-2">
          <h3 className="text-sm font-medium">Historial de salidas</h3>
          <HistorialPublicacion inmuebleId={pub.inmuebleId} />
        </section>

        <DialogFooter className="flex-wrap justify-between gap-2">
          <Button variant="destructive" onClick={() => void borrar()} disabled={ocupado || publicada}>
            {confirmarBorrado ? 'Confirmar borrado' : 'Borrar'}
          </Button>
          <div className="flex gap-2">
            <Button variant="outline" onClick={alCerrar} disabled={ocupado}>
              Cerrar
            </Button>
            <Button onClick={() => void guardar()} disabled={ocupado}>
              {guardando ? 'Guardando…' : 'Guardar cambios'}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
