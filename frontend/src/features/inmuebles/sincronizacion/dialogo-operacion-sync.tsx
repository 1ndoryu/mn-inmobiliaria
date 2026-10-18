/* [13AA-12] Diálogo de publicar o traer un inmueble que existe solo en un lado (D16).
 * Nombra el servidor antes de escribir. Cada apertura es un montaje nuevo (`key`), así el
 * estado de la anterior no se arrastra; el informe queda visible tras escribir. */
import { useEffect, useState } from 'react';
import { obtenerDestinoSync, publicarSync, traerSync, type DestinoSync, type InformeSync } from '@/data/inmuebles/sync';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';

export type DireccionSync = 'publicar' | 'traer';

export interface OperacionSync {
  slug: string;
  titulo: string;
  direccion: DireccionSync;
}

const TEXTOS: Record<DireccionSync, { boton: string; descripcion: string; etiquetaServidor: string }> = {
  publicar: {
    boton: 'Publicar en el servidor',
    descripcion: 'Se crea en el servidor una copia de este inmueble local.',
    etiquetaServidor: 'Servidor de destino',
  },
  traer: {
    boton: 'Traer al local',
    descripcion: 'Se crea en este entorno una copia del inmueble del servidor.',
    etiquetaServidor: 'Servidor de origen',
  },
};

function mensaje(e: unknown, respaldo: string): string {
  return e instanceof Error ? e.message : respaldo;
}

interface Props {
  operacion: OperacionSync | null;
  alCerrar: () => void;
}

export function DialogoOperacionSync({ operacion, alCerrar }: Props) {
  if (!operacion) return null;
  return <OperacionAbierta key={`${operacion.direccion}:${operacion.slug}`} operacion={operacion} alCerrar={alCerrar} />;
}

function OperacionAbierta({ operacion, alCerrar }: { operacion: OperacionSync; alCerrar: () => void }) {
  const [destino, setDestino] = useState<DestinoSync | null>(null);
  const [errorDestino, setErrorDestino] = useState<string | null>(null);
  const [ejecutando, setEjecutando] = useState(false);
  const [informe, setInforme] = useState<InformeSync | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Lee el host que se va a nombrar. Al desmontarse, las respuestas tardías se ignoran.
  useEffect(() => {
    let vigente = true;
    obtenerDestinoSync()
      .then((d) => {
        if (vigente) setDestino(d);
      })
      .catch((e: unknown) => {
        if (vigente) setErrorDestino(mensaje(e, 'No se pudo leer el servidor.'));
      });
    return () => {
      vigente = false;
    };
  }, []);

  async function ejecutar() {
    setEjecutando(true);
    setError(null);
    try {
      const resultado =
        operacion.direccion === 'publicar' ? await publicarSync(operacion.slug) : await traerSync(operacion.slug);
      setInforme(resultado);
    } catch (e) {
      setError(mensaje(e, 'La operación no se pudo completar.'));
    } finally {
      setEjecutando(false);
    }
  }

  const texto = TEXTOS[operacion.direccion];

  return (
    <Dialog open onOpenChange={(abierto) => !abierto && !ejecutando && alCerrar()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{texto.boton}</DialogTitle>
          <DialogDescription>{operacion.titulo || operacion.slug}</DialogDescription>
        </DialogHeader>
        {informe ? (
          <>
            <p className="text-sm font-medium">Lo escrito:</p>
            <ul className="list-disc space-y-1 pl-5 text-sm">
              {informe.hecho.map((paso) => (
                <li key={paso}>{paso}</li>
              ))}
            </ul>
            {informe.fallo && (
              <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                Quedó a medias: {informe.fallo}. Lo ya escrito no se deshace.
              </p>
            )}
            <DialogFooter>
              <Button type="button" onClick={alCerrar}>
                Cerrar
              </Button>
            </DialogFooter>
          </>
        ) : (
          <>
            <div className="space-y-2 text-sm">
              <p>{texto.descripcion}</p>
              <p>
                Se copian los campos, el copy, la receta, la ficha privada, las fotos y la visibilidad tal como están en el lado
                de origen. Si algo falla a mitad, lo ya escrito queda y se informa.
              </p>
              <p>
                {texto.etiquetaServidor}:{' '}
                <span className="font-mono font-medium">{destino?.host ?? (errorDestino ? 'no disponible' : 'consultando…')}</span>
              </p>
            </div>
            {errorDestino && (
              <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">{errorDestino}</p>
            )}
            {error && (
              <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={alCerrar} disabled={ejecutando}>
                Cancelar
              </Button>
              <Button type="button" onClick={() => void ejecutar()} disabled={!destino || ejecutando}>
                {ejecutando ? 'Escribiendo…' : texto.boton}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
