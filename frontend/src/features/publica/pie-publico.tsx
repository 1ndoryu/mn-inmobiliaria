import { ANCHO_PAGINA, CLASE_TINTA, RELLENO_LATERAL_SITIO, RELLENO_PIE, SEPARACION_PIE } from './disenno';

/* Pie público: solo la línea de derechos con año dinámico, centrada y
 * sin borde. La suscripción (cliente en `data/suscriptores`, hook
 * `useSuscripcion` y endpoint del backend) queda aparcada hasta que
 * vuelva al pie o al panel. */
export function PiePublico() {
  return (
    <footer className={`w-full ${SEPARACION_PIE}`}>
      <div className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO}`}>
        <p className={`text-center text-sm ${CLASE_TINTA} ${RELLENO_PIE}`}>
          © {new Date().getFullYear()} MN Inmobiliaria · Puerto Ordaz
        </p>
      </div>
    </footer>
  );
}
