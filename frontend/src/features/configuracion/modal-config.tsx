import { useEffect, useState } from 'react';
import { Bot, Hand, ImageIcon, Loader2, Megaphone, Monitor, Moon, Settings2, Sun, WandSparkles } from 'lucide-react';
import type { Tema } from '@/app/tema';
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
import { leerSalud } from '@/data/mejora/cliente-mejora';
import { PestanaCopy } from './pestana-copy';
import { PestanaIA } from './pestana-ia';
import type { ConfigMejora, ModoMejora } from '@/domain/foto-mejora';
import type { ConfigCopy } from '@/domain/copy';
import { cn } from '@/lib/utils';

type Pestana = 'general' | 'mejora' | 'copy' | 'ia';

const PESTANAS: { valor: Pestana; texto: string; Icono?: typeof Sun }[] = [
  { valor: 'general', texto: 'General' },
  { valor: 'mejora', texto: 'Mejora', Icono: ImageIcon },
  { valor: 'copy', texto: 'Copy', Icono: Megaphone },
  { valor: 'ia', texto: 'IA', Icono: Bot },
];

const TEMAS: { valor: Tema; texto: string; Icono: typeof Sun }[] = [
  { valor: 'sistema', texto: 'Sistema', Icono: Monitor },
  { valor: 'claro', texto: 'Claro', Icono: Sun },
  { valor: 'oscuro', texto: 'Oscuro', Icono: Moon },
];

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

/* Modal Configuración centralizada: General (tema) + Mejora de fotos
 * (contenido migrado de `ModalConfigMejora`) + Copy para redes + IA de texto
 * (`PestanaIA`: proveedores, activo y pruebas del centro de IA del backend).
 * Sustituye al modal de mejora suelto; el aside y la vista Imágenes
 * abren este mismo modal. */
