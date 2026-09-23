// Config del chat (169A-5): prompt extra, teléfonos, aviso WhatsApp,
// kill-switch global y tools deshabilitadas. Guarda solo lo cambiado.

import { useConfigChat } from '../../hooks/chat/use-config-chat';
import type { ClaveConfig } from '../../data/chat/cliente-admin';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';

const CAMPOS: { clave: ClaveConfig; etiqueta: string; ayuda: string; multilinea?: boolean }[] = [
  { clave: 'prompt_extra', etiqueta: 'Instrucciones extra para la IA', ayuda: 'Se añade al prompt inmobiliario en cada turno.', multilinea: true },
  { clave: 'contacto_telefono', etiqueta: 'Teléfono de contacto público', ayuda: 'Lo da la IA y sale en la ficha del chat.' },
  { clave: 'whatsapp_admin', etiqueta: 'WhatsApp que recibe los avisos', ayuda: 'Dígitos con prefijo, p. ej. +34600111222.' },
  {
    clave: 'ai_enabled_global',
    etiqueta: 'IA global (on|off)',
    ayuda: 'Kill-switch: en off ningún hilo responde con IA.',
  },
  {
    clave: 'tools_deshabilitadas',
    etiqueta: 'Tools deshabilitadas',
    ayuda: 'Nombres separados por comas, p. ej. escalar_a_humano.',
  },
];

export function ConfigChat() {
  const c = useConfigChat();
  /* Local para que TS estreche el nulo dentro de los callbacks. */
  const valores = c.valores;

  if (c.cargando || !valores) {
    return (
      <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
        Cargando configuración…
      </p>
    );
  }

  return (
    <div className="max-w-2xl space-y-4">
      {c.error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {c.error}
        </p>
      )}
      {c.aviso && (
        <p className="rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">{c.aviso}</p>
      )}
      {CAMPOS.map((campo) => (
        <div key={campo.clave}>
          <label htmlFor={`config-${campo.clave}`} className="mb-1 block text-sm font-medium">
            {campo.etiqueta}
          </label>
          {campo.multilinea ? (
            <Textarea
              id={`config-${campo.clave}`}
              value={valores[campo.clave] ?? ''}
              onChange={(e) => c.poner(campo.clave, e.target.value)}
              rows={4}
            />
          ) : (
            <Input
              id={`config-${campo.clave}`}
              value={valores[campo.clave] ?? ''}
              onChange={(e) => c.poner(campo.clave, e.target.value)}
            />
          )}
          <p className="mt-1 text-xs text-muted-foreground">{campo.ayuda}</p>
        </div>
      ))}
      <div className="flex gap-2">
        <Button onClick={() => void c.guardar()} disabled={c.guardando}>
          {c.guardando ? 'Guardando…' : 'Guardar cambios'}
        </Button>
        <Button variant="outline" onClick={() => void c.recargar()}>
          Recargar
        </Button>
      </div>
    </div>
  );
}
