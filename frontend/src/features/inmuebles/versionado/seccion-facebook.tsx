/* [30AA-1] Cuentas de Facebook del inmueble: a cuál está vinculado y con qué
 * aviso de Marketplace en cada una. La cuenta principal es la que espeja
 * `inmuebles.marketplace_id`, así que su ID se sigue editando en la ficha.
 * [30AA-7] El vínculo es POR VERSIÓN: cada versión se publica como aviso
 * distinto y lleva su propio ID, aunque sea en la misma cuenta. */
import { useState } from 'react';
import { Check, Plus, Unlink } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { CuentaFacebook, VinculoFacebook } from '@/domain/versiones';
import { normalizarMarketplaceId, esMarketplaceIdValido } from '@/domain/inmueble';

interface Props {
  cuentas: CuentaFacebook[];
  vinculos: VinculoFacebook[];
  /** Versiones por inmueble (1..N): una fila de aviso por cada una. */
  configuradas: number;
  ocupado: boolean;
  alVincular: (cuentaId: string, version: number, marketplaceId: string | null) => void;
  alDesvincular: (cuentaId: string, version: number) => void;
  alCrearCuenta: (nombre: string, url: string) => void;
}

export function SeccionFacebook({
  cuentas,
  vinculos,
  configuradas,
  ocupado,
  alVincular,
  alDesvincular,
  alCrearCuenta,
}: Props) {
  const [nuevaNombre, setNuevaNombre] = useState('');
  const [nuevaUrl, setNuevaUrl] = useState('');
  const [creando, setCreando] = useState(false);
  const versiones = Array.from({ length: Math.max(1, configuradas) }, (_, i) => i + 1);

  function crear() {
    const nombre = nuevaNombre.trim();
    if (!nombre) return;
    alCrearCuenta(nombre, nuevaUrl.trim());
    setNuevaNombre('');
    setNuevaUrl('');
    setCreando(false);
  }

  return (
    <div className="grid gap-4">
      <p className="text-xs text-muted-foreground">
        Cada versión se publica como un aviso distinto: pon el ID de Marketplace de cada una en su cuenta. La cuenta
        principal es la que usa el ID de aviso de la ficha.
      </p>

      {vinculos.length === 0 && cuentas.length > 0 && (
        <p className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
          Este inmueble aún no está vinculado a ninguna cuenta.
        </p>
      )}

      <ul className="grid gap-2">
        {cuentas.map((cuenta) => {
          const deCuenta = vinculos.filter((v) => v.facebook_id === cuenta.id);
          return (
            <li key={cuenta.id} className="rounded-md border p-3">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{cuenta.nombre}</span>
                {cuenta.principal && <Badge variant="secondary">Principal</Badge>}
                {deCuenta.length > 0 && <Badge variant="outline">Vinculada</Badge>}
                <span className="ml-auto text-xs text-muted-foreground">{cuenta.inmuebles} inmuebles</span>
              </div>
              {cuenta.url && (
                <a
                  href={cuenta.url}
                  target="_blank"
                  rel="noreferrer"
                  className="text-xs text-primary underline-offset-4 hover:underline"
                >
                  {cuenta.url}
                </a>
              )}
              <div className="mt-2 grid gap-1">
                {versiones.map((version) => (
                  <CampoAviso
                    /* `key` con la versión: al cambiar el vínculo el campo se
                     * remonta con el valor del servidor. */
                    key={`${cuenta.id}-${version}`}
                    version={version}
                    vinculo={deCuenta.find((v) => v.version === version)}
                    ocupado={ocupado}
                    alVincular={(id) => alVincular(cuenta.id, version, id)}
                    alDesvincular={() => alDesvincular(cuenta.id, version)}
                  />
                ))}
              </div>
            </li>
          );
        })}
      </ul>

      {vinculos.length < cuentas.length * versiones.length && cuentas.length > 0 && (
        <p className="text-xs text-muted-foreground">
          Para vincular, escribe el ID del aviso de Marketplace de esa versión y pulsa «Vincular».
        </p>
      )}

      {creando ? (
        <div className="grid gap-2 rounded-md border p-3">
          <Input
            value={nuevaNombre}
            onChange={(e) => setNuevaNombre(e.target.value)}
            placeholder="Nombre de la cuenta (ej. Maye)"
          />
          <Input
            value={nuevaUrl}
            onChange={(e) => setNuevaUrl(e.target.value)}
            placeholder="https://facebook.com/…"
          />
          <div className="flex gap-2">
            <Button size="sm" onClick={crear} disabled={!nuevaNombre.trim() || ocupado}>
              <Check /> Crear cuenta
            </Button>
            <Button size="sm" variant="outline" onClick={() => setCreando(false)}>
              Cancelar
            </Button>
          </div>
        </div>
      ) : (
        <Button variant="outline" size="sm" className="w-fit" onClick={() => setCreando(true)}>
          <Plus /> Nueva cuenta de Facebook
        </Button>
      )}
    </div>
  );
}

/* Una versión concreta en una cuenta: sin vínculo pide el ID y vincula; con
 * vínculo enseña el ID y permite desvincular. */
function CampoAviso({
  version,
  vinculo,
  ocupado,
  alVincular,
  alDesvincular,
}: {
  version: number;
  vinculo: VinculoFacebook | undefined;
  ocupado: boolean;
  alVincular: (id: string | null) => void;
  alDesvincular: () => void;
}) {
  const [valor, setValor] = useState(vinculo?.marketplace_id ?? '');
  const valido = esMarketplaceIdValido(valor);
  const id = normalizarMarketplaceId(valor);
  const etiqueta = `v${version}`;

  if (vinculo) {
    return (
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span className="w-6 shrink-0 font-mono text-xs text-muted-foreground">{etiqueta}</span>
        <span className="text-muted-foreground">ID aviso:</span>
        <span className="font-mono">{vinculo.marketplace_id ?? '—'}</span>
        <Button size="sm" variant="ghost" disabled={ocupado} onClick={alDesvincular}>
          <Unlink /> Desvincular
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="w-6 shrink-0 font-mono text-xs text-muted-foreground">{etiqueta}</span>
      <Input
        value={valor}
        onChange={(e) => setValor(e.target.value)}
        placeholder="ID del aviso en esta cuenta"
        inputMode="numeric"
        className="h-8 max-w-56"
        aria-invalid={!valido}
        aria-label={`ID del aviso de la versión ${version}`}
      />
      {/* Sin ID también vale: se puede vincular y poner el aviso después. */}
      <Button size="sm" disabled={ocupado || !valido} onClick={() => alVincular(id)}>
        Vincular
      </Button>
      {!valido && valor.trim() !== '' && (
        <span className="text-xs text-destructive">ID inválido (5–32 dígitos).</span>
      )}
    </div>
  );
}
