import { describe, expect, it, beforeEach } from 'vitest';
import { mockIpc, resetStoreToDefaults } from './mockIpc';
import { useEngineStore } from '../store/engineStore';
import { migratePersistedEngineState } from '../store/persistence';

describe('Test Group D — Store Persistence Queue Characterization', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('D-01: executes store write operations in sequential queue order', async () => {
    const writtenKeys: string[] = [];
    mockIpc.registerHandler('settings_set', (payload) => {
      writtenKeys.push(payload?.key);
      return {};
    });

    const storage = (useEngineStore.persist as any).getOptions().storage;
    await Promise.all([
      storage.setItem('key1', 'val1'),
      storage.setItem('key2', 'val2'),
      storage.setItem('key3', 'val3'),
    ]);

    expect(writtenKeys).toEqual(['key1', 'key2', 'key3']);
  });

  it('D-02: keeps persistence queue running even after a write rejection', async () => {
    let callCount = 0;
    mockIpc.registerHandler('settings_set', () => {
      callCount++;
      if (callCount === 1) throw new Error('Write failed');
      return {};
    });

    const storage = (useEngineStore.persist as any).getOptions().storage;
    await expect(storage.setItem('failKey', 'val')).rejects.toThrow('Write failed');
    await expect(storage.setItem('successKey', 'val')).resolves.not.toThrow();

    expect(callCount).toBe(2);
  });

  it('D-03: delays getItem calls until pending store writes complete', async () => {
    const executionOrder: string[] = [];
    mockIpc.registerHandler('settings_set', async () => {
      await new Promise((r) => setTimeout(r, 20));
      executionOrder.push('write');
    });
    mockIpc.registerHandler('settings_get', async () => {
      executionOrder.push('read');
      return JSON.stringify('readVal');
    });

    const storage = (useEngineStore.persist as any).getOptions().storage;
    const writePromise = storage.setItem('key', 'val');
    const readPromise = storage.getItem('key');

    await Promise.all([writePromise, readPromise]);
    expect(executionOrder).toEqual(['write', 'read']);
  });

  it('D-04: enqueues removeItem calls behind pending writes', async () => {
    const ops: string[] = [];
    mockIpc.registerHandler('settings_set', async () => {
      ops.push('set');
    });
    mockIpc.registerHandler('settings_remove', async () => {
      ops.push('remove');
    });

    const storage = (useEngineStore.persist as any).getOptions().storage;
    await Promise.all([
      storage.setItem('k', 'v'),
      storage.removeItem('k'),
    ]);

    expect(ops).toEqual(['set', 'remove']);
  });

  it('D-05: preserves write call order across 20 rapid setItem invocations', async () => {
    const setKeys: string[] = [];
    mockIpc.registerHandler('settings_set', (payload) => {
      setKeys.push(payload?.key);
      return {};
    });

    const storage = (useEngineStore.persist as any).getOptions().storage;
    const promises = Array.from({ length: 20 }, (_, i) => storage.setItem(`k_${i}`, `v_${i}`));
    await Promise.all(promises);

    const expectedKeys = Array.from({ length: 20 }, (_, i) => `k_${i}`);
    expect(setKeys).toEqual(expectedKeys);
  });

  it('D-06: prevents permanent queue deadlock when writes throw non-Error rejections', async () => {
    mockIpc.registerHandler('settings_set', () => {
      throw 'string rejection';
    });

    const storage = (useEngineStore.persist as any).getOptions().storage;
    await expect(storage.setItem('k1', 'v1')).rejects.toBe('string rejection');

    mockIpc.registerHandler('settings_set', () => ({}));
    await expect(storage.setItem('k2', 'v2')).resolves.not.toThrow();
  });
});

describe('Test Group K — Hydration and Persisted State Characterization', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('K-01: returns default initial state when no persisted settings exist', () => {
    const state = useEngineStore.getState();
    expect(state.activePresetId).toBe('default');
    expect(state.bypassMode).toBe('all');
    expect(state.dnsProtocol).toBe('doh');
    expect(state.killSwitch).toBe(false);
  });

  it('K-02: restores valid persisted state correctly', () => {
    const migrated = migratePersistedEngineState({
      activePresetId: 'preset-1',
      bypassMode: 'whitelist',
      whitelistDomains: ['example.com'],
      dnsProtocol: 'dot',
      killSwitch: true,
    });
    expect(migrated).toMatchObject({
      activePresetId: 'preset-1',
      bypassMode: 'whitelist',
      whitelistDomains: ['example.com'],
      dnsProtocol: 'dot',
      killSwitch: true,
    });
  });

  it('K-03: verifies session fields are omitted from persistent storage (partialize test)', () => {
    const partialize = (useEngineStore.persist as any).getOptions().partialize;
    const partialState = partialize(useEngineStore.getState());

    expect(partialState.status).toBeUndefined();
    expect(partialState.logs).toBeUndefined();
    expect(partialState.activeTab).toBeUndefined();
    expect(partialState.dnsSynced).toBeUndefined();
    expect(partialState.hasHydrated).toBeUndefined();
    expect(partialState.persistenceError).toBeUndefined();
  });

  it('K-04: merges legacy string domain lists into array fields during migration', () => {
    const migrated = migratePersistedEngineState({
      whitelistDomains: 'site1.com\nsite2.com',
      blacklistDomains: 'bad.com',
    });
    expect(migrated.whitelistDomains).toEqual(['site1.com', 'site2.com']);
    expect(migrated.blacklistDomains).toEqual(['bad.com']);
  });

  it('K-05: throws explicit error on null/scalar persisted state schemas', () => {
    expect(() => migratePersistedEngineState(null)).toThrow(
      'Persisted settings schema is not an object.',
    );
    expect(() => migratePersistedEngineState('invalid json string')).toThrow(
      'Persisted settings schema is not an object.',
    );
  });

  it('K-06: protects against wrong field types in persisted settings', () => {
    const migrated = migratePersistedEngineState({
      dnsForwarderEnabled: 'true' as any,
      killSwitch: 1 as any,
    });
    expect(migrated.dnsForwarderEnabled).toBe(false);
  });

  it('K-07: characterizes legacy domainList handling when whitelistDomains is absent', () => {
    const migrated = migratePersistedEngineState({
      bypassMode: 'whitelist',
      domainList: 'legacy.com',
    });
    expect(migrated.whitelistDomains).toEqual([]);
    expect(migrated.domainList).toBe('legacy.com');
  });
});
