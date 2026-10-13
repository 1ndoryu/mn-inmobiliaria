import { useCallback, useEffect, useMemo, useState } from 'react';
import { Check, Link2, Loader2 } from 'lucide-react';
import {
  cargarPerfilAvisos,
  type AvisoPendiente,
  type PerfilAvisos,
} from '@/data/inmuebles/perfil-marketplace';
import type { Inmueble } from '@/domain/inmueble';
import { formatearFecha, formatearPrecio, miniaturaDe, portadaDe } from '@/domain/inmueble';
import { Button } from '@/components/ui/button';
import { MiniaturaFoto } from '@/components/ui/miniatura-foto';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface Props {
  abierto: boolean;
  alCambiarAbierto: (abierto: boolean) => void;
  inmuebles: Inmueble[];
  /** Falso si el servidor rechazó el vínculo (el aviso de la página lo explica). */
  alVincular: (inmueble: Inmueble, avisoId: string) => Promise<boolean>;
}

/* Portada del inmueble en pequeño (la `min160-` si existe; si no, el original). */
function FotoInmueble({ inmueble, titulo, tamano }: { inmueble: Inmueble | undefined; titulo: string; tamano: string }) {
  const portada = inmueble ? portadaDe(inmueble) : '';
  return (
    <MiniaturaFoto
      key={portada}
      src={portada ? miniaturaDe(portada) : undefined}
      respaldo={portada || undefined}
      titulo={titulo}
      tamano={tamano}
    />
  );
}

/* [13AA-4] Vincula avisos del perfil de Facebook con sus inmuebles. Los
 * candidatos llegan ya ordenados (más parecido primero) y el primero queda
 * elegido, pero nada se vincula sin pulsar «Vincular». «No es este» oculta
 * el aviso solo mientras el modal esté abierto. */
