import type { InmueblePublico } from '../../../domain/inmueble';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { CLASE_BORDE, CLASE_FONDO, CLASE_TINTA } from '../disenno';
import { ContenidoInmueble } from '../detalle/contenido-inmueble';

/* Detalle público del inmueble: se abre al pulsar cualquier parte de la
 * caja. Mismo lenguaje que la lista: cuadrado, tinta, sin sombras, Söhne
 * 400 sin negritas. Cierra con overlay o Escape (sin X). El contenido es el
 * mismo que la página `/inmueble/<slug>` (`ContenidoInmueble`). */
export function ModalDetallePublico({
  inmueble,
  alCerrar,
}: {
  inmueble: InmueblePublico | null;
  alCerrar: () => void;
}) {
  return (
    <Dialog open={inmueble !== null} onOpenChange={(abierto) => !abierto && alCerrar()}>
      <DialogContent
        showCloseButton={false}
        className={`max-h-[95dvh] overflow-y-auto rounded-none border ${CLASE_BORDE} ${CLASE_FONDO} ${CLASE_TINTA} p-0 font-soehne font-normal sm:max-w-5xl`}
      >
        {inmueble && <ContenidoInmueble key={inmueble.id} inmueble={inmueble} tituloComo="dialogo" />}
      </DialogContent>
    </Dialog>
  );
}
