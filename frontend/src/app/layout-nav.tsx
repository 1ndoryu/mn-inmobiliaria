import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { ButtonPlano } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export function ItemNav({
  icono,
  texto,
  activo = false,
  pronto = false,
  contraido = false,
  aviso = 0,
  onClick,
}: {
  icono: ReactNode;
  texto: string;
  activo?: boolean;
  pronto?: boolean;
  contraido?: boolean;
  /* Contador de avisos pendientes (solicitudes nuevas) [28AE-1] */
  aviso?: number;
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
      {!contraido && aviso > 0 && (
        <Badge className="ml-auto text-[10px]">{aviso}</Badge>
      )}
      {!contraido && pronto && (
        <Badge variant="secondary" className="ml-auto text-[10px]">
          Pronto
        </Badge>
      )}
    </ButtonPlano>
  );
}

/* Pestaña del menú inferior móvil (259A-2): icono + etiqueta, como las
 * apps nativas. Solo móvil (`md:hidden` en el `nav` padre); el `aside`
 * de escritorio no cambia. */
export function BotonTab({
  icono,
  texto,
  activo = false,
  aviso = 0,
  onClick,
}: {
  icono: ReactNode;
  texto: string;
  activo?: boolean;
  aviso?: number;
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
      <span className="relative">
        {icono}
        {aviso > 0 && (
          <span className="absolute -top-1 -right-2 min-w-4 rounded-full bg-primary px-1 text-[9px] leading-4 text-primary-foreground">
            {aviso}
          </span>
        )}
      </span>
      {texto}
    </ButtonPlano>
  );
}
