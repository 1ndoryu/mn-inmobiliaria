import type { ReactNode } from 'react';
import { Building2, CalendarClock, ChevronsLeft, ChevronsRight, ImageIcon, Megaphone, MessageCircle, Monitor, Moon, Settings2, Sun, Users } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useTema } from '@/hooks/app/use-tema';
import { useAside } from '@/hooks/app/use-aside';
import type { Tema } from '@/app/tema';
import { cn } from '@/lib/utils';

function ItemNav({
  icono,
  texto,
  activo = false,
  pronto = false,
  contraido = false,
  onClick,
}: {
  icono: ReactNode;
  texto: string;
  activo?: boolean;
  pronto?: boolean;
  contraido?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      disabled={pronto || !onClick}
      onClick={onClick}
      title={texto}
      aria-label={texto}
      className={cn(
        'flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm font-medium',
        contraido && 'justify-center px-2',
        activo ? 'bg-primary text-primary-foreground' : 'text-muted-foreground',
        pronto ? 'cursor-default opacity-60' : 'hover:bg-muted',
      )}
    >
      {icono}
      {!contraido && texto}
      {!contraido && pronto && (
        <Badge variant="secondary" className="ml-auto text-[10px]">
          Pronto
        </Badge>
      )}
    </button>
  );
}

const ICONO_TEMA = { sistema: Monitor, claro: Sun, oscuro: Moon } as const;
const TEXTO_TEMA: Record<Tema, string> = {
  sistema: 'Tema del sistema',
  claro: 'Tema claro',
  oscuro: 'Tema oscuro',
};

function BotonTema({ tema, alCambiar }: { tema: Tema; alCambiar: () => void }) {
  const Icono = ICONO_TEMA[tema];
  return (
    <Button
      variant="ghost"
      size="icon"
      title={`${TEXTO_TEMA[tema]} — clic para cambiar`}
      aria-label={`${TEXTO_TEMA[tema]} — clic para cambiar`}
      onClick={alCambiar}
    >
      <Icono />
    </Button>
  );
}

// Layout con menú lateral en escritorio y cabecera compacta en móvil.
// `mensajes` (169A-5) es la bandeja del chat con IA + su configuración.
// `publicidad` es la galería de imágenes para redes (plantilla Post I).
export type VistaApp = 'inmuebles' | 'imagenes' | 'mensajes' | 'publicidad';

