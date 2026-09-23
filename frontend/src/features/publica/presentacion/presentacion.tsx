import { useMaquinaEscribir } from '../../../hooks/publica/use-maquina-escribir';
import { AIRE_PRESENTACION_SENCILLA, ANCHO_PAGINA, ANCHO_TITULO_SENCILLO, CLASE_TINTA, CURSOR_MAQUINA, RELLENO_LATERAL_SITIO, SOLO_ESCRITORIO_ENLINEA, SOLO_MOVIL, TITULO_SENCILLO } from '../disenno';

/* Concepto 2 (activo): presentación sencilla, titular a la izquierda en
 * flujo normal y sin botones ni texto secundario. La palabra rotativa
 * lleva efecto máquina de escribir; el h1 conserva una etiqueta estable
 * para lectores de pantalla. La caja animada de 3 fases se conserva
 * aparte en `presentacion-caja.tsx` por si vuelve a hacer falta.
 * En móvil (<md) el titular va centrado y con la primera palabra fija:
 * sin máquina de escribir. */
export function Presentacion() {
  const palabra = useMaquinaEscribir();
  return (
    <section aria-label="Presentación" className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO} ${AIRE_PRESENTACION_SENCILLA}`}>
      <h1 aria-label="Encuentra el hogar de tus sueños en Puerto Ordaz" className={`${TITULO_SENCILLO} ${ANCHO_TITULO_SENCILLO} ${CLASE_TINTA}`}>
        <span aria-hidden="true" className={SOLO_MOVIL}>
          Encuentra el hogar
          <br />
          de tus sueños
          <br />
          en Puerto Ordaz
        </span>
        <span aria-hidden="true" className={SOLO_ESCRITORIO_ENLINEA}>
          Encuentra el{' '}
          {palabra}
          <span aria-hidden="true" className={CURSOR_MAQUINA} /> de tus
          <br />
          sueños en Puerto Ordaz
        </span>
      </h1>
    </section>
  );
}
