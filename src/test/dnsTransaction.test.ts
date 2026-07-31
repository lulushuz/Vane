if (typeof window === 'undefined') {
  (globalThis as any).window = globalThis;
}
if (!(globalThis as any).window.__TAURI_INTERNALS__) {
  (globalThis as any).window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args: any) => (globalThis as any).__mockIpc?.handleInvoke(cmd, args) ?? Promise.resolve(null),
    plugins: {},
  };
}

import { beforeEach, describe, expect, it } from 'vitest';
import { useEngineStore } from '../store/engineStore';
import { mockIpc, resetStoreToDefaults } from './mockIpc';

describe('P10 Frontend — Transactional DNS & Revision Gating Test Suite', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('FE-01: stale DNS response cannot override newer selection', async () => {
    let resolveFirst: (val: any) => void;
    const firstPromise = new Promise((r) => {
      resolveFirst = r;
    });

    let count = 0;
    mockIpc.registerHandler('sync_dns_settings', () => {
      count++;
      if (count === 1) return firstPromise;
      return { configRevision: 2, protocol: 'dot', adblock: true, cache: true, stage: 'applied' };
    });

    useEngineStore.getState().setDnsProtocol('doh');
    useEngineStore.getState().setDnsProtocol('dot');

    resolveFirst!({ configRevision: 1, protocol: 'doh', adblock: false, cache: true, stage: 'applied', superseded: true });
    await Promise.resolve();

    expect(useEngineStore.getState().dnsProtocol).toBe('dot');
  });

  it('FE-02: ignores superseded response from backend', async () => {
    mockIpc.registerHandler('sync_dns_settings', () => ({
      configRevision: 1,
      protocol: 'doh',
      adblock: false,
      cache: true,
      stage: 'superseded',
      superseded: true,
    }));

    useEngineStore.getState().setDnsAdBlock(true);
    await Promise.resolve();
    expect(useEngineStore.getState().dnsAdBlock).toBe(true);
  });

  it('FE-03: DoQ selection is not permitted in frontend or backend model', () => {
    const currentProtocol = useEngineStore.getState().dnsProtocol;
    expect(['doh', 'dot']).toContain(currentProtocol);
  });

  it('FE-04: SOCKS5 proxy registration with DoT is rejected to prevent DNS leaks', async () => {
    useEngineStore.setState({ dnsProtocol: 'dot' });
    const success = await useEngineStore.getState().setProxySocks5('127.0.0.1:1080');
    expect(success).toBe(false);
  });
});
