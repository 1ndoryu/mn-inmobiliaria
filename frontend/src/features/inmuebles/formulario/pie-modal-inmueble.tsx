/* [25AA-1] Pie del modal de inmueble: descartar borrador, error del guardado y los dos
 * botones. Mientras guarda, ambos quedan inertes para que un clic extra no lance otro envío. */
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { DialogFooter } from '@/components/ui/dialog';

interface Props {
  esEdicion: boolean;
  guardando: boolean;
  /* Solo en «nuevo» y con contenido: ofrece tirar el borrador. */
  puedeDescartar: boolean;
  /* Motivo del último guardado fallido: el banner de la app queda tapado por el modal. */
  errorGuardado?: string | null;
  alDescartar: () => void;
  alCerrar: () => void;
  alGuardar: () => void;
}

export function PieModalInmueble({ esEdicion, guardando, puedeDescartar, errorGuardado, alDescartar, alCerrar, alGuardar }: Props) {
  return (
    <DialogFooter>
      {puedeDescartar && (
        <Button type="button" variant="ghost" onClick={alDescartar} className="sm:mr-auto">
          <Trash2 /> Descartar borrador
        </Button>
      )}
      {errorGuardado && (
        <p role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive sm:mr-auto">
          {errorGuardado}
        </p>
      )}
      <Button type="button" variant="outline" disabled={guardando} onClick={alCerrar}>
        {esEdicion ? 'Cancelar' : 'Cerrar (conserva borrador)'}
      </Button>
      <Button type="button" disabled={guardando} aria-busy={guardando} onClick={alGuardar}>
        {guardando ? 'Guardando…' : esEdicion ? 'Guardar cambios' : 'Guardar inmueble'}
      </Button>
    </DialogFooter>
  );
}
