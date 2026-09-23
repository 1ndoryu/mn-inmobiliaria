// Vista Mensajes (169A-5): bandeja del chat con IA + configuración.
// Pestañas internas para no ocupar dos huecos del menú.

import { useState } from 'react';
import { BandejaMensajes } from './bandeja-mensajes';
import { ConfigChat } from './config-chat';
import { Button } from '@/components/ui/button';

export function VistaMensajes() {
  const [pestana, setPestana] = useState<'bandeja' | 'config'>('bandeja');
  return (
    <div>
      <div className="mb-4 flex gap-2">
        <Button
          variant={pestana === 'bandeja' ? 'default' : 'outline'}
          size="sm"
          onClick={() => setPestana('bandeja')}
        >
          Bandeja
        </Button>
        <Button
          variant={pestana === 'config' ? 'default' : 'outline'}
          size="sm"
          onClick={() => setPestana('config')}
        >
          Configuración del chat
        </Button>
      </div>
      {pestana === 'bandeja' ? <BandejaMensajes /> : <ConfigChat />}
    </div>
  );
}
