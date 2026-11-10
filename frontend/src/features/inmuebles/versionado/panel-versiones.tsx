/* [30AA-1] Pestañas de versionado dentro del modal de inmueble. Solo se
 * pinta al editar un inmueble guardado: las versiones necesitan su id.
 *
 * Público → v1 siempre. Interno → v2+ y el historial. La pestaña de la v2+
 * solo aparece si la configuración pide más de una versión. */
import { useState } from 'react';
import { AlertTriangle, History, Users } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { hayVersion, versionNumero, type VersionesInmueble } from '@/domain/versiones';
import { paresDeFotos, type ProgresoVariantes } from '@/hooks/inmuebles/use-versionado';
import type { EstadoVersion } from '@/domain/versiones';
import { PestanaFotos } from './pestana-fotos';
import { PestanaHistorial } from './pestana-historial';
import { PestanaTextos } from './pestana-textos';
import { SeccionFacebook } from './seccion-facebook';
import type { CuentaFacebook, VinculoFacebook } from '@/domain/versiones';

export type Seccion = 'v1' | `v${number}` | 'facebook' | 'historial';

interface Props {
  datos: VersionesInmueble | null;
  cuentas: CuentaFacebook[];
  vinculos: VinculoFacebook[];
  ocupado: boolean;
  aviso: string | null;
  error: string | null;
  progreso: ProgresoVariantes | null;
  /* Fotos del inmueble en orden, para emparejar con la variante de cada
   * versión (la misma foto puede tener v2 y v3 a la vez). */
  fotosParaVariante: string[];
  alGenerarTexto: (version: number, instruccion: string) => void;
  alGuardarTexto: (version: number, titulo: string, descripcion: string, estado?: EstadoVersion) => void;
  alBorrarTexto: (version: number) => void;
  alGenerarFotos: (version: number) => void;
  alBorrarFoto: (orden: number, version: number) => void;
  alVincular: (cuentaId: string, marketplaceId: string | null) => void;
  alDesvincular: (cuentaId: string) => void;
  alCrearCuenta: (nombre: string, url: string) => void;
}

/* Etiquetas de las pestañas: v1, v2…, Facebook e historial. */
function etiquetas(datos: VersionesInmueble): { id: Seccion; texto: string }[] {
  const lista: { id: Seccion; texto: string }[] = [{ id: 'v1', texto: 'Versión 1' }];
  for (let n = 2; n <= datos.configuradas; n++) {
    lista.push({ id: `v${n}`, texto: `Versión ${n}` });
  }
  lista.push({ id: 'facebook', texto: 'Facebook' }, { id: 'historial', texto: 'Historial' });
  return lista;
}

export function PanelVersiones(props: Props) {
  const [seccion, setSeccion] = useState<Seccion>('v1');
  const { datos } = props;

  if (!datos) {
    return <p className="text-sm text-muted-foreground">Cargando versiones…</p>;
  }

  const tabs = etiquetas(datos);
  const numero = seccion.startsWith('v') && seccion !== 'v1' ? Number(seccion.slice(1)) : null;
  const version = numero !== null && Number.isFinite(numero) ? numero : null;

  return (
    <section className="grid gap-3 rounded-md border p-3">
      <div role="tablist" aria-label="Versiones del inmueble" className="flex flex-wrap gap-1 border-b pb-2">
        {tabs.map((t) => {
          const activa = t.id === seccion;
          const vacia = t.id.startsWith('v') && t.id !== 'v1' && !hayVersion(datos, Number(t.id.slice(1)));
          return (
            <Button
              key={t.id}
              role="tab"
              aria-selected={activa}
              size="sm"
              variant={activa ? 'secondary' : 'ghost'}
              className={cn('gap-1', vacia && 'text-muted-foreground')}
              onClick={() => setSeccion(t.id)}
            >
              {t.id === 'facebook' && <Users className="h-3.5 w-3.5" />}
              {t.id === 'historial' && <History className="h-3.5 w-3.5" />}
              {t.texto}
              {t.id.startsWith('v') && t.id !== 'v1' && (vacia ? ' ·' : ' •')}
            </Button>
          );
        })}
      </div>

      {(props.aviso || props.error) && (
        <p
          role="alert"
          className={cn(
            'rounded-md border px-3 py-2 text-sm',
            props.error
              ? 'border-destructive/40 bg-destructive/10 text-destructive'
              : 'border-amber-500/40 bg-amber-500/10',
          )}
        >
          {props.error ?? props.aviso}
        </p>
      )}

      <div role="tabpanel">
        {seccion === 'v1' && (
          <PestanaTextos
            version={1}
            tituloV1={datos.titulo_v1}
            descripcionV1={datos.descripcion_v1}
            variante={null}
            ocupado={props.ocupado}
            alGenerar={props.alGenerarTexto}
            alGuardar={props.alGuardarTexto}
            alBorrar={props.alBorrarTexto}
          />
        )}

        {version !== null && (
          <div className="grid gap-4">
            <PestanaTextos
              version={version}
              tituloV1={datos.titulo_v1}
              descripcionV1={datos.descripcion_v1}
              variante={versionNumero(datos, version)}
              ocupado={props.ocupado}
              alGenerar={props.alGenerarTexto}
              alGuardar={props.alGuardarTexto}
              alBorrar={props.alBorrarTexto}
            />
            <PestanaFotos
              pares={paresDeFotos(props.fotosParaVariante, datos.fotos, version)}
              progreso={props.progreso}
              ocupado={props.ocupado}
              alGenerar={() => props.alGenerarFotos(version)}
              alBorrar={(orden) => props.alBorrarFoto(orden, version)}
            />
          </div>
        )}

        {seccion === 'facebook' && (
          <SeccionFacebook
            cuentas={props.cuentas}
            vinculos={props.vinculos}
            ocupado={props.ocupado}
            alVincular={props.alVincular}
            alDesvincular={props.alDesvincular}
            alCrearCuenta={props.alCrearCuenta}
          />
        )}

        {seccion === 'historial' && <PestanaHistorial entradas={datos.historial} />}
      </div>

      <NotaVersiones versiones={datos.configuradas} />
    </section>
  );
}

/* Ajuste global (cuántas versiones por inmueble). Vive aquí como nota
 * desplegable porque cambiarlo afecta a TODOS los inmuebles: se hace desde
 * la configuración general, no por inmueble. */
function NotaVersiones({ versiones }: { versiones: number }) {
  const [abierto, setAbierto] = useState(false);
  return (
    <div className="border-t pt-2 text-xs text-muted-foreground">
      <button
        type="button"
        className="flex items-center gap-1 underline-offset-4 hover:underline"
        onClick={() => setAbierto((a) => !a)}
      >
        <AlertTriangle className="h-3 w-3" /> Versiones configuradas: {versiones}
      </button>
      {abierto && (
        <p className="mt-1">
          Se cambia en Configuración → Versiones, y afecta a todos los inmuebles: la pestaña de cada versión se muestra
          u oculta según ese número.
        </p>
      )}
    </div>
  );
}
