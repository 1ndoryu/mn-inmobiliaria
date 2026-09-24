import { Suspense, lazy } from 'react';
import { PaginaPublica } from './features/publica/pagina-publica';
import { rutaActual } from './platform/ventana';

/* [249A-1] El panel va en chunk aparte (`React.lazy`): el visitante público
 * solo descarga la página pública + sus datos, sin el código de gestión. */
const AppAdmin = lazy(() =>
  import('./app/app-admin').then((m) => ({ default: m.AppAdmin })),
);

export default function App() {
  /* `/admin*` = gestión con sesión; el resto = web pública. */
  if (rutaActual().startsWith('/admin')) {
    return (
      <Suspense fallback={<p className="px-6 py-16 text-center text-sm">Cargando panel…</p>}>
        <AppAdmin />
      </Suspense>
    );
  }
  return <PaginaPublica />;
}
