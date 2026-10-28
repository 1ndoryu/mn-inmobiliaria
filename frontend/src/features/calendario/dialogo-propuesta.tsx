/* [28AA-6] Propuesta de la cola: el siguiente inmueble que toca publicar. Se
 * puede publicar (crea el borrador del día) o saltar (queda como visto, sin contar). */
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import type { Propuesta } from '@/domain/redes/publicacion-redes';

interface DialogoPropuestaProps {
  propuesta: Propuesta;
  ocupado: boolean;
  alPublicar: () => void;
  alSaltar: () => void;
  alCerrar: () => void;
}

export function DialogoPropuesta({ propuesta, ocupado, alPublicar, alSaltar, alCerrar }: DialogoPropuestaProps) {
  return (
    <Dialog
      open
      onOpenChange={(abierto: boolean) => {
        if (!abierto && !ocupado) alCerrar();
      }}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Toca publicar</DialogTitle>
          <DialogDescription>
            Siguiente de la cola: <strong className="text-foreground">{propuesta.titulo}</strong>. Pasa a publicada
            cuando los cuatro destinos estén marcados.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={alSaltar} disabled={ocupado}>
            Saltar
          </Button>
          <Button onClick={alPublicar} disabled={ocupado}>
            Publicar este
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