export function Layout({
  children,
  vista,
  alCambiarVista,
  alAbrirConfig,
  temaExterno,
  alCiclarTemaExterno,
}: {
  children: ReactNode;
  vista?: VistaApp;
  alCambiarVista?: (vista: VistaApp) => void;
  alAbrirConfig?: () => void;
  temaExterno?: Tema;
  alCiclarTemaExterno?: () => void;
}) {
  const interno = useTema();
  const tema = temaExterno ?? interno.tema;
  const ciclar = alCiclarTemaExterno ?? interno.ciclar;
  /* Aside contraíble en escritorio: contraído solo deja iconos con tooltip.
   * El estado persiste (useAside). `overflow-x-hidden`: con `overflow-y`
   * en auto, el eje X computa a auto y los botones (36px) desbordaban la
   * caja contraída (32px con p-4), pintando scroll horizontal. */
  const { contraido, alternar } = useAside();
  const IconoContraer = contraido ? ChevronsRight : ChevronsLeft;
  return (
    <div className="min-h-dvh bg-background text-foreground md:flex">
      {/* Aside pegado a la pantalla con su propio desplazamiento: con muchos
        inmuebles el contenido ya no lo empuja fuera de la vista. */}
      <aside
        className={cn(
          'hidden shrink-0 flex-col gap-1 overflow-x-hidden border-r md:sticky md:top-0 md:flex md:h-dvh md:overflow-y-auto',
          contraido ? 'w-16 items-center p-2' : 'w-60 p-4',
        )}
      >
        <div className={cn('mb-4 flex items-center gap-2 px-1', contraido && 'flex-col')}>
          <span className="flex h-9 w-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Building2 className="h-5 w-5" />
          </span>
          {!contraido && <span className="text-base font-bold">Inmobiliaria</span>}
          <Button
            variant="ghost"
            size="icon"
            title={contraido ? 'Estirar menú' : 'Contraer menú'}
            aria-label={contraido ? 'Estirar menú' : 'Contraer menú'}
            onClick={alternar}
            className={cn(!contraido && 'ml-auto')}
          >
            <IconoContraer />
          </Button>
        </div>
        <nav className="flex w-full flex-col gap-1">
          <ItemNav
            icono={<Building2 className="h-4 w-4" />}
            texto="Inmuebles"
            activo={(vista ?? 'inmuebles') === 'inmuebles'}
            contraido={contraido}
            onClick={alCambiarVista ? () => alCambiarVista('inmuebles') : undefined}
          />
          <ItemNav
            icono={<ImageIcon className="h-4 w-4" />}
            texto="Imágenes"
            activo={vista === 'imagenes'}
            contraido={contraido}
            onClick={alCambiarVista ? () => alCambiarVista('imagenes') : undefined}
          />
          <ItemNav
            icono={<MessageCircle className="h-4 w-4" />}
            texto="Mensajes"
            activo={vista === 'mensajes'}
            contraido={contraido}
            onClick={alCambiarVista ? () => alCambiarVista('mensajes') : undefined}
          />
          <ItemNav
            icono={<Megaphone className="h-4 w-4" />}
            texto="Publicidad"
            activo={vista === 'publicidad'}
            contraido={contraido}
            onClick={alCambiarVista ? () => alCambiarVista('publicidad') : undefined}
          />
          <ItemNav icono={<Users className="h-4 w-4" />} texto="Clientes" pronto contraido={contraido} />
          <ItemNav icono={<CalendarClock className="h-4 w-4" />} texto="Visitas" pronto contraido={contraido} />
          {alAbrirConfig && (
            <ItemNav
              icono={<Settings2 className="h-4 w-4" />}
              texto="Configuración"
              contraido={contraido}
              onClick={alAbrirConfig}
            />
          )}
        </nav>
        {!contraido && <p className="mt-auto px-1 text-xs text-muted-foreground">Panel admin · API</p>}
        <div className={cn('flex items-center justify-between px-1 pt-1', contraido && 'mt-auto flex-col gap-1')}>
          {!contraido && <span className="text-xs text-muted-foreground">{TEXTO_TEMA[tema]}</span>}
          <BotonTema tema={tema} alCambiar={ciclar} />
        </div>
      </aside>

      <div className="min-w-0 flex-1">
        <header className="flex items-center gap-2 border-b px-4 py-3 md:hidden">
          <span className="flex h-8 w-8 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Building2 className="h-4 w-4" />
          </span>
          <span className="font-bold">Inmobiliaria</span>
          <Badge variant="secondary" className="ml-auto">
            {(vista ?? 'inmuebles') === 'imagenes' ? 'Imágenes' : vista === 'mensajes' ? 'Mensajes' : vista === 'publicidad' ? 'Publicidad' : 'Inmuebles'}
          </Badge>
          {alAbrirConfig && (
            <Button variant="ghost" size="icon" title="Configuración" aria-label="Abrir configuración" onClick={alAbrirConfig}>
              <Settings2 />
            </Button>
          )}
          <BotonTema tema={tema} alCambiar={ciclar} />
        </header>
        <main className="mx-auto w-full max-w-6xl p-4 md:p-6">
          {alCambiarVista && (
            <div className="mb-4 flex gap-2 md:hidden">
              <Button
                variant={(vista ?? 'inmuebles') === 'inmuebles' ? 'default' : 'outline'}
                size="sm"
                onClick={() => alCambiarVista('inmuebles')}
              >
                <Building2 className="h-3.5 w-3.5" /> Inmuebles
              </Button>
              <Button
                variant={vista === 'imagenes' ? 'default' : 'outline'}
                size="sm"
                onClick={() => alCambiarVista('imagenes')}
              >
                <ImageIcon className="h-3.5 w-3.5" /> Imágenes
              </Button>
              <Button
                variant={vista === 'mensajes' ? 'default' : 'outline'}
                size="sm"
                onClick={() => alCambiarVista('mensajes')}
              >
                <MessageCircle className="h-3.5 w-3.5" /> Mensajes
              </Button>
              <Button
                variant={vista === 'publicidad' ? 'default' : 'outline'}
                size="sm"
                onClick={() => alCambiarVista('publicidad')}
              >
                <Megaphone className="h-3.5 w-3.5" /> Publicidad
              </Button>
            </div>
          )}
          {children}
        </main>
      </div>
    </div>
  );
}
