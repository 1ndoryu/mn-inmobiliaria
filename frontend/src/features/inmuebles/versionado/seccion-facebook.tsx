/* [30AA-1] Cuentas de Facebook del inmueble: a cuál está vinculado y con qué
 * aviso de Marketplace en cada una. La cuenta principal es la que espeja
 * `inmuebles.marketplace_id`, así que su ID se sigue editando en la ficha.
 * [30AA-13] Cada cuenta publica UNA versión fija: la v1 en el Facebook de Maye
 * y la v2 en el de Wan (decisión de la usuaria, 2026-11-16). Aquí solo se
 * puede poner el ID de la versión de esa cuenta; lo pendiente por publicar se
 * mide contra ella.
 * [30AA-9] `compacto` lo usa el modal exclusivo de vínculos: misma conducta,
 * sin el párrafo de introducción y con las filas en una línea, para que las
 * cuentas entren sin scroll. */
import { useState } from 'react';
import { Check, Plus, Unlink } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { CuentaFacebook, VinculoFacebook } from '@/domain/versiones';
import { normalizarMarketplaceId, esMarketplaceIdValido } from '@/domain/inmueble';
import { AccionesCuenta } from './acciones-cuenta';

interface Props {
  cuentas: CuentaFacebook[];
  vinculos: VinculoFacebook[];
  /** Versiones que ESTE inmueble tiene (1 y las que se generaron): si la
   * versión de una cuenta no está aquí, esa cuenta no pide ID. */
  versiones: number[];
  ocupado: boolean;
  compacto?: boolean;
  alVincular: (cuentaId: string, version: number, marketplaceId: string | null) => void;
  alDesvincular: (cuentaId: string, version: number) => void;
  alCrearCuenta: (nombre: string, url: string) => void;
  /** [30AA-19] Renombrar una cuenta ya creada (el resto de sus datos no cambia). */
  alEditarCuenta: (cuenta: CuentaFacebook, nombre: string) => void;
  /** [30AA-19] Borrar una cuenta; sus vínculos se van en cascada. */
  alBorrarCuenta: (cuentaId: string) => void;
  /** [30AA-13] Reasignar la versión de una cuenta (v1 = Maye, v2 = Wan). */
  alAsignarVersion: (cuentaId: string, version: number | null) => void;
}

