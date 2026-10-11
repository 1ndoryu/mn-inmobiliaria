import type { ConexionLab, ResumenLab } from '@/domain/lab';
import { cn } from '@/lib/utils';

/* Estado de la conexión del lab con el backend [10AA-23]. Verde: el lab
 * autenticó hace poco; ámbar: sin señal (lab apagado o aún no arrancó);
 * rojo: su último intento fue rechazado (token o máquina). Independiente del
 * punto de IA: aquél mide la prueba del proveedor, este el latido del lab. */
const COLOR_LAB: Record<ConexionLab, string> = {
  conectado: 'bg-emerald-500',
  'sin-senal': 'bg-amber-500',
  rechazado: 'bg-destructive',
  'sin-leer': 'bg-destructive',
  cargando: 'bg-muted-foreground/40',
};

function textoLab(r: ResumenLab): string {
  switch (r.conexion) {
    case 'cargando':
      return 'Lab · cargando…';
    case 'sin-leer':
      return 'Lab · no se pudo leer el estado';
    case 'conectado':
      return 'Lab · conectado con el backend';
    case 'sin-senal':
      return 'Lab · sin señal (¿arrancado?)';
    case 'rechazado':
      return 'Lab · el backend rechazó su token';
  }
}

/* `punto` deja solo el círculo (aside contraído y cabecera móvil); el texto
 * queda en `title` y `aria-label`, igual que `IndicadorIA`. */
export function IndicadorLab({ resumen, punto = false }: { resumen: ResumenLab; punto?: boolean }) {
  const texto = textoLab(resumen);
  const color = <span aria-hidden className={cn('h-2.5 w-2.5 shrink-0 rounded-full', COLOR_LAB[resumen.conexion])} />;
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
