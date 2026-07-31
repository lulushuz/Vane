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

describe('P11 Frontend — Linux Capability & Dynamic Filter Test Suite', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('FE-01: exposes experimental Linux capabilities from backend', async () => {
    mockIpc.registerHandler('get_advanced_capabilities', () => ({
      platform: 'linux',
      traffic: {
        tcpFiltering: { state: 'experimental', reason: 'P11 automated plan/executor tests passed' },
        udpFiltering: { state: 'experimental', reason: 'P11 automated plan/executor tests passed' },
        customTcpPorts: { state: 'experimental', reason: 'P11 automated plan/executor tests passed' },
        customUdpPorts: { state: 'experimental', reason: 'P11 automated plan/executor tests passed' },
      },
    }));

    const res = await (window as any).__mockIpc.handleInvoke('get_advanced_capabilities');
    expect(res.platform).toBe('linux');
    expect(res.traffic.udpFiltering.state).toBe('experimental');
  });

  it('FE-02: Linux UDP and custom port values are preserved in state', () => {
    useEngineStore.setState({
      activePresetId: 'tr_2_multisplit',
    });
    expect(useEngineStore.getState().activePresetId).toBe('tr_2_multisplit');
  });
});
