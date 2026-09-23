import { useEffect, useState } from 'react';
import { Hand, Loader2, Settings2, WandSparkles } from 'lucide-react';
import type { ConfigMejora, ModoMejora } from '@/domain/foto-mejora';
import { leerSalud } from '@/data/mejora/cliente-mejora';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

const MODOS: { valor: ModoMejora; titulo: string; descripcion: string }[] = [
  {
    valor: 'manual',
    titulo: 'Manual',
    descripcion: 'Nada se procesa solo. Tú pulsas Reintentar en cada foto.',
  },
  {
    valor: 'automatico',
    titulo: 'Automático',
    descripcion: 'Al subir fotos se encolan solas, de una en una con pausas.',
  },
];

/* Modal de configuración de mejora automática (shadcn Dialog).
 * Intervalo, prompt y reintento manual por foto. */
export function ModalConfigMejora(props: {
  abierto: boolean;
  alCambiarAbierto: (abierto: boolean) => void;
  config: ConfigMejora;
  alGuardar: (config: ConfigMejora) => void;
}) {
  const { abierto, alCambiarAbierto, config, alGuardar } = props;
  const [form, setForm] = useState<ConfigMejora>(config);
  const [salud, setSalud] = useState<string | null>(null);
  const [probando, setProbando] = useState(false);

  useEffect(() => {
    if (abierto) {
      setForm({ ...config });
      setSalud(null);
    }
  }, [abierto, config]);

  const numero = (v: string, defecto: number): number => {
    const n = Number(v);
    return Number.isFinite(n) ? n : defecto;
  };

  async function probar() {
    setProbando(true);
    setSalud(null);
    const s = await leerSalud();
    setProbando(false);
    if (!s.ok) {
      setSalud(s.detalle ?? 'Backend no disponible.');
      return;
    }
    if (!s.listo) {
      setSalud('Backend sin credenciales de Gemini.');
      return;
    }
    const ritmo =
      typeof s.intervaloSeg === 'number'
        ? ` · ritmo cada ${s.intervaloSeg}s${typeof s.jitterPct === 'number' ? ` ±${s.jitterPct}%` : ''}${typeof s.maxPorDia === 'number' ? ` · tope ${s.maxPorDia}/día` : ''}`
        : '';
    setSalud(`Backend listo · cola ${s.cola} · hoy ${s.procesadosHoy}${ritmo}.`);
  }

  function guardar() {
    alGuardar({
      intervaloSeg: Math.min(3600, Math.max(30, Math.floor(form.intervaloSeg))),
      jitterPct: Math.min(50, Math.max(0, Math.floor(form.jitterPct))),
      maxPorDia: Math.min(200, Math.max(1, Math.floor(form.maxPorDia))),
      modo: form.modo,
      prompt: form.prompt.trim() ? form.prompt.trim().slice(0, 2000) : form.prompt,
    });
    alCambiarAbierto(false);
  }

  return (
    <Dialog open={abierto} onOpenChange={alCambiarAbierto}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings2 className="h-4 w-4" /> Mejora automática de fotos
          </DialogTitle>
          <DialogDescription>
            Cola secuencial con pausas para cuidar la cuenta. El original nunca se toca; la mejorada se guarda en la mejor resolución.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-2">
            {MODOS.map((m) => {
              const activo = form.modo === m.valor;
              const Icono = m.valor === 'manual' ? Hand : WandSparkles;
              return (
                <button
                  key={m.valor}
                  type="button"
                  onClick={() => setForm({ ...form, modo: m.valor })}
                  aria-pressed={activo}
                  className={cn(
                    'flex flex-col gap-1 rounded-lg border p-3 text-left transition-colors',
                    activo ? 'border-primary bg-primary/5' : 'hover:bg-muted',
                  )}
                >
                  <span className="flex items-center gap-1.5 text-sm font-semibold">
                    <Icono className="h-4 w-4" /> {m.titulo}
                    {activo && (
                      <span className="ml-auto rounded-full bg-primary px-2 py-0.5 text-[10px] font-medium text-primary-foreground">
                        Activo
                      </span>
                    )}
                  </span>
                  <span className="text-xs font-normal text-muted-foreground">{m.descripcion}</span>
                </button>
              );
            })}
          </div>
          <div className="grid grid-cols-3 gap-3">
            <label className="flex flex-col gap-1 text-sm font-medium">
              Cada (seg)
              <Input
                type="number"
                min={30}
                max={3600}
                value={form.intervaloSeg}
                onChange={(e) => setForm({ ...form, intervaloSeg: numero(e.target.value, form.intervaloSeg) })}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm font-medium">
              Jitter (%)
              <Input
                type="number"
                min={0}
                max={50}
                value={form.jitterPct}
                onChange={(e) => setForm({ ...form, jitterPct: numero(e.target.value, form.jitterPct) })}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm font-medium">
              Máx/día
              <Input
                type="number"
                min={1}
                max={200}
                value={form.maxPorDia}
                onChange={(e) => setForm({ ...form, maxPorDia: numero(e.target.value, form.maxPorDia) })}
              />
            </label>
          </div>

          <label className="flex flex-col gap-1 text-sm font-medium">
            Prompt de mejora
            <Textarea
              rows={5}
              value={form.prompt}
              onChange={(e) => setForm({ ...form, prompt: e.target.value })}
            />
          </label>

          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={probar} disabled={probando}>
              {probando ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : null} Probar conexión
            </Button>
            {salud && <span className="text-xs text-muted-foreground">{salud}</span>}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => alCambiarAbierto(false)}>
            Cancelar
          </Button>
          <Button onClick={guardar}>Guardar</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
