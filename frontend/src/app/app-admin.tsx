import { useEffect, useMemo, useState } from 'react';
import { Link2, LogOut, Plus, Search, Settings2 } from 'lucide-react';
import { Layout, type VistaApp } from './layout';
import { ModalesApp } from './modales-app';
import { PantallaLogin } from '../features/sesion/pantalla-login';
import { TablaInmuebles } from '../features/inmuebles/tabla-inmuebles';
import { VistaImagenes } from '../features/imagenes/vista-imagenes';
import { VistaMensajes } from '../features/chat/vista-mensajes';
import { VistaPublicidad } from '../features/publicidad/vista-publicidad';
import { VistaSolicitudes } from '../features/solicitudes/vista-solicitudes';
import { useAnadirFotos } from '../hooks/inmuebles/use-anadir-fotos';
import { useBorrador } from '../hooks/inmuebles/use-borrador';
import { useConfigCopy } from '../hooks/copy/use-config-copy';
import { useCopy } from '../hooks/copy/use-copy';
import { useInmuebles } from '../hooks/inmuebles/use-inmuebles';
import { completarAlPublicar } from '../data/ia/completar-al-publicar';
import { useConfigIA } from '../hooks/ia/use-config-ia';
import { useSesion } from '../hooks/sesion/use-sesion';
import { useTema } from '../hooks/app/use-tema';
import { useFotosMejora } from '../hooks/mejora/use-fotos-mejora';
import { useColaMejora } from '../hooks/mejora/use-cola-mejora';
import { eliminarMejorada } from '../data/inmuebles/api';
import type { EstadoInmueble, Inmueble } from '../domain/inmueble';
import { resumirConexionIA } from '../domain/ia';
import { descripcionVista, guardarVista, leerVistaGuardada, tituloVista } from './vista-admin';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';

/* [249A-1] Panel de gestión en módulo aparte: `App.tsx` lo carga con
 * `React.lazy` solo bajo `/admin*`, así el visitante público no descarga el
 * código de gestión (tabla, mejora IA, publicidad en canvas). */
export function AppAdmin() {
  const { email, alEntrar, salir, autoEntrando } = useSesion();
  /* [03AA-2] Mientras la auto-entrada local intenta el login no se muestra
   * el formulario (evita el parpadeo login→dentro en cada recarga). */
  if (!email && autoEntrando) {
    return (
      <div className="flex min-h-dvh items-center justify-center bg-background p-4">
        <p className="text-sm text-muted-foreground">Entrando automáticamente…</p>
      </div>
    );
  }
  if (!email) return <PantallaLogin alEntrar={alEntrar} />;
  return <ContenidoApp key={email} email={email} alSalir={salir} />;
}

/* Contenido tras el login: se monta de nuevo al cambiar de usuario para
 * recargar la vista con su sesión. */