export function ModalConfig({
  abierto,
  alCambiarAbierto,
  tema,
  alCambiarTema,
  configMejora,
  alCambiarMejora,
  configCopy,
  alCambiarCopy,
}: {
  abierto: boolean;
  alCambiarAbierto: (abierto: boolean) => void;
  tema: Tema;
  alCambiarTema: (tema: Tema) => void;
  configMejora: ConfigMejora;
  alCambiarMejora: (c: ConfigMejora) => void;
  configCopy: ConfigCopy;
  alCambiarCopy: (c: ConfigCopy) => void;
}) {
  const [pestana, setPestana] = useState<Pestana>('general');
  const [formMejora, setFormMejora] = useState<ConfigMejora>(configMejora);
  const [formCopy, setFormCopy] = useState<ConfigCopy>(configCopy);
  const [salud, setSalud] = useState<string | null>(null);
  const [probando, setProbando] = useState(false);
  const [mensajeCopy, setMensajeCopy] = useState<string | null>(null);

  useEffect(() => {
    if (abierto) {
      setFormMejora({ ...configMejora });
      setFormCopy({ ...configCopy });
      setSalud(null);
      setMensajeCopy(null);
    }
  }, [abierto, configMejora, configCopy]);

  const numero = (v: string, defecto: number): number => {
    const n = Number(v);
    return Number.isFinite(n) ? n : defecto;
  };

  async function probar() {
    setProbando(true);
    setSalud(null);
    const s = await leerSalud();
    setProbando(false);
    setSalud(
      s.ok
        ? s.listo
          ? `Backend listo · cola ${s.cola} · hoy ${s.procesadosHoy}.`
          : 'Backend sin credenciales de Gemini.'
        : (s.detalle ?? 'Backend no disponible.'),
    );
  }

  function guardar() {
    const copyTrim: ConfigCopy = { ...formCopy, prompt: formCopy.prompt.trim(), cta: formCopy.cta.trim() };
    if (!copyTrim.prompt || !copyTrim.cta) {
      setPestana('copy');
      setMensajeCopy('El copy necesita instrucciones y llamado a la acción (no pueden quedar vacíos).');
      return;
    }
    alCambiarMejora({
      intervaloSeg: Math.min(3600, Math.max(30, Math.floor(formMejora.intervaloSeg))),
      jitterPct: Math.min(50, Math.max(0, Math.floor(formMejora.jitterPct))),
      maxPorDia: Math.min(200, Math.max(1, Math.floor(formMejora.maxPorDia))),
      modo: formMejora.modo,
      prompt: formMejora.prompt.trim() ? formMejora.prompt.trim().slice(0, 2000) : formMejora.prompt,
    });
    alCambiarCopy(copyTrim);
    alCambiarAbierto(false);
  }

  return (
    <Dialog open={abierto} onOpenChange={alCambiarAbierto}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings2 className="h-4 w-4" /> Configuración
          </DialogTitle>
          <DialogDescription>Aspecto, mejora de fotos, copy para redes e IA de texto.</DialogDescription>
        </DialogHeader>

        <div className="grid grid-cols-4 gap-1 rounded-lg border p-1">
          {PESTANAS.map(({ valor, texto, Icono }) => (
            <Button
              key={valor}
              variant={pestana === valor ? 'default' : 'ghost'}
              size="sm"
              onClick={() => setPestana(valor)}
            >
              {Icono && <Icono className="h-3.5 w-3.5" />} {texto}
            </Button>
          ))}
        </div>

        {pestana === 'general' && (
          <div className="grid gap-2">
            <span className="text-sm font-medium">Tema de la aplicación</span>
            <div className="grid grid-cols-3 gap-2">
              {TEMAS.map(({ valor, texto, Icono }) => (
                <Button
                  key={valor}
                  type="button"
                  variant={tema === valor ? 'default' : 'outline'}
                  className="h-auto flex-col gap-1 py-4"
                  onClick={() => alCambiarTema(valor)}
                >
                  <Icono className="h-4 w-4" /> {texto}
                </Button>
              ))}
            </div>
          </div>
        )}

        {pestana === 'mejora' && (
          <div className="flex flex-col gap-4">
            <div className="grid grid-cols-2 gap-2">
              {MODOS.map((m) => {
                const activo = formMejora.modo === m.valor;
                const Icono = m.valor === 'manual' ? Hand : WandSparkles;
                return (
                  <button
                    key={m.valor}
                    type="button"
                    onClick={() => setFormMejora({ ...formMejora, modo: m.valor })}
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
                  value={formMejora.intervaloSeg}
                  onChange={(e) => setFormMejora({ ...formMejora, intervaloSeg: numero(e.target.value, formMejora.intervaloSeg) })}
                />
              </label>
              <label className="flex flex-col gap-1 text-sm font-medium">
                Jitter (%)
                <Input
                  type="number"
                  min={0}
                  max={50}
                  value={formMejora.jitterPct}
                  onChange={(e) => setFormMejora({ ...formMejora, jitterPct: numero(e.target.value, formMejora.jitterPct) })}
                />
              </label>
              <label className="flex flex-col gap-1 text-sm font-medium">
                Máx/día
                <Input
                  type="number"
                  min={1}
                  max={200}
                  value={formMejora.maxPorDia}
                  onChange={(e) => setFormMejora({ ...formMejora, maxPorDia: numero(e.target.value, formMejora.maxPorDia) })}
                />
              </label>
            </div>
            <label className="flex flex-col gap-1 text-sm font-medium">
              Prompt de mejora
              <Textarea
                rows={5}
                value={formMejora.prompt}
                onChange={(e) => setFormMejora({ ...formMejora, prompt: e.target.value })}
              />
            </label>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={probar} disabled={probando}>
                {probando ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : null} Probar conexión
              </Button>
              {salud && <span className="text-xs text-muted-foreground">{salud}</span>}
            </div>
          </div>
        )}

        {pestana === 'copy' && (
          <PestanaCopy formCopy={formCopy} alCambiarCopy={setFormCopy} mensaje={mensajeCopy} />
        )}

        {pestana === 'ia' && <PestanaIA />}

        {/* El pie solo guarda en mejora/copy (estado local). General aplica
          al instante e IA guarda sola contra el backend (`PestanaIA`). */}
        <DialogFooter>
          <Button variant="ghost" onClick={() => alCambiarAbierto(false)}>
            Cerrar
          </Button>
          {(pestana === 'mejora' || pestana === 'copy') && <Button onClick={guardar}>Guardar cambios</Button>}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
