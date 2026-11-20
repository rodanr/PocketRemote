// React glue for the connection: owns one RemoteConnection for the paired Mac, reconnects when
// the app comes to the foreground and stops it in the background so the Mac sees a clean disconnect.
import { useCallback, useEffect, useRef, useState } from 'react';
import { AppState } from 'react-native';
import { RemoteConnection, type ConnState } from './connection';
import type { Keypair } from './noise';
import type { Message } from './protocol';
import { secureRng } from './rng';
import type { Pairing } from './storage';

export function useRemote(pairing: Pairing | null, phone: Keypair | null) {
  const [state, setState] = useState<ConnState>({ phase: 'waiting', attempt: 0, needsRescan: false });
  const conn = useRef<RemoteConnection | null>(null);

  useEffect(() => {
    if (!pairing || !phone) return;
    const c = new RemoteConnection(
      {
        createSocket: (url) => new WebSocket(url) as unknown as ReturnType<NonNullable<ConstructorParameters<typeof RemoteConnection>[0]['createSocket']>>,
        rng: secureRng,
        timers: {
          setTimeout: (fn, ms) => setTimeout(fn, ms),
          clearTimeout: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
          now: () => Date.now(),
        },
      },
      { addr: pairing.addr, phone, hostKey: pairing.hostKey },
    );
    conn.current = c;
    const unsub = c.subscribe(setState);
    if (AppState.currentState === 'active') c.start();
    const sub = AppState.addEventListener('change', (s) => {
      if (s === 'active') {
        if (c.getState().phase === 'stopped' && c.getState().reason === undefined) c.start();
        else c.reconnectNow();
      } else {
        c.stop();
      }
    });
    return () => {
      sub.remove();
      unsub();
      c.stop();
      conn.current = null;
    };
  }, [pairing, phone]);

  const send = useCallback((m: Message) => conn.current?.send(m) ?? false, []);
  const reconnect = useCallback(() => conn.current?.reconnectNow(), []);
  return { state, send, reconnect };
}
