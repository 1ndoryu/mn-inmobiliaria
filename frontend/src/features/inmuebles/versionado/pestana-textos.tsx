/* [30AA-1] Texto de una versión: la v1 es de solo lectura (es la ficha
 * pública) y la v2+ se genera con IA, se edita a mano y se marca lista.
 * El botón «Guardar» solo aparece con cambios: editar una variante deja
 * entrada en el historial del servidor. */
import { useState } from 'react';
import { Check, RefreshCw, Save, Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import type { EstadoVersion, VersionTexto } from '@/domain/versiones';

interface Props {
  version: number;
  tituloV1: string;
  descripcionV1: string;
  variante: VersionTexto | null;
  ocupado: boolean;
  alGenerar: (version: number, instruccion: string) => void;
  alGuardar: (version: number, titulo: string, descripcion: string, estado?: EstadoVersion) => void;
  alBorrar: (version: number) => void;
}

export function PestanaTextos({
  version,
  tituloV1,
  descripcionV1,
  variante,
  ocupado,
  alGenerar,
  alGuardar,
  alBorrar,
}: Props) {
  if (version <= 1) {
    return (
      <div className="grid gap-3">
        <p className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
          Esta es la versión pública: la que ve cualquiera en la web. Las versiones 2 en adelante son para publicar en
          otra cuenta de Facebook y no se muestran aquí.
        </p>
        <CampoSoloLectura etiqueta="Título" valor={tituloV1} />
        <CampoSoloLectura etiqueta="Descripción" valor={descripcionV1} multilinea />
      </div>
    );
  }

  return (
    <EditorVariante
      /* La clave remonta el editor al cambiar de versión o al regenerarla:
       * sin ella el formulario conservaría lo que había escrito antes. */
      key={`${version}-${variante?.actualizada_en ?? 'vacia'}`}
      version={version}
      variante={variante}
      ocupado={ocupado}
      alGenerar={alGenerar}
      alGuardar={alGuardar}
      alBorrar={alBorrar}
    />
  );
}

function CampoSoloLectura({
  etiqueta,
  valor,
  multilinea = false,
}: {
  etiqueta: string;
  valor: string;
  multilinea?: boolean;
}) {
  return (
    <div className="grid gap-1">
      <span className="text-sm font-medium">{etiqueta}</span>
      {multilinea ? (
        <p className="whitespace-pre-wrap rounded-md border bg-background px-3 py-2 text-sm">{valor || '—'}</p>
      ) : (
        <p className="rounded-md border bg-background px-3 py-2 text-sm">{valor || '—'}</p>
      )}
    </div>
  );
}

function EditorVariante({
  version,
  variante,
  ocupado,
  alGenerar,
  alGuardar,
  alBorrar,
}: {
  version: number;
  variante: VersionTexto | null;
  ocupado: boolean;
  alGenerar: (version: number, instruccion: string) => void;
  alGuardar: (version: number, titulo: string, descripcion: string, estado?: EstadoVersion) => void;
  alBorrar: (version: number) => void;
}) {
  const [titulo, setTitulo] = useState(variante?.titulo ?? '');
  const [descripcion, setDescripcion] = useState(variante?.descripcion ?? '');
  const [instruccion, setInstruccion] = useState('');

  const cambios =
    variante !== null && (titulo.trim() !== variante.titulo || descripcion.trim() !== variante.descripcion);

  return (
    <div className="grid gap-3">
      <p className="rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
        La variante dice lo mismo que la ficha con otras palabras: no cambia habitaciones, precios ni servicios. El
        servidor rechaza cualquier número que no esté en la v1.
      </p>

      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" disabled={ocupado} onClick={() => alGenerar(version, instruccion)}>
          <RefreshCw className={ocupado ? 'animate-spin' : undefined} />
          {variante ? 'Regenerar con IA' : 'Generar con IA'}
        </Button>
        {variante && (
          <>
            <Badge variant={variante.estado === 'lista' ? 'default' : 'secondary'}>
              {variante.estado === 'lista' ? 'Lista' : 'Borrador'}
            </Badge>
            <span className="text-xs text-muted-foreground">{variante.modelo}</span>
            {variante.estado === 'borrador' && (
              <Button
                size="sm"
                variant="outline"
                disabled={ocupado}
                onClick={() => alGuardar(version, titulo, descripcion, 'lista')}
              >
                <Check /> Marcar lista
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="ml-auto text-destructive"
              disabled={ocupado}
              onClick={() => alBorrar(version)}
            >
              <Trash2 /> Borrar
            </Button>
          </>
        )}
      </div>

      <label className="grid gap-1">
        <span className="text-sm font-medium">Título</span>
        <Input value={titulo} onChange={(e) => setTitulo(e.target.value)} placeholder="Se rellena al generar" />
      </label>

      <label className="grid gap-1">
        <span className="text-sm font-medium">Descripción</span>
        <Textarea
          rows={6}
          value={descripcion}
          onChange={(e) => setDescripcion(e.target.value)}
          placeholder="Se rellena al generar"
        />
      </label>

      <label className="grid gap-1">
        <span className="text-sm font-medium">Instrucción para la IA (opcional)</span>
        <Input
          value={instruccion}
          onChange={(e) => setInstruccion(e.target.value)}
          placeholder="Ej. más formal, o empieza por la ubicación"
        />
      </label>

      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          disabled={ocupado || !cambios || !titulo.trim() || !descripcion.trim()}
          onClick={() => alGuardar(version, titulo, descripcion)}
        >
          <Save /> Guardar cambios
        </Button>
        {cambios && <span className="self-center text-xs text-muted-foreground">Sin guardar</span>}
      </div>
    </div>
  );
}
