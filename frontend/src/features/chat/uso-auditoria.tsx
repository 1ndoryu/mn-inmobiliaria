// Uso y auditoría de la dueña (279A-2 F5): tokens por día×remitente
// (exactos del núcleo F0 en `ai`, estima en el resto) y últimas tomas
// humanas con contexto. Tablas simples, sin N+1 (una consulta cada una).

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useUsoAuditoria } from '@/hooks/chat/use-uso-auditoria';

export function UsoAuditoria() {
  const { uso, auditoria, error, cargando, recargar } = useUsoAuditoria();

  if (cargando) {
    return (
      <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
        Cargando uso y auditoría…
      </p>
    );
  }

  return (
    <div className="grid gap-4">
      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
      <section>
        <h3 className="mb-2 text-sm font-medium">Uso (7 días, por remitente)</h3>
        <div className="overflow-x-auto rounded-md border">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b bg-muted/50">
                <th className="px-2 py-1">Día</th>
                <th className="px-2 py-1">Remitente</th>
                <th className="px-2 py-1 text-right">Mensajes</th>
                <th className="px-2 py-1 text-right">Est.</th>
                <th className="px-2 py-1 text-right">In</th>
                <th className="px-2 py-1 text-right">Out</th>
              </tr>
            </thead>
            <tbody>
              {uso.map((u, i) => (
                <tr key={`${u.dia}-${u.remitente}-${i}`} className="border-b last:border-0">
                  <td className="px-2 py-1">{u.dia ?? '—'}</td>
                  <td className="px-2 py-1">{u.remitente ?? '—'}</td>
                  <td className="px-2 py-1 text-right">{u.mensajes ?? 0}</td>
                  <td className="px-2 py-1 text-right">{u.tokens_est ?? 0}</td>
                  <td className="px-2 py-1 text-right">{u.tokens_in ?? 0}</td>
                  <td className="px-2 py-1 text-right">{u.tokens_out ?? 0}</td>
                </tr>
              ))}
              {uso.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-2 py-4 text-center text-muted-foreground">
                    Sin uso registrado.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
      <section>
        <h3 className="mb-2 text-sm font-medium">Auditoría (últimas tomas humanas)</h3>
        <ul className="space-y-2">
          {auditoria.map((a) => (
            <li key={a.session_id} className="rounded-md border px-3 py-2 text-xs">
              <span className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{a.nombre || a.telefono || a.session_id.slice(0, 8)}</span>
                {a.estado_atencion && (
                  <Badge variant={a.estado_atencion === 'delegada' ? 'destructive' : 'secondary'} className="text-[10px]">
                    {a.estado_atencion}
                  </Badge>
                )}
                {a.ai_enabled === false && (
                  <Badge variant="outline" className="text-[10px]">
                    manual
                  </Badge>
                )}
                <span className="ml-auto text-muted-foreground">
                  IA {a.ia ?? 0} · humano {a.humano ?? 0} · visitante {a.visitante ?? 0}
                </span>
              </span>
              {a.extracto && <span className="mt-0.5 block truncate text-muted-foreground">{a.extracto}</span>}
            </li>
          ))}
          {auditoria.length === 0 && (
            <li className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
              Sin tomas humanas.
            </li>
          )}
        </ul>
      </section>
      <div>
        <Button
          variant="outline"
          size="sm"
          onClick={recargar}
        >
          Recargar
        </Button>
      </div>
    </div>
  );
}
