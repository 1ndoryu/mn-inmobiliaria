import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { CTA_DEFECTO, PROMPT_COPY_DEFECTO, type ConfigCopy } from '@/domain/copy';

/* Pestaña Copy del modal Configuración: instrucciones de tono, llamado a
 * la acción y generación automática. Formulario puro (sin guardar): el pie
 * del modal guarda mejora+copy juntos. */
export function PestanaCopy({
  formCopy,
  alCambiarCopy,
  mensaje,
}: {
  formCopy: ConfigCopy;
  alCambiarCopy: (c: ConfigCopy) => void;
  mensaje: string | null;
}) {
  return (
    <div className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm font-medium">
        Instrucciones para la IA
        <span className="-mt-0.5 text-xs font-normal text-muted-foreground">
          Tono y estilo de las 2 descripciones. Los datos del inmueble se añaden solos.
        </span>
        <Textarea
          rows={4}
          value={formCopy.prompt}
          onChange={(e) => alCambiarCopy({ ...formCopy, prompt: e.target.value })}
        />
      </label>
      <Button
        variant="ghost"
        size="sm"
        className="justify-start self-start text-xs"
        onClick={() => alCambiarCopy({ ...formCopy, prompt: PROMPT_COPY_DEFECTO })}
      >
        Restaurar texto original
      </Button>
      <label className="flex flex-col gap-1 text-sm font-medium">
        Llamado a la acción (final del pie)
        <Input value={formCopy.cta} onChange={(e) => alCambiarCopy({ ...formCopy, cta: e.target.value })} />
      </label>
      <Button
        variant="ghost"
        size="sm"
        className="justify-start self-start text-xs"
        onClick={() => alCambiarCopy({ ...formCopy, cta: CTA_DEFECTO })}
      >
        Restaurar llamado original
      </Button>
      <label className="flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm">
        <input
          type="checkbox"
          checked={formCopy.generarAlCrear}
          onChange={(e) => alCambiarCopy({ ...formCopy, generarAlCrear: e.target.checked })}
          className="mt-1"
        />
        <span>
          <span className="font-medium">Generar al subir el inmueble</span>
          <span className="block text-xs text-muted-foreground">
            Crea el copy automáticamente al añadir un inmueble nuevo.
          </span>
        </span>
      </label>
      {mensaje && <p className="text-xs text-amber-700">{mensaje}</p>}
    </div>
  );
}