function ContenidoApp({ email, alSalir }: { email: string; alSalir: () => void }) {
  const { inmuebles, total, cargando, crear, eliminar, actualizar, publicar, reponer, aviso, error } = useInmuebles();
  const borrador = useBorrador();
  const [modalNuevo, setModalNuevo] = useState(false);
  const [vinculandoFb, setVinculandoFb] = useState(false);
  const [editando, setEditando] = useState<Inmueble | null>(null);
  const [viendo, setViendo] = useState<Inmueble | null>(null);
  const [busqueda, setBusqueda] = useState('');
  const [filtroEstado, setFiltroEstado] = useState<'todos' | EstadoInmueble>('todos');
  /* La pestaña sobrevive a recargas (ver vista-admin.ts). */
  const [vista, setVista] = useState<VistaApp>(leerVistaGuardada);
  useEffect(() => guardarVista(vista), [vista]);
  const [configAbierta, setConfigAbierta] = useState(false);
  /* Indicador de IA de la cabecera [10AA-16]: lee el estado al montar y lo
   * vuelve a leer al cerrar Configuración (mientras está abierta no recarga). */
  const ia = useConfigIA(!configAbierta);
  /* Momento de montaje: la caducidad de la última prueba se juzga contra él, sin llamar a Date.now() en cada render. */
  const [montadoEn] = useState(() => Date.now() / 1000);
  const [copyId, setCopyId] = useState<string | null>(null);
  const fotosNuevas = useAnadirFotos(inmuebles, actualizar);
  const temaApp = useTema();
  const { config: configCopy, guardar: guardarCopy } = useConfigCopy();
  /* Copy con IA: persiste en el propio inmueble (`copy`), sobrevive a
   * recargas y ediciones. */
  const copy = useCopy(actualizar);

  const { fotos, config, guardar, marcar, aviso: avisoFotos, borrarDeInmueble, historial, registrar, vaciarHistorial } =
    useFotosMejora(inmuebles);
  const cola = useColaMejora(fotos, config, marcar, registrar);

  const filtrados = useMemo(() => {
    const q = busqueda.trim().toLowerCase();
    return inmuebles.filter((i) => {
      if (filtroEstado !== 'todos' && i.estado !== filtroEstado) return false;
      if (!q) return true;
      return (
        i.titulo.toLowerCase().includes(q) ||
        i.ubicacion.toLowerCase().includes(q) ||
        i.descripcion.toLowerCase().includes(q)
      );
    });
  }, [inmuebles, busqueda, filtroEstado]);

  async function eliminarConFotos(id: string) {
    await eliminar(id);
    void borrarDeInmueble(id);
  }

  /* [259A-1] Restaura el original de una foto: borra su mejorada del
   * servidor, refresca la lista y limpia la copia local (queda pendiente
   * para mejorarla de nuevo; en modo automático se re-encola sola). Los
   * fallos se propagan a la tarjeta, que los muestra (nunca silenciosos). */
  async function restaurarMejorada(inmuebleId: string, orden: number) {
    const actualizado = await eliminarMejorada(inmuebleId, orden);
    reponer(actualizado);
    const entrada = fotos.find((f) => f.inmuebleId === inmuebleId && f.orden === orden);
    if (entrada) await marcar(entrada, { mejorada: null, estado: 'pendiente', error: null, intentos: 0 });
  }

  /* [18AA-7] Publicar/retirar desde la vista: antes de dejarlo visible, la IA
   * escribe lo que falte de la ficha (y el copy, si nunca se generó); después
   * se publica y se refresca el modal abierto. Si la IA no responde o no hay
   * nada que completar, se publica igual: la IA nunca bloquea la publicación. */
  async function publicarDesdeVista(id: string, publicado: boolean) {
    if (publicado) {
      const actual = inmuebles.find((i) => i.id === id);
      if (actual) {
        const completado = await completarAlPublicar(actual);
        const guardado = completado && (await actualizar(completado)) ? completado : actual;
        if (guardado !== actual) setViendo((v) => (v && v.id === id ? guardado : v));
        if (!guardado.copy) void copy.generar(guardado, configCopy);
      }
    }
    const ok = await publicar(id, publicado);
    if (ok) setViendo((v) => (v && v.id === id ? { ...v, publicado } : v));
  }

  /* Al subir un inmueble se genera su copy una vez (si está activado en
   * Configuración); los que ya existen lo generan a mano desde su acción. */
  async function crearConCopy(inmueble: Inmueble) {
    const ok = await crear(inmueble);
    if (ok && configCopy.generarAlCrear) {
      void copy.generar(inmueble, configCopy);
    }
    return ok;
  }

  /* Abre el modal Copy; si el inmueble aún no tiene textos, los genera. */
  function abrirCopy(inmueble: Inmueble) {
    setCopyId(inmueble.id);
    if (!inmueble.copy && !copy.generando[inmueble.id]) {
      void copy.generar(inmueble, configCopy);
    }
  }

  const inmuebleCopy = copyId ? (inmuebles.find((i) => i.id === copyId) ?? null) : null;

  return (
    <Layout
      vista={vista}
      alCambiarVista={setVista}
      alAbrirConfig={() => setConfigAbierta(true)}
      temaExterno={temaApp.tema}
      alCiclarTemaExterno={temaApp.ciclar}
      iaResumen={resumirConexionIA(ia.estado, ia.error, montadoEn)}
    >
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">{tituloVista(vista)}</h1>
          <p className="text-sm text-muted-foreground">{descripcionVista(vista, fotos.length, total)}</p>
        </div>
        <div className="ml-auto flex items-center gap-2">
          <span className="hidden text-xs text-muted-foreground sm:inline" title="Sesión iniciada">
            {email}
          </span>
          <Button variant="ghost" size="icon" title="Cerrar sesión" aria-label="Cerrar sesión" onClick={alSalir}>
            <LogOut className="h-4 w-4" />
          </Button>
          {vista === 'mensajes' || vista === 'publicidad' || vista === 'solicitudes' ? null : vista === 'imagenes' ? (
            <Button variant="outline" onClick={() => setConfigAbierta(true)}>
              <Settings2 /> Configurar mejora
            </Button>
          ) : (
            <>
              <Button variant="outline" onClick={() => setVinculandoFb(true)}>
                <Link2 /> Vincular con Facebook
              </Button>
              <Button className="ml-auto" onClick={() => setModalNuevo(true)}>
                <Plus /> Añadir inmueble
              </Button>
            </>
          )}
        </div>
      </div>

      {aviso && (
        <p className="mb-4 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">{aviso}</p>
      )}
      {error && (
        <p className="mb-4 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
      {avisoFotos && (
        <p className="mb-4 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">{avisoFotos}</p>
      )}
      {cola.aviso && (
        <p className="mb-4 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">{cola.aviso}</p>
      )}
      {fotosNuevas.notaFotos && (
        <p className="mb-4 rounded-md border border-sky-300 bg-sky-50 px-3 py-2 text-sm text-sky-900">
          {fotosNuevas.notaFotos}
        </p>
      )}

      {vista === 'mensajes' ? (
        <VistaMensajes />
      ) : vista === 'solicitudes' ? (
        <VistaSolicitudes />
      ) : vista === 'inmuebles' ? (
        <>
          <div className="mb-4 flex flex-col gap-2 sm:flex-row">
            <div className="relative flex-1">
              <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={busqueda}
                onChange={(e) => setBusqueda(e.target.value)}
                placeholder="Buscar por título, ubicación o descripción…"
                className="pl-9"
              />
            </div>
            <Select sinBase
              value={filtroEstado}
              onChange={(e) => setFiltroEstado(e.target.value as 'todos' | EstadoInmueble)}
              className="flex h-9 rounded-md border border-input bg-background px-3 py-1 text-sm capitalize focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
            >
              <option value="todos">Todos los estados</option>
              <option value="disponible">disponible</option>
              <option value="reservado">reservado</option>
              <option value="vendido">vendido</option>
              <option value="alquilado">alquilado</option>
            </Select>
          </div>

          {cargando ? (
            <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
              Cargando inmuebles…
            </p>
          ) : (
            <TablaInmuebles
              inmuebles={filtrados}
              onVer={setViendo}
              onEditar={setEditando}
              onEliminar={(id) => void eliminarConFotos(id)}
              onAnadirFotos={(id) => fotosNuevas.setAnadiendoId(id)}
              onCopy={abrirCopy}
              onPublicar={(id, publicado) => void publicarDesdeVista(id, publicado)}
              onActualizarInmueble={reponer}
            />
          )}
        </>
      ) : vista === 'publicidad' ? (
        <VistaPublicidad inmuebles={inmuebles} alEditarPropiedad={setEditando} alActualizarInmueble={reponer} />
      ) : (
        <VistaImagenes
          fotos={fotos}
          inmuebles={inmuebles}
          config={config}
          alCambiarConfig={guardar}
          cola={cola}
          historial={historial}
          alLimpiarHistorial={vaciarHistorial}
          alRestaurar={(id, orden) => restaurarMejorada(id, orden)}
        />
      )}

      <ModalesApp
        nuevos={fotosNuevas}
        inmuebles={inmuebles}
        fotos={fotos}
        vista={{
          viendo,
          alCerrar: () => setViendo(null),
          alPublicar: (id, publicado) => void publicarDesdeVista(id, publicado),
          alEditar: (i) => {
            setViendo(null);
            setEditando(i);
          },
        }}
        ajustes={{
          abierta: configAbierta,
          alCambiar: setConfigAbierta,
          tema: temaApp,
          mejora: config,
          alCambiarMejora: guardar,
          copia: configCopy,
          alCambiarCopia: guardarCopy,
        }}
        copia={{
          inmueble: inmuebleCopy,
          generando: copyId ? (copy.generando[copyId] ?? false) : false,
          error: copyId ? (copy.errores[copyId] ?? null) : null,
          alGenerar: () => {
            if (inmuebleCopy) void copy.generar(inmuebleCopy, configCopy);
          },
          alCerrar: () => setCopyId(null),
        }}
        edicion={{
          nuevoAbierto: modalNuevo,
          alCambiarNuevo: setModalNuevo,
          editando,
          alCerrarEdicion: () => setEditando(null),
          borrador,
          alCrear: crearConCopy,
          alActualizar: actualizar,
          errorGuardado: error,
        }}
        vinculacion={{
          abierto: vinculandoFb,
          alCambiar: setVinculandoFb,
          alVincular: (inmueble, avisoId) => actualizar({ ...inmueble, marketplaceId: avisoId }),
        }}
      />
    </Layout>
  );
}
