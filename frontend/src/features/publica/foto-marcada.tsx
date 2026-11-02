import { useState, type ImgHTMLAttributes } from 'react';

/* Logo de la marca (solo el dibujo blanco), centrado abajo: el mismo que sale al
 * descargar la foto. Va en todas las fotos del lado público, a escala con el marco. */
const RUTA_MARCA = '/img/logo-mn-blanco.svg';
/* Logo a 7 % del ancho de la foto (igual que en `logo-descarga.ts`), no del marco. */
const FRACCION_LOGO = 0.07;

/* Ancho del logo en CSS: el 7 % del ancho que ocupa la foto pintada dentro del marco.
 * Con `cover` la foto se recorta y con `contain` sobra borde, así que medir contra el
 * marco hacía el logo más grande en unas fotos que en otras. `cqw`/`cqh` son % del
 * marco (que es contenedor, ver `containerType` abajo). */
function anchoLogoCss(proporcion: number, cubre: boolean): string {
  const ancho = cubre ? `max(100cqw, 100cqh * ${proporcion})` : `min(100cqw, 100cqh * ${proporcion})`;
  return `calc(${FRACCION_LOGO} * (${ancho}))`;
}

/* La foto llena el marco (`className` lo da el llamador: tamaño, margen, aspecto).
 * `fit` elige cómo encajar la foto en el marco, como `object-cover` / `object-contain`.
 * `marca={false}` deja la foto sin logo (la imagen publicitaria ya es el diseño final). */
export function FotoMarcada({ className = '', fit = 'cover', marca = true, onLoad, ...img }: ImgHTMLAttributes<HTMLImageElement> & { fit?: 'cover' | 'contain'; marca?: boolean }) {
  /* Proporción ancho/alto de la foto, cuando carga: hace falta para medir el logo. */
  const [proporcion, setProporcion] = useState<number | null>(null);
  const cubre = fit === 'cover';
  return (
    <div className={`relative overflow-hidden ${className}`} style={{ containerType: 'size' }}>
      <img
        {...img}
        onLoad={(e) => {
          const { naturalWidth: w, naturalHeight: h } = e.currentTarget;
          if (h > 0) setProporcion(w / h);
          onLoad?.(e);
        }}
        className={`block h-full w-full ${cubre ? 'object-cover' : 'object-contain'}`}
      />
      {marca && (
        <img
          src={RUTA_MARCA}
          alt=""
          aria-hidden="true"
          /* Hasta que carga la foto no se conoce su proporción: el 7 % del marco sirve de provisional. */
          style={{ width: proporcion ? anchoLogoCss(proporcion, cubre) : `${FRACCION_LOGO * 100}%` }}
          className="pointer-events-none absolute bottom-[3%] left-1/2 -translate-x-1/2"
        />
      )}
    </div>
  );
}
