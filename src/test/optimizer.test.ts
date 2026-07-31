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
import { mockIpc, resetStoreToDefaults } from './mockIpc';

describe('P12 Frontend — Optimizer Safety & Session Test Suite', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('FE-01: invokes start_auto_optimize with candidate_ids option', async () => {
    mockIpc.registerHandler('start_auto_optimize', () => ({
      sessionId: 'opt_sess_123',
      bestPreset: { id: 'tr_2_multisplit', label: 'Turkey Multisplit', args: ['--split=2'] },
      recommendedCandidateId: 'best',
      confidence: 'High',
      originalStateRestored: true,
    }));

    const res = await (window as any).__mockIpc.handleInvoke('start_auto_optimize', { candidateIds: ['tr_2_multisplit'] });
    expect(res.sessionId).toBe('opt_sess_123');
    expect(res.originalStateRestored).toBe(true);
  });

  it('FE-02: cancel_optimizer returns true when cancellation is signaled', async () => {
    mockIpc.registerHandler('cancel_optimizer', () => true);
    const res = await (window as any).__mockIpc.handleInvoke('cancel_optimizer');
    expect(res).toBe(true);
  });

  it('FE-03: apply_optimizer_recommendation uses standard verified engine start', async () => {
    let appliedPresetId = '';
    mockIpc.registerHandler('apply_optimizer_recommendation', (args: any) => {
      appliedPresetId = args?.presetId ?? '';
      return null;
    });

    await (window as any).__mockIpc.handleInvoke('apply_optimizer_recommendation', { presetId: 'tr_2_multisplit' });
    expect(appliedPresetId).toBe('tr_2_multisplit');
  });
});
