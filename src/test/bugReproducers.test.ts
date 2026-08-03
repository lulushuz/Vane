import { describe, expect, it, beforeEach } from 'vitest';
import { mockIpc, resetStoreToDefaults } from './mockIpc';
import { useEngineStore } from '../store/engineStore';
import { parseArgsToConfig, serializeConfigToArgs } from '../utils/argsParser';

describe('Bug Reproducer Tests (P01 Behavior Freezing)', () => {
  beforeEach(() => {
    mockIpc.reset();
    resetStoreToDefaults();
  });

  it('BR-01 resolved: engine restart uses the verified Pattern snapshot', async () => {
    useEngineStore.setState({
      bypassMode: 'whitelist',
      whitelistDomains: ['verified-authoritative.com'],
    });

    const startPromise = useEngineStore.getState().startEngine('default');

    const bypassCall = mockIpc.getCallsForCommand('sync_bypass_config').pop();
    expect(bypassCall).toBeDefined();
    expect(bypassCall?.payload?.mode).toBe('whitelist');
    expect(bypassCall?.payload?.whitelistDomains).toEqual(['verified-authoritative.com']);

    await startPromise;
  });


  it('BR-02: documents .json vs .vane export extension mismatch (Target: P08, Risk: R-02)', async () => {
    mockIpc.registerError('export_preset', 'Backend requires signed .vane format');

    await expect(
      mockIpc.handleInvoke('export_preset', {
        filePath: 'C:/presets/my-preset.json',
        content: '{}',
      }),
    ).rejects.toBe('Backend requires signed .vane format');
  });

  it('BR-03: verifies legacy doq selection is migrated to doh upon state hydration (R-14 resolved)', async () => {
    const { migratePersistedEngineState } = await import('../store/persistence');
    const migrated = migratePersistedEngineState({ dnsProtocol: 'doq' });
    expect(migrated.dnsProtocol).toBe('doh');
  });


  it('BR-04: documents optimistic UI representation when backend returns Prepared stage (Target: P04/P05, Risk: R-26)', async () => {
    mockIpc.registerHandler('sync_bypass_config', () => ({
      configRevision: 1,
      mode: 'whitelist',
      domainCount: 1,
      whitelistDomains: ['site.com'],
      blacklistDomains: [],
      stage: 'prepared', // Not yet started
      engineRunning: false,
    }));

    useEngineStore.getState().setBypassMode('whitelist');
    useEngineStore.getState().setWhitelistDomains(['site.com']);

    // UI state shows whitelist immediately (optimistic applied state)
    expect(useEngineStore.getState().bypassMode).toBe('whitelist');
    expect(useEngineStore.getState().whitelistDomains).toEqual(['site.com']);
  });

  it('BR-05: documents PID-only running status without active traffic health check (Target: P14, Risk: R-17)', async () => {
    mockIpc.registerHandler('start_engine_with_dns_guard', () => ({
      variant: 'ready', generation: 1, revision: 1, fingerprint: 'fixture',
      pid: 4321,
    }));

    await useEngineStore.getState().startEngine('default');

    const status = useEngineStore.getState().status;
    expect(status).toEqual({ variant: 'ready', pid: 4321, generation: 1, revision: 1, fingerprint: 'fixture' });
    // Healthy connectivity state is not verified separately in current UI state model
  });

  it('BR-06 resolved: non-443 UDP port ranges survive Advanced parse and serialization', () => {
    const rawArgs = ['--wf-tcp=80,443', '--wf-udp=50000-65535', '--dpi-desync=fake'];
    const parsed = parseArgsToConfig(rawArgs);
    const serialized = serializeConfigToArgs(parsed);

    // --wf-udp=50000-65535 is preserved in reserialized output by new P09 parser
    expect(serialized).toContain('--wf-udp=50000-65535');
  });

  it('BR-07 resolved: late start completion cannot override a newer stop operation', async () => {
    let resolveStart: (val: any) => void;
    const slowStartPromise = new Promise((r) => {
      resolveStart = r;
    });

    mockIpc.registerHandler('start_engine_with_dns_guard', () => slowStartPromise);

    const startPromise = useEngineStore.getState().startEngine('default');
    expect(useEngineStore.getState().status).toEqual({ variant: 'starting' });

    // Stop engine while start is in flight
    await useEngineStore.getState().stopEngine();
    expect(useEngineStore.getState().status).toEqual({ variant: 'stopped' });

    // Resolve slow start afterwards
    resolveStart!({ variant: 'ready', pid: 7777, generation: 1, revision: 1, fingerprint: 'fixture' });
    await startPromise;

    // Late start completion MUST NOT override status to running when stop was requested
    expect(useEngineStore.getState().status).toEqual({ variant: 'stopped' });
  });

  it('BR-08 resolved: stale DNS responses cannot override a newer backend revision', async () => {
    let resolveFirstDns: (val: any) => void;
    const slowDnsPromise = new Promise((r) => {
      resolveFirstDns = r;
    });

    let callCount = 0;
    mockIpc.registerHandler('sync_dns_settings', () => {
      callCount++;
      if (callCount === 1) return slowDnsPromise;
      return { configRevision: 2, protocol: 'dot', adblock: false, cache: true, stage: 'applied' };
    });

    // Call 1
    useEngineStore.getState().setDnsAdBlock(true);

    // Call 2
    useEngineStore.getState().setDnsAdBlock(false);

    // Resolve call 1 after call 2 with stale revision 1
    resolveFirstDns!({ configRevision: 1, protocol: 'doh', adblock: true, cache: true, stage: 'applied', superseded: true });
    await Promise.resolve();

    expect(useEngineStore.getState().dnsAdBlock).toBe(false);
  });
});
