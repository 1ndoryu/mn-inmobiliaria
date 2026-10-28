/* [28AA-6] Vista del calendario de publicidad: cola del día («Publicar hoy»),
 * rejilla mensual y editor de cada publicación. */
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { crearPublicacion, proponerSiguiente, saltarPropuesta } from '@/data/publicaciones/api';
import type { Inmueble } from '@/domain/inmueble';
import type { Propuesta, Publicacion } from '@/domain/redes/publicacion-redes';
import { aIso, useCalendario } from '@/hooks/publicaciones/use-calendario';
import { DialogoPropuesta } from './dialogo-propuesta';
import { EditorPublicacion } from './editor-publicacion';
import { RejillaMes } from './rejilla-mes';

interface VistaCalendarioProps {
  inmuebles: Inmueble[];
}

export function VistaCalendario({ inmuebles }: VistaCalendarioProps) {
  const cal = useCalendario();
  const [propuesta, setPropuesta] = useState<Propuesta | null>(null);
  const [editando, setEditando] = useState<Publicacion | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);

  /* `undefined` = falló (ya mostrado por `operar`); `null` = la cola está vacía. */
  async function avanzar(excluir?: string) {
    const siguiente = await cal.operar(() => proponerSiguiente(excluir), 'No se pudo buscar el siguiente inmueble.');
    if (siguiente === undefined) return;
    setAviso(siguiente ? null : 'No queda ningún inmueble publicable.');
    setPropuesta(siguiente);
  }

  async function publicarPropuesta() {
    if (!propuesta) return;
    /* El borrador nace con la fecha de hoy: así sale en la rejilla del día. */
    const creada = await cal.operar(
      () => crearPublicacion(propuesta.inmuebleId, aIso(new Date())),
      'No se pudo crear la publicación.',
    );
    if (!creada) return;
    setPropuesta(null);
    setEditando(creada);
  }

  async function saltarActual() {
    if (!propuesta) return;
    const id = propuesta.inmuebleId;
    const saltada = await cal.operar(
      async () => {
        await saltarPropuesta(id);
        return true;
      },
      'No se pudo saltar el inmueble.',
    );
    /* Se excluye el saltado: la cola lo volvería a proponer. */
    if (saltada) await avanzar(id);
  }

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
        <Button onClick={() => void avanzar()} disabled={cal.cargando}>
          Publicar hoy
        </Button>
      </div>

      {cal.error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {cal.error}
        </p>
      )}
      {aviso && <p className="text-sm text-muted-foreground">{aviso}</p>}

      <RejillaMes mes={cal.mes} publicaciones={cal.publicaciones} hoy={hoy} alElegir={(p) => setEditando(p)} />

      {propuesta && (
        <DialogoPropuesta
          propuesta={propuesta}
          ocupado={cal.cargando}
          alPublicar={() => void publicarPropuesta()}
          alSaltar={() => void saltarActual()}
          alCerrar={() => setPropuesta(null)}
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
