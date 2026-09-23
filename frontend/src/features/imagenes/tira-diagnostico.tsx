import { Activity, Check, Copy, FlaskConical, ListRestart } from 'lucide-react';
import type { FotoMejora } from '@/domain/foto-mejora';
import { useDiagnostico } from '@/hooks/mejora/use-diagnostico';
import { useTiraDiagnostico } from '@/hooks/mejora/use-tira-diagnostico';
import { Button } from '@/components/ui/button';

function fmtSeg(seg: number): string {
  if (seg < 60) return `${seg} s`;
  if (seg < 3600) return `${Math.round(seg / 60)} min`;
  return `${Math.round(seg / 3600)} h`;
}

/* Tira de diagnóstico (F18): estado real del backend —no el optimista del
 * frontal— con posición en cola, reintentos programados y último motivo.
 * "Copiar diagnóstico" deja un JSON listo para pegar al asistente.
 * F21: "Probar conexión" valida las cookies con un mensaje de texto (sin
 * gastar foto) y "Reiniciar cola" empieza de nuevo sin tocar lo lista.
 * F26: la sonda exige además estado AVAILABLE (el texto solo no basta: con
 * sesión degradada el texto pasa pero las imágenes se rechazan). */
export function TiraDiagnostico(props: { fotos: FotoMejora[]; alReiniciarTodo: () => Promise<number> }) {
  const { fotos, alReiniciarTodo } = props;
  const { estado, eventos, vivo, refrescar } = useDiagnostico();
  const { copiado, probando, prueba, reiniciando, copiarDiagnostico, probar, reiniciar } = useTiraDiagnostico(
    alReiniciarTodo,
    refrescar,
  );

  const resumenFotos = {
    pendiente: fotos.filter((f) => f.estado === 'pendiente').length,
    procesando: fotos.filter((f) => f.estado === 'procesando').length,
    lista: fotos.filter((f) => f.estado === 'lista').length,
    error: fotos.filter((f) => f.estado === 'error').length,
  };
  const ultimoFallo = [...eventos]
    .reverse()
    .find((e) => e.tipo === 'error-final' || e.tipo === 'reintento' || e.tipo === 'tope-diario' || e.tipo === 'sin-credenciales');

  /* "Copiar diagnóstico" deja un JSON listo para pegar al asistente. */
  const copiar = () =>
    void copiarDiagnostico({
      t: new Date().toISOString(),
      backendVivo: vivo,
      backend: estado,
      ultimosEventos: eventos.slice(-50),
      fotos: resumenFotos,
    });

  if (!vivo || !estado) {
    return (
      <div className="mb-4 flex flex-wrap items-center gap-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm">
        <Activity className="h-4 w-4 text-destructive" />
        <span>
          <strong>Backend no disponible.</strong> Arranca <code>npm run server:mejora</code> y las fotos pendientes se
          reanudarán solas.
        </span>
        <Button variant="ghost" size="sm" className="ml-auto h-7 px-2 text-xs" onClick={() => void copiar()}>
          {copiado ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />} Copiar diagnóstico
        </Button>
      </div>
    );
  }

  const enEspera = estado.cola.filter((t) => t.estado === 'encolado' && !t.motivo).length;
  const conReintento = estado.cola.filter((t) => t.estado === 'encolado' && t.motivo).length;

  return (
    <div className="mb-4 flex flex-wrap items-center gap-x-3 gap-y-1 rounded-md border px-3 py-2 text-sm text-muted-foreground">
      <span className="flex items-center gap-1.5">
        <span
          className={`inline-block h-2 w-2 rounded-full ${estado.listo ? 'bg-emerald-500' : 'bg-amber-500'}`}
          title={estado.listo ? 'Backend con credenciales' : (estado.detalle ?? 'Sin credenciales')}
        />
        <strong className="font-medium text-foreground">Backend {estado.listo ? 'listo' : 'sin credenciales'}</strong>
      </span>
      <span>
        Cola {estado.cola.length + (estado.activo ? 1 : 0)}
        {estado.activo ? ' · 1 en proceso' : ''}
        {enEspera > 0 ? ` · ${enEspera} esperando` : ''}
        {conReintento > 0 ? ` · ${conReintento} con reintento` : ''}
      </span>
      <span>
        Hoy {estado.procesadosHoy}/{estado.maxPorDia}
      </span>
      <span>Worker {estado.worker.vivo ? `vivo (${estado.worker.trabajos})` : 'parado'}</span>
      {estado.activo && <span title={estado.activo.fotoId}>En proceso hace {fmtSeg(estado.activo.encoladoHaceSeg)}</span>}
      {estado.detalle && <span className="text-amber-700">{estado.detalle}</span>}
      {ultimoFallo?.detalle && (
        <span className="max-w-full truncate" title={ultimoFallo.detalle}>
          Último fallo: {ultimoFallo.detalle.slice(0, 120)}
        </span>
      )}
      <span className="text-xs">
        Frontal: {resumenFotos.pendiente} pendientes · {resumenFotos.procesando} en seguimiento · {resumenFotos.lista}{' '}
        listas · {resumenFotos.error} en error
      </span>
      {prueba && (
        <span className="max-w-full truncate text-xs" title={prueba}>
          {prueba}
        </span>
      )}
      <span className="ml-auto flex gap-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs"
          disabled={probando}
          onClick={() => void probar()}
          title="Valida las cookies con login real y mensaje de texto, sin gastar foto (OK solo si sirven para imágenes)"
        >
          <FlaskConical className="h-3.5 w-3.5" /> {probando ? 'Probando…' : 'Probar conexión'}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs"
          disabled={reiniciando}
          onClick={() => void reiniciar()}
          title="Encolado, en proceso y error vuelven a empezar; lo lista no se toca"
        >
          <ListRestart className="h-3.5 w-3.5" /> {reiniciando ? 'Reiniciando…' : 'Reiniciar cola'}
        </Button>
        <Button variant="ghost" size="sm" className="h-7 px-2 text-xs" onClick={() => void copiar()}>
          {copiado ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}{' '}
          {copiado ? 'Copiado' : 'Copiar diagnóstico'}
        </Button>
      </span>
    </div>
  );
}
