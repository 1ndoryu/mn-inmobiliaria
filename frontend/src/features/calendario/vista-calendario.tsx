/* [28AA-6] Vista del calendario de publicidad: «Publicar hoy» (elegir un inmueble de la
 * lista), rejilla mensual y editor de cada publicación. */
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import type { Inmueble } from '@/domain/inmueble';
import type { Publicacion } from '@/domain/redes/publicacion-redes';
import { aIso, useCalendario } from '@/hooks/publicaciones/use-calendario';
import { useEleccionHoy } from '@/hooks/publicaciones/use-eleccion-hoy';
import { DialogoEleccion } from './dialogo-eleccion';
import { EditorPublicacion } from './editor-publicacion';
import { RejillaMes } from './rejilla-mes';

interface VistaCalendarioProps {
  inmuebles: Inmueble[];
}

export function VistaCalendario({ inmuebles }: VistaCalendarioProps) {
  const cal = useCalendario();
  const [editando, setEditando] = useState<Publicacion | null>(null);
  const eleccion = useEleccionHoy(cal, setEditando);

  /* Publicables (mismo criterio que la cola del servidor), de mayor a menor precio. */
  const candidatos = useMemo(
    () => inmuebles.filter((i) => i.publicado && i.estado === 'disponible').sort((a, b) => b.precio - a.precio),
    [inmuebles],
  );

  const titulo = cal.mes.toLocaleString('es', { month: 'long', year: 'numeric' });
  const hoy = aIso(new Date());

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-1">
          <Button variant="outline" size="icon-sm" onClick={() => cal.cambiarMes(-1)} aria-label="Mes anterior">
            <ChevronLeft />
          </Button>
          <h2 className="min-w-40 text-center text-base font-semibold capitalize">{titulo}</h2>
          <Button variant="outline" size="icon-sm" onClick={() => cal.cambiarMes(1)} aria-label="Mes siguiente">
            <ChevronRight />
          </Button>
          <Button variant="outline" size="sm" onClick={cal.irAHoy}>
            Hoy
          </Button>
        </div>
        <Button onClick={() => void eleccion.abrir()} disabled={cal.cargando}>
          Publicar hoy
        </Button>
      </div>

      {cal.error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {cal.error}
        </p>
      )}

      <RejillaMes mes={cal.mes} publicaciones={cal.publicaciones} hoy={hoy} alElegir={(p) => setEditando(p)} />

      {eleccion.eligiendo && (
        <DialogoEleccion
          inmuebles={candidatos}
          ultimas={eleccion.ultimas}
          ocupado={eleccion.creando}
          alElegir={(id) => void eleccion.elegir(id)}
          alCerrar={eleccion.cerrar}
        />
      )}

      {editando && (
        <EditorPublicacion
          key={editando.id}
          publicacion={editando}
          inmueble={inmuebles.find((i) => i.id === editando.inmuebleId)}
          alCambiar={() => cal.recargar()}
          alBorrar={() => {
            setEditando(null);
            cal.recargar();
          }}
          alCerrar={() => setEditando(null)}
        />
      )}
    </div>
  );
}
