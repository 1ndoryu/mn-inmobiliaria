import type { ConexionIA, ResumenConexionIA } from '@/domain/ia';
import { cn } from '@/lib/utils';

/* Estado de la conexión de IA en la cabecera del admin [10AA-16]. Verde:
 * última prueba OK reciente; ámbar: sin comprobar, deshabilitada o prueba
 * caducada; rojo: no conecta o no se pudo leer. La fecha va en el texto. */
const COLOR_CONEXION: Record<ConexionIA, string> = {
  conectada: 'bg-emerald-500',
  caducada: 'bg-amber-500',
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
    case 'caducada':
      return `${proveedor} · última prueba OK ${fechaIA(r.comprobadoEn)}, repetir Probar`;
    case 'fallo':
      return `${proveedor} · la última prueba falló (${fechaIA(r.comprobadoEn)})`;
    case 'conectada':
      return `${proveedor} · conectada, última prueba OK ${fechaIA(r.comprobadoEn)}`;
  }
}

/* `punto` deja solo el círculo de color (aside contraído y cabecera móvil);
 * el texto queda en `title` y `aria-label`. */
export function IndicadorIA({ resumen, punto = false }: { resumen: ResumenConexionIA; punto?: boolean }) {
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
