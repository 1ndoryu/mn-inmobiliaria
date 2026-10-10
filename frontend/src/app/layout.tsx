import type { ReactNode } from 'react';
import { Building2, CalendarClock, ChevronsLeft, ChevronsRight, ImageIcon, Megaphone, MessageCircle, Monitor, Moon, Settings2, Sun, Users } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button, ButtonPlano } from '@/components/ui/button';
import { useTema } from '@/hooks/app/use-tema';
import { useAside } from '@/hooks/app/use-aside';
import type { Tema } from '@/app/tema';
import type { ConexionIA, ResumenConexionIA } from '@/domain/ia';
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
    <ButtonPlano
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
    </ButtonPlano>
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

/* Estado de la conexión de IA en la cabecera del admin [10AA-16]. Verde:
 * última prueba OK; ámbar: sin comprobar o deshabilitada; rojo: no conecta
 * o no se pudo leer. La fecha va en el texto para juzgar si la prueba es vieja. */
const COLOR_CONEXION: Record<ConexionIA, string> = {
  conectada: 'bg-emerald-500',
  'sin-comprobar': 'bg-amber-500',
  deshabilitada: 'bg-amber-500',
  fallo: 'bg-destructive',
  'sin-clave': 'bg-destructive',
  'sin-leer': 'bg-destructive',
  cargando: 'bg-muted-foreground/40',
};

function fechaIA(epoch: number | null): string {
  if (!epoch) return 'nunca';
  return new Date(epoch * 1000).toLocaleString('es', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
}

function textoIA(r: ResumenConexionIA): string {
  const proveedor = r.proveedor ?? 'IA';
  switch (r.conexion) {
    case 'cargando':
      return 'IA · cargando…';
    case 'sin-leer':
      return 'IA · no se pudo leer el estado';
    case 'sin-clave':
      return `${proveedor} · sin clave en el servidor`;
    case 'deshabilitada':
      return `${proveedor} · deshabilitada`;
    case 'sin-comprobar':
      return `${proveedor} · sin comprobar`;
    case 'fallo':
      return `${proveedor} · la última prueba falló (${fechaIA(r.comprobadoEn)})`;
    case 'conectada':
      return `${proveedor} · conectada, última prueba OK ${fechaIA(r.comprobadoEn)}`;
  }
}

/* `punto` deja solo el círculo de color (aside contraído y cabecera móvil);
 * el texto queda en `title` y `aria-label`. */
function IndicadorIA({ resumen, punto = false }: { resumen: ResumenConexionIA; punto?: boolean }) {
  const texto = textoIA(resumen);
  const color = <span aria-hidden className={cn('h-2.5 w-2.5 shrink-0 rounded-full', COLOR_CONEXION[resumen.conexion])} />;
  if (punto) {
    return (
      <span role="img" title={texto} aria-label={texto} className="flex h-8 w-8 items-center justify-center">
        {color}
      </span>
    );
  }
  return (
    <p className="flex items-center gap-1.5 text-xs leading-snug text-muted-foreground">
      {color}
      <span>{texto}</span>
    </p>
  );
}

/* Pestaña del menú inferior móvil (259A-2): icono + etiqueta, como las
 * apps nativas. Solo móvil (`md:hidden` en el `nav` padre); el `aside`
 * de escritorio no cambia. */
function BotonTab({
  icono,
  texto,
  activo = false,
  onClick,
}: {
  icono: ReactNode;
  texto: string;
  activo?: boolean;
  onClick: () => void;
}) {
  return (
    <ButtonPlano
      type="button"
      onClick={onClick}
      aria-current={activo ? 'page' : undefined}
      className={cn(
        'flex min-h-12 flex-col items-center justify-center gap-0.5 rounded-md py-1.5 text-[10px] font-medium',
        activo ? 'text-primary' : 'text-muted-foreground',
      )}
    >
      {icono}
      {texto}
    </ButtonPlano>
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
  iaResumen,
}: {
  children: ReactNode;
  vista?: VistaApp;
  alCambiarVista?: (vista: VistaApp) => void;
  alAbrirConfig?: () => void;
  temaExterno?: Tema;
  alCiclarTemaExterno?: () => void;
  iaResumen?: ResumenConexionIA;
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
        {!contraido && (
          <div className="mt-auto flex flex-col gap-1 px-1">
            {iaResumen && <IndicadorIA resumen={iaResumen} />}
            <p className="text-xs text-muted-foreground">Panel admin · API</p>
          </div>
        )}
        <div className={cn('flex items-center justify-between px-1 pt-1', contraido && 'mt-auto flex-col gap-1')}>
          {!contraido && <span className="text-xs text-muted-foreground">{TEXTO_TEMA[tema]}</span>}
          {contraido && iaResumen && <IndicadorIA resumen={iaResumen} punto />}
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
          {iaResumen && <IndicadorIA resumen={iaResumen} punto />}
          {alAbrirConfig && (
            <Button variant="ghost" size="icon" title="Configuración" aria-label="Abrir configuración" onClick={alAbrirConfig}>
              <Settings2 />
            </Button>
          )}
          <BotonTema tema={tema} alCambiar={ciclar} />
        </header>
        {/* Contenido con reserva inferior en móvil: la barra de
          pestañas es fija y si no, tapa el final de la vista. */}
        <main className="mx-auto w-full max-w-6xl p-4 pb-24 md:p-6 md:pb-6">
          {children}
        </main>
        {/* Menú inferior móvil estilo app (259A-2): reemplaza la fila de
          botones bajo la cabecera (no cabía: 4 botones + iconos en 360px
          desbordaban). Fijo abajo, solo móvil; respeta el área segura
          (`env(safe-area-inset-bottom)`) en iPhone con gestos. */}
        {alCambiarVista && (
          <nav
            aria-label="Navegación principal"
            className="fixed inset-x-0 bottom-0 z-40 border-t bg-background/95 backdrop-blur md:hidden"
          >
            <div className="grid grid-cols-4 gap-1 px-2 pt-2 pb-[calc(0.5rem+env(safe-area-inset-bottom))]">
              <BotonTab
                icono={<Building2 className="h-5 w-5" />}
                texto="Inmuebles"
                activo={(vista ?? 'inmuebles') === 'inmuebles'}
                onClick={() => alCambiarVista('inmuebles')}
              />
              <BotonTab
                icono={<ImageIcon className="h-5 w-5" />}
                texto="Imágenes"
                activo={vista === 'imagenes'}
                onClick={() => alCambiarVista('imagenes')}
              />
              <BotonTab
                icono={<MessageCircle className="h-5 w-5" />}
                texto="Mensajes"
                activo={vista === 'mensajes'}
                onClick={() => alCambiarVista('mensajes')}
              />
              <BotonTab
                icono={<Megaphone className="h-5 w-5" />}
                texto="Publicidad"
                activo={vista === 'publicidad'}
                onClick={() => alCambiarVista('publicidad')}
              />
            </div>
          </nav>
        )}
      </div>
    </div>
  );
}
