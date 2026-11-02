import type { ImgHTMLAttributes } from 'react';

/* Logo de la marca (solo el dibujo blanco), centrado abajo: el mismo que sale al
 * descargar la foto. Va en todas las fotos del lado público, a escala con el marco. */
const RUTA_MARCA = '/img/logo-mn-blanco.svg';

/* La foto llena el marco (`className` lo da el llamador: tamaño, margen, aspecto).
 * `fit` elige cómo encajar la foto en el marco, como `object-cover` / `object-contain`.
 * `marca={false}` deja la foto sin logo (la imagen publicitaria ya es el diseño final). */
export function FotoMarcada({ className = '', fit = 'cover', marca = true, ...img }: ImgHTMLAttributes<HTMLImageElement> & { fit?: 'cover' | 'contain'; marca?: boolean }) {
  return (
    <div className={`relative overflow-hidden ${className}`}>
      <img {...img} className={`block h-full w-full ${fit === 'contain' ? 'object-contain' : 'object-cover'}`} />
      {marca && (
        <img
          src={RUTA_MARCA}
          alt=""
          aria-hidden="true"
          className="pointer-events-none absolute bottom-[3%] left-1/2 w-[7%] -translate-x-1/2"
        />
      )}
    </div>
  );
}
