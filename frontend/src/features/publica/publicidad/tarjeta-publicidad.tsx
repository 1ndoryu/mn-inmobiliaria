import { useState } from 'react';
import type { PublicidadPublicaItem } from '../../../domain/redes/publicacion-redes';
import { copiarTexto } from '../../../platform/documento';
import { descargarFotoConLogo } from '../../../platform/logo-descarga';
import { CLASE_BORDE } from '../disenno';
import { BOTON_PUBLICIDAD, CAJA_PUBLICIDAD } from './estilos-publicidad';
import { ImagenPublicidad } from './imagen-publicidad';

const etiquetaHash = (t: string): string => `#${t.replace(/^#/, '')}`;

/* Botón que copia un texto y lo confirma un momento. */
function BotonCopiar({ texto, etiqueta }: { texto: string; etiqueta: string }) {
  const [estado, setEstado] = useState<'reposo' | 'copiado' | 'fallo'>('reposo');
  const copiar = async () => {
    setEstado((await copiarTexto(texto)) ? 'copiado' : 'fallo');
  };
  return (
    <button type="button" className={BOTON_PUBLICIDAD} onClick={() => void copiar()}>
      {estado === 'copiado' ? 'Copiado' : estado === 'fallo' ? 'No se pudo copiar' : etiqueta}
    </button>
  );
}

/* Todas las fotos del inmueble; la descarga lleva el logo abajo (la web las muestra sin él). */
function Fotos({ fotos, titulo }: { fotos: string[]; titulo: string }) {
  const [fallo, setFallo] = useState(false);
  const bajar = async (url: string, i: number) => {
    setFallo(false);
    try {
      await descargarFotoConLogo(url, `${titulo.replace(/\s+/g, '-').toLowerCase()}-${i + 1}.jpg`);
    } catch {
      setFallo(true);
    }
  };
  return (
    <div className="space-y-2">
      <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
        {fotos.map((url, i) => (
          <div key={url} className={`border ${CLASE_BORDE}`}>
            <img src={url} alt={`${titulo}, foto ${i + 1}`} loading="lazy" className="aspect-[4/3] w-full object-cover" />
            <button type="button" className={`${BOTON_PUBLICIDAD} w-full border-0 border-t`} onClick={() => void bajar(url, i)}>
              Descargar con logo
            </button>
          </div>
        ))}
      </div>
      {fallo && <p>No se pudo descargar la foto.</p>}
    </div>
  );
}

/* Un inmueble con todo su material: fotos, imagen de publicidad, piezas de redes y texto. */
export function TarjetaPublicidad({ item, revision }: { item: PublicidadPublicaItem; revision: number }) {
  const etiquetas = item.tags.map(etiquetaHash).join(' ');
  const textoCompleto = [item.descripcion, etiquetas].filter(Boolean).join('\n\n');
  return (
    <article className={`${CAJA_PUBLICIDAD} space-y-6`}>
      <h2 className="text-xl font-bold">{item.titulo}</h2>
      <section className="space-y-2">
        <h3 className="font-bold">Fotos</h3>
        <Fotos fotos={item.fotos} titulo={item.titulo} />
      </section>
      <section className="space-y-2">
        <h3 className="font-bold">Imagen de publicidad</h3>
        <ImagenPublicidad slug={item.slug} revision={revision} />
      </section>
      {item.piezas.length > 0 && (
        <section className="space-y-2">
          <h3 className="font-bold">Piezas para redes</h3>
          <div className="flex flex-wrap gap-3">
            {item.piezas.map((p) => (
              <a key={p.id} href={p.url} download target="_blank" rel="noreferrer" className={`block w-40 border ${CLASE_BORDE}`}>
                <img src={p.url} alt={`Pieza ${p.formato}`} loading="lazy" className="w-full" />
                <span className="block px-2 py-1">Descargar ({p.formato})</span>
              </a>
            ))}
          </div>
        </section>
      )}
      {textoCompleto && (
        <section className="space-y-2">
          <h3 className="font-bold">Descripción y etiquetas</h3>
          {item.descripcion && <p className="whitespace-pre-line">{item.descripcion}</p>}
          {etiquetas && <p>{etiquetas}</p>}
          <BotonCopiar texto={textoCompleto} etiqueta="Copiar texto" />
        </section>
      )}
    </article>
  );
}