export function SeccionFacebook({
  cuentas,
  vinculos,
  versiones,
  ocupado,
  compacto = false,
  alVincular,
  alDesvincular,
  alCrearCuenta,
  alEditarCuenta,
  alBorrarCuenta,
  alAsignarVersion,
}: Props) {
  const [nuevaNombre, setNuevaNombre] = useState('');
  const [nuevaUrl, setNuevaUrl] = useState('');
  const [creando, setCreando] = useState(false);

  function crear() {
    const nombre = nuevaNombre.trim();
    if (!nombre) return;
    alCrearCuenta(nombre, nuevaUrl.trim());
    setNuevaNombre('');
    setNuevaUrl('');
    setCreando(false);
  }

  return (
    <div className={compacto ? 'grid min-w-0 gap-2' : 'grid min-w-0 gap-4'}>
      {!compacto && (
        <p className="text-xs text-muted-foreground">
          Cada cuenta publica una sola versión: la v1 en el Facebook de Maye y la v2 en el de Wan. Pon el ID de
          Marketplace de la versión de cada cuenta; lo que falte es lo que queda por publicar.
        </p>
      )}

      {vinculos.length === 0 && cuentas.length > 0 && (
        <p className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
          Este inmueble aún no está vinculado a ninguna cuenta.
        </p>
      )}

      {/* Sin ninguna cuenta no hay campos: se dice por qué antes del botón. */}
      {cuentas.length === 0 && (
        <p className="rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
          Todavía no hay cuentas de Facebook: crea la primera para poder vincular los avisos.
        </p>
      )}

      <ul className={compacto ? 'grid min-w-0 gap-1.5' : 'grid min-w-0 gap-2'}>
        {cuentas.map((cuenta) => {
          const deCuenta = vinculos.filter((v) => v.facebook_id === cuenta.id);
          const version = cuenta.version ?? 1;
          const existeVersion = versiones.includes(version);
          return (
            <li
              key={cuenta.id}
              className={compacto ? 'min-w-0 rounded-md border px-2.5 py-1.5' : 'min-w-0 rounded-md border p-3'}
            >
              <div className="flex min-w-0 flex-wrap items-center gap-1.5">
                <span className="font-medium">{cuenta.nombre}</span>
                <Badge variant="secondary">v{version}</Badge>
                {cuenta.principal && <Badge variant="outline">Principal</Badge>}
                {deCuenta.some((v) => v.version === version) && <Badge variant="outline">Vinculada</Badge>}
                <span className="ml-auto text-xs text-muted-foreground">{cuenta.inmuebles} inmuebles</span>
                {/* [30AA-19] Se puede renombrar y borrar cualquier cuenta: antes
                 * solo se creaban, y una cuenta de prueba se quedaba para siempre. */}
                <AccionesCuenta
                  cuenta={cuenta}
                  ocupado={ocupado}
                  alEditar={(nombre) => alEditarCuenta(cuenta, nombre)}
                  alBorrar={() => alBorrarCuenta(cuenta.id)}
                />
              </div>
              {cuenta.url && !compacto && (
                <a
                  href={cuenta.url}
                  target="_blank"
                  rel="noreferrer"
                  className="text-xs text-primary underline-offset-4 hover:underline"
                >
                  {cuenta.url}
                </a>
              )}
              <div className={compacto ? 'mt-1 grid min-w-0 gap-0.5' : 'mt-2 grid min-w-0 gap-1'}>
                {existeVersion ? (
                  <CampoAviso
                    key={`${cuenta.id}-${version}`}
                    version={version}
                    vinculo={deCuenta.find((v) => v.version === version)}
                    ocupado={ocupado}
                    compacto={compacto}
                    alVincular={(id) => alVincular(cuenta.id, version, id)}
                    alDesvincular={() => alDesvincular(cuenta.id, version)}
                  />
                ) : (
                  <p className="text-xs text-muted-foreground">
                    Sin versión {version} en este inmueble: no hay nada que publicar aquí.
                  </p>
                )}
              </div>
              {/* La versión de la cuenta se cambia aquí mismo: es el único
               * sitio donde se decide qué se publica dónde. */}
              <label className="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground">
                Publica la
                <VersionCuenta
                  cuenta={cuenta}
                  ocupado={ocupado}
                  alAsignar={(v) => alAsignarVersion(cuenta.id, v)}
                />
              </label>
            </li>
          );
        })}
      </ul>

      {creando ? (
        <div className={compacto ? 'grid gap-1.5 rounded-md border px-2.5 py-2' : 'grid gap-2 rounded-md border p-3'}>
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

/* Selector de la versión de una cuenta. `''` = sin asignar: esa cuenta deja
 * de pedir ID y no entra en el export. */
function VersionCuenta({
  cuenta,
  ocupado,
  alAsignar,
}: {
  cuenta: CuentaFacebook;
  ocupado: boolean;
  alAsignar: (version: number | null) => void;
}) {
  return (
    <select /* sentinel-disable html-nativo-en-vez-de-componente */
      className="h-7 rounded-md border border-input bg-background px-1 text-xs"
      value={cuenta.version ?? ''}
      disabled={ocupado}
      aria-label={`Versión que publica ${cuenta.nombre}`}
      onChange={(e) => alAsignar(e.target.value === '' ? null : Number(e.target.value))}
    >
      <option value="">— sin versión —</option>
      {[0, 1, 2, 3, 4, 5].map((n) => (
        <option key={n} value={n}>
          {n === 0 ? 'v0 · originales' : `v${n}`}
        </option>
      ))}
    </select>
  );
}

/* Una versión concreta en una cuenta: sin vínculo pide el ID y vincula; con
 * vínculo enseña el ID y permite desvincular. */
function CampoAviso({
  version,
  vinculo,
  ocupado,
  compacto,
  alVincular,
  alDesvincular,
}: {
  version: number;
  vinculo: VinculoFacebook | undefined;
  ocupado: boolean;
  compacto: boolean;
  alVincular: (id: string | null) => void;
  alDesvincular: () => void;
}) {
  const [valor, setValor] = useState(vinculo?.marketplace_id ?? '');
  const valido = esMarketplaceIdValido(valor);
  const id = normalizarMarketplaceId(valor);
  const etiqueta = `v${version}`;

  if (vinculo) {
    return (
      <div className={`flex items-center gap-2 text-sm ${compacto ? 'h-8' : 'flex-wrap'}`}>
        <span className="w-6 shrink-0 font-mono text-xs text-muted-foreground">{etiqueta}</span>
        <span className="shrink-0 text-muted-foreground">ID aviso:</span>
        <span className="min-w-0 flex-1 truncate font-mono">{vinculo.marketplace_id ?? '—'}</span>
        <Button size="sm" variant="ghost" disabled={ocupado} onClick={alDesvincular} title="Quitar esta versión de la cuenta">
          <Unlink />
          {!compacto && 'Desvincular'}
        </Button>
      </div>
    );
  }

  return (
    <div className={`flex items-center gap-2 ${compacto ? 'h-8' : 'flex-wrap'}`}>
      <span className="w-6 shrink-0 font-mono text-xs text-muted-foreground">{etiqueta}</span>
      <Input
        value={valor}
        onChange={(e) => setValor(e.target.value)}
        placeholder="ID del aviso en esta cuenta"
        inputMode="numeric"
        className="h-8 min-w-0 flex-1"
        aria-invalid={!valido}
        aria-label={`ID del aviso de la versión ${version}`}
      />
      {/* Sin ID también vale: se puede vincular y poner el aviso después. */}
      <Button size="sm" disabled={ocupado || !valido} onClick={() => alVincular(id)}>
        Vincular
      </Button>
      {!valido && valor.trim() !== '' && compacto && (
        <span className="shrink-0 text-xs text-destructive">5–32 dígitos</span>
      )}
      {!valido && valor.trim() !== '' && !compacto && (
        <span className="text-xs text-destructive">ID inválido (5–32 dígitos).</span>
      )}
    </div>
  );
}