export function ModalVincularFacebook({ abierto, alCambiarAbierto, inmuebles, alVincular }: Props) {
  const [perfil, setPerfil] = useState<PerfilAvisos | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [elegido, setElegido] = useState<Record<string, string>>({});
  const [ocultos, setOcultos] = useState<string[]>([]);
  const [vinculando, setVinculando] = useState<string | null>(null);
  const [fallo, setFallo] = useState<string | null>(null);
  const porId = useMemo(() => new Map(inmuebles.map((i) => [i.id, i])), [inmuebles]);

  const cargar = useCallback(
    () =>
      cargarPerfilAvisos()
        .then((p) => {
          setPerfil(p);
          setError(null);
        })
        .catch((e: unknown) => setError(e instanceof Error ? e.message : 'No se pudo leer el perfil.')),
    [],
  );

  useEffect(() => {
    if (abierto) void cargar();
  }, [abierto, cargar]);

  function cambiarAbierto(siguiente: boolean) {
    if (!siguiente) {
      setOcultos([]);
      setFallo(null);
    }
    alCambiarAbierto(siguiente);
  }

  function elegidoDe(aviso: AvisoPendiente): string | undefined {
    return elegido[aviso.aviso_id] ?? aviso.candidatos[0]?.inmueble_id;
  }

  async function vincular(aviso: AvisoPendiente) {
    const id = elegidoDe(aviso);
    const inmueble = id ? porId.get(id) : undefined;
    if (!inmueble) {
      setFallo('Ese inmueble ya no está en la lista. Recarga la página e inténtalo de nuevo.');
      return;
    }
    setFallo(null);
    setVinculando(aviso.aviso_id);
    const ok = await alVincular(inmueble, aviso.aviso_id);
    setVinculando(null);
    if (!ok) {
      setFallo('No se pudo vincular. Inténtalo de nuevo.');
      return;
    }
    await cargar();
  }

  const avisos = (perfil?.avisos ?? []).filter((a) => !ocultos.includes(a.aviso_id));

  return (
    <Dialog open={abierto} onOpenChange={cambiarAbierto}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Link2 className="h-4 w-4" /> Vincular con Facebook
          </DialogTitle>
          <DialogDescription>
            Avisos de tu perfil de Marketplace que todavía no tienen inmueble. Elige el que corresponde y pulsa «Vincular».
            {perfil && (
              <>
                {' '}
                Última lectura: {perfil.ultima_lectura ? formatearFecha(perfil.ultima_lectura) : 'todavía no hay'}.
              </>
            )}
          </DialogDescription>
        </DialogHeader>

        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : !perfil ? (
          <p className="text-sm text-muted-foreground">Cargando…</p>
        ) : avisos.length === 0 ? (
          <p className="rounded-lg border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
            No hay avisos pendientes de vincular. La lectura del perfil se hace sola una vez al día desde el laboratorio.
          </p>
        ) : (
          <div className="space-y-3">
            {avisos.map((aviso) => {
              const marcado = elegidoDe(aviso);
              const ocupado = vinculando !== null;
              return (
                <section key={aviso.aviso_id} className="rounded-lg border p-3">
                  <div className="flex items-start gap-3">
                    <MiniaturaFoto
                      key={aviso.imagen_url ?? aviso.aviso_id}
                      src={aviso.imagen_url ?? undefined}
                      titulo={aviso.titulo}
                      tamano="h-20 w-20"
                      sinReferrer
                    />
                    <div className="min-w-0">
                      <p className="font-medium">{aviso.titulo}</p>
                      <p className="text-xs text-muted-foreground">
                        {aviso.precio ? `Precio en Facebook: ${aviso.precio} · ` : ''}
                        <a href={aviso.url} target="_blank" rel="noreferrer" className="underline">
                          Ver en Facebook
                        </a>
                      </p>
                    </div>
                  </div>

                  {aviso.candidatos.length === 0 ? (
                    <p className="mt-2 text-sm text-muted-foreground">
                      No hay ningún inmueble parecido. Revisa el título del inmueble o créalo primero.
                    </p>
                  ) : (
                    <ul className="mt-2 space-y-1">
                      {aviso.candidatos.map((c) => {
                        const elegidoAhora = marcado === c.inmueble_id;
                        return (
                          <li key={c.inmueble_id}>
                            <button
                              type="button"
                              aria-pressed={elegidoAhora}
                              disabled={ocupado}
                              onClick={() => setElegido((e) => ({ ...e, [aviso.aviso_id]: c.inmueble_id }))}
                              className={`flex w-full items-center justify-between gap-2 rounded-md border px-2 py-1.5 text-left text-sm transition disabled:opacity-60 ${
                                elegidoAhora ? 'border-primary bg-primary/5' : 'hover:bg-muted/50'
                              }`}
                            >
                              <FotoInmueble inmueble={porId.get(c.inmueble_id)} titulo={c.titulo} tamano="h-10 w-14" />
                              <span className="min-w-0 flex-1 truncate">{c.titulo}</span>
                              <span className="shrink-0 text-xs text-muted-foreground">
                                {c.directo ? 'Muy parecido' : 'Parecido'} · {formatearPrecio(c.precio)}
                              </span>
                              {elegidoAhora && <Check className="h-4 w-4 shrink-0 text-primary" />}
                            </button>
                          </li>
                        );
                      })}
                    </ul>
                  )}

                  <div className="mt-3 flex justify-end gap-2">
                    <Button
                      variant="ghost"
                      disabled={ocupado}
                      onClick={() => setOcultos((o) => [...o, aviso.aviso_id])}
                    >
                      No es este
                    </Button>
                    <Button disabled={!marcado || ocupado} onClick={() => void vincular(aviso)}>
                      {vinculando === aviso.aviso_id && <Loader2 className="h-4 w-4 animate-spin" />}
                      Vincular
                    </Button>
                  </div>
                </section>
              );
            })}
          </div>
        )}

        {perfil && perfil.inmuebles_sin_aviso.length > 0 && (
          <details className="rounded-lg border px-3 py-2 text-sm">
            <summary className="cursor-pointer text-muted-foreground">
              Inmuebles sin aviso vinculado ({perfil.inmuebles_sin_aviso.length})
            </summary>
            <ul className="mt-2 space-y-1">
              {perfil.inmuebles_sin_aviso.map((i) => (
                <li key={i.inmueble_id} className="flex items-center gap-2">
                  <FotoInmueble inmueble={porId.get(i.inmueble_id)} titulo={i.titulo} tamano="h-8 w-11" />
                  <span className="min-w-0 flex-1 truncate">{i.titulo}</span>
                  <span className="shrink-0 text-xs text-muted-foreground">{formatearPrecio(i.precio)}</span>
                </li>
              ))}
            </ul>
          </details>
        )}

        {fallo ? <p className="text-xs text-destructive">{fallo}</p> : null}
      </DialogContent>
    </Dialog>
  );
}
