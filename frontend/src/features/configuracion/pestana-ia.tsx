import { useEffect, useState } from 'react';
import { Bot, FlaskConical, Loader2, Power } from 'lucide-react';
import { useConfigIA } from '@/hooks/ia/use-config-ia';
import type { EstadoIA, ProveedorIA } from '@/domain/ia';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

/* Pestaña IA del modal Configuración [199A-2]: estado de los 2 proveedores
 * de texto, proveedor activo, habilitar/deshabilitar y probar cada uno.
 * Las claves viven en el `.env` del servidor: aquí solo se ve si hay clave
 * (`Configurado`/`Sin clave`), nunca su valor. */

interface Borrador {
  activo: ProveedorIA;
  hab: Record<ProveedorIA, boolean>;
}

function desdeEstado(e: EstadoIA): Borrador {
  const hab = { gloryapi: true, 'opencode-go': true } as Record<ProveedorIA, boolean>;
  for (const p of e.proveedores) {
    if (p.id === 'gloryapi' || p.id === 'opencode-go') hab[p.id] = p.habilitado;
  }
  return { activo: e.activo, hab };
}

function sucio(form: Borrador, e: EstadoIA): boolean {
  const base = desdeEstado(e);
  return form.activo !== base.activo || form.hab.gloryapi !== base.hab.gloryapi || form.hab['opencode-go'] !== base.hab['opencode-go'];
}

function fechaCorta(epoch: number | null): string {
  if (!epoch) return 'nunca';
  try {
    return new Date(epoch * 1000).toLocaleString('es', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
  } catch {
    return 'nunca';
  }
}

export function PestanaIA() {
  const { estado, cargando, guardando, probando, error, recargar, guardar, probar } = useConfigIA(true);
  const [form, setForm] = useState<Borrador | null>(null);

  useEffect(() => {
    if (estado && !form) setForm(desdeEstado(estado));
  }, [estado, form]);

  if (cargando || !estado) {
    return (
      <div className="flex flex-col items-start gap-2 text-sm text-muted-foreground">
        {cargando || !error ? (
          <span className="flex items-center gap-2">
            <Loader2 className="h-4 w-4 animate-spin" /> Cargando estado de la IA…
          </span>
        ) : (
          <>
            <span className="text-red-600">{error}</span>
            <Button variant="outline" size="sm" onClick={() => void recargar()}>
              Reintentar
            </Button>
          </>
        )}
      </div>
    );
  }
  const borrador = form ?? desdeEstado(estado);
  const modificado = form ? sucio(form, estado) : false;

  const poner = (parche: Partial<Borrador>) => setForm({ ...borrador, ...parche });
  const ponerHab = (id: ProveedorIA, v: boolean) => setForm({ ...borrador, hab: { ...borrador.hab, [id]: v } });

  async function guardarCambios() {
    await guardar(borrador.activo, borrador.hab.gloryapi, borrador.hab['opencode-go']);
    setForm(null);
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-muted-foreground">
        Redacción de fichas y copy van al proveedor activo; si falla, reintenta con el otro. Las claves están en el servidor.
      </p>
      {estado.proveedores.map((p) => {
        const id = p.id as ProveedorIA;
        const esBorradorActivo = borrador.activo === id;
        const habBorrador = borrador.hab[id] ?? true;
        return (
          <div key={p.id} className={cn('flex flex-col gap-2 rounded-lg border p-3', esBorradorActivo && 'border-primary')}>
            <div className="flex flex-wrap items-center gap-1.5">
              <Bot className="h-4 w-4" />
              <span className="text-sm font-semibold">{p.nombre}</span>
              {esBorradorActivo && <Badge>Activo</Badge>}
              <Badge variant={habBorrador ? 'secondary' : 'outline'}>{habBorrador ? 'Habilitado' : 'Deshabilitado'}</Badge>
              <Badge variant={p.configurado ? 'secondary' : 'destructive'}>{p.configurado ? 'Configurado' : 'Sin clave'}</Badge>
            </div>
            <p className="text-xs text-muted-foreground">Modelo: {p.modelo}</p>
            <p className="text-xs text-muted-foreground">
              {p.estado === 'ok'
                ? `Última prueba OK · ${p.latenciaMs ?? '?'} ms · ${p.ultimoModelo ?? p.modelo} · ${fechaCorta(p.comprobadoEn)}.`
                : p.estado === 'error'
                  ? `Última prueba falló · ${p.ultimoError ?? 'sin detalle'} · ${fechaCorta(p.comprobadoEn)}.`
                  : 'Aún sin probar desde aquí.'}
            </p>
            <div className="flex flex-wrap gap-2">
              {!esBorradorActivo && (
                <Button variant="outline" size="sm" onClick={() => poner({ activo: id })}>
                  Elegir activo
                </Button>
              )}
              <Button variant="outline" size="sm" onClick={() => ponerHab(id, !habBorrador)}>
                <Power className="h-3.5 w-3.5" /> {habBorrador ? 'Deshabilitar' : 'Habilitar'}
              </Button>
              <Button variant="ghost" size="sm" disabled={probando !== null} onClick={() => void probar(id)}>
                {probando === id ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <FlaskConical className="h-3.5 w-3.5" />} Probar
              </Button>
            </div>
          </div>
        );
      })}
      <div className="flex items-center gap-2">
        <Button size="sm" disabled={!modificado || guardando} onClick={guardarCambios}>
          {guardando ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : null} Guardar cambios
        </Button>
        {probando && <span className="text-xs text-muted-foreground">Probando {probando}…</span>}
        {error && <span className="text-xs text-red-600">{error}</span>}
      </div>
    </div>
  );
}
