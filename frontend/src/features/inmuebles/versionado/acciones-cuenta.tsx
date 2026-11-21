/* [30AA-19] Renombrar y borrar una cuenta de Facebook. Vive aparte de
 * `seccion-facebook` porque allí pasaba del límite de líneas del componente;
 * el estado local (nombre a medias, confirmación) no sale de aquí. */
import { useState } from 'react';
import { Check, Pencil, Trash2, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { CuentaFacebook } from '@/domain/versiones';

/* El borrado pide confirmación en el mismo botón (dos pasos, sin diálogo
 * encima del modal, que ya es uno) porque se lleva por cascada los vínculos
 * de todos sus inmuebles. */
export function AccionesCuenta({
  cuenta,
  ocupado,
  alEditar,
  alBorrar,
}: {
  cuenta: CuentaFacebook;
  ocupado: boolean;
  alEditar: (nombre: string) => void;
  alBorrar: () => void;
}) {
  const [editando, setEditando] = useState(false);
  const [nombre, setNombre] = useState(cuenta.nombre);
  const [confirmando, setConfirmando] = useState(false);
  const valido = nombre.trim().length > 0 && nombre.trim() !== cuenta.nombre;

  if (editando) {
    return (
      <span className="flex min-w-0 items-center gap-1">
        <Input
          value={nombre}
          onChange={(e) => setNombre(e.target.value)}
          className="h-7 w-32"
          aria-label={`Nuevo nombre de ${cuenta.nombre}`}
          autoFocus
          onKeyDown={(e) => {
            if (e.key === 'Enter' && valido) alEditar(nombre.trim());
            if (e.key === 'Escape') setEditando(false);
          }}
        />
        <Button
          size="icon-xs"
          variant="ghost"
          disabled={ocupado || !valido}
          onClick={() => alEditar(nombre.trim())}
          title="Guardar el nombre"
        >
          <Check />
        </Button>
        <Button size="icon-xs" variant="ghost" disabled={ocupado} onClick={() => setEditando(false)} title="Cancelar">
          <X />
        </Button>
      </span>
    );
  }

  return (
    <span className="flex items-center gap-1">
      <Button
        size="icon-xs"
        variant="ghost"
        disabled={ocupado}
        onClick={() => {
          setNombre(cuenta.nombre);
          setEditando(true);
        }}
        title="Renombrar esta cuenta"
      >
        <Pencil />
      </Button>
      {confirmando ? (
        <Button
          size="xs"
          variant="destructive"
          disabled={ocupado}
          onClick={alBorrar}
          onBlur={() => setConfirmando(false)}
          title="Borra la cuenta y desvincula todos sus inmuebles"
        >
          <Trash2 /> ¿Borrar?
        </Button>
      ) : (
        <Button
          size="icon-xs"
          variant="ghost"
          className="text-destructive"
          disabled={ocupado}
          onClick={() => setConfirmando(true)}
          title="Borrar esta cuenta"
        >
          <Trash2 />
        </Button>
      )}
    </span>
  );
}
