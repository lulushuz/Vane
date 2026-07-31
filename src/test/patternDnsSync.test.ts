import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';
import { mockIpc, resetStoreToDefaults } from './mockIpc';
import { useEngineStore } from '../store/engineStore';
import { activePatternDomains, normalizePersistedDomains } from '../store/persistence';

describe('Test Group F — Pattern Debounce and Revision Characterization', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockIpc.reset();
    resetStoreToDefaults();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('F-01: debounces single pattern change by 100ms before sending IPC', async () => {
    useEngineStore.getState().setWhitelistDomains(['example.com']);

    expect(mockIpc.getCallsForCommand('sync_bypass_config')).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(100);

    expect(mockIpc.getCallsForCommand('sync_bypass_config')).toHaveLength(1);
  });

  it('F-02: collapses 10 rapid pattern domain changes into a single IPC payload', async () => {
    for (let i = 1; i <= 10; i++) {
      useEngineStore.getState().setWhitelistDomains([`site${i}.example`]);
      await vi.advanceTimersByTimeAsync(10);
    }

    expect(mockIpc.getCallsForCommand('sync_bypass_config')).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(100);

    const calls = mockIpc.getCallsForCommand('sync_bypass_config');
    expect(calls).toHaveLength(1);
    expect(calls[0].payload?.whitelistDomains).toEqual(['site10.example']);
  });

  it('F-03: discards stale pattern response when revision is outdated', async () => {
    let respondFirst: (value: any) => void;
    const firstCallPromise = new Promise((resolve) => {
      respondFirst = resolve;
    });

    let callCount = 0;
    mockIpc.registerHandler('sync_bypass_config', () => {
      callCount++;
      if (callCount === 1) {
        return firstCallPromise;
      }
      return {
        configRevision: 2,
        mode: 'whitelist',
        domainCount: 1,
        whitelistDomains: ['second.example'],
        blacklistDomains: [],
      };
    });

    // Request 1
    useEngineStore.getState().setWhitelistDomains(['first.example']);
    await vi.advanceTimersByTimeAsync(100);

    // Request 2
    useEngineStore.getState().setWhitelistDomains(['second.example']);
    await vi.advanceTimersByTimeAsync(100);

    // Resolve request 1 stale response
    respondFirst!({
      configRevision: 1,
      mode: 'whitelist',
      domainCount: 1,
      whitelistDomains: ['stale.example'],
      blacklistDomains: [],
    });
    await Promise.resolve();

    const state = useEngineStore.getState();
    expect(state.whitelistDomains).toEqual(['second.example']);
  });

  it('F-04: sets whitelistDomains and blacklistDomains from verified backend response', async () => {
    mockIpc.registerHandler('sync_bypass_config', () => ({
      configRevision: 10,
      mode: 'whitelist',
      domainCount: 2,
      whitelistDomains: ['canonical1.com', 'canonical2.com'],
      blacklistDomains: [],
      stage: 'process_started',
    }));

    useEngineStore.getState().setWhitelistDomains(['CANONICAL1.COM']);
    await vi.advanceTimersByTimeAsync(100);

    expect(useEngineStore.getState().whitelistDomains).toEqual(['canonical1.com', 'canonical2.com']);
  });

  it('F-05: distinguishes prepared vs process_started stage log messages in Turkish', async () => {
    useEngineStore.setState({ language: 'tr' });
    mockIpc.registerHandler('sync_bypass_config', () => ({
      configRevision: 1,
      mode: 'all',
      domainCount: 0,
      whitelistDomains: [],
      blacklistDomains: [],
      stage: 'process_started',
    }));

    useEngineStore.getState().setBypassMode('all');
    await vi.advanceTimersByTimeAsync(100);

    const logs = useEngineStore.getState().logs;
    expect(logs.some((l) => l.content.includes('Yeni kurallarla motor prosesi başlatıldı'))).toBe(true);
  });

  it('F-06: allows sending empty whitelist payload to backend', async () => {
    useEngineStore.getState().setBypassMode('whitelist');
    useEngineStore.getState().setWhitelistDomains([]);

    await vi.advanceTimersByTimeAsync(100);

    const call = mockIpc.getCallsForCommand('sync_bypass_config').pop();
    expect(call?.payload?.whitelistDomains).toEqual([]);
  });
});

describe('Test Group G — DNS Debounce and Rollback Characterization', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockIpc.reset();
    resetStoreToDefaults();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('G-01: sends correct sync_dns_settings payload after 100ms debounce', async () => {
    useEngineStore.getState().setDnsAdBlock(true);

    await vi.advanceTimersByTimeAsync(100);

    const call = mockIpc.getCallsForCommand('sync_dns_settings').pop();
    expect(call?.payload).toMatchObject({
      protocol: 'doh',
      adblock: true,
      cache: true,
    });
  });

  it('G-02: debounces rapid protocol changes (doh -> dot -> doh)', async () => {
    useEngineStore.getState().setDnsProtocol('dot');
    await vi.advanceTimersByTimeAsync(20);
    useEngineStore.getState().setDnsProtocol('doh');

    await vi.advanceTimersByTimeAsync(100);

    const calls = mockIpc.getCallsForCommand('sync_dns_settings');
    expect(calls).toHaveLength(1);
    expect(calls[0].payload?.protocol).toBe('doh');
  });


  it('G-03: rolls back frontend state on backend DNS sync rejection', async () => {
    useEngineStore.setState({ language: 'tr' });
    mockIpc.registerError('sync_dns_settings', {
      code: 'DNS_SYNC_FAILED',
      message: 'Invalid DNS configuration',
    });

    useEngineStore.getState().setDnsAdBlock(true);
    await vi.advanceTimersByTimeAsync(100);

    expect(useEngineStore.getState().dnsAdBlock).toBe(false);
    expect(useEngineStore.getState().logs.some((l) => l.content.includes('DNS ayarı doğrulanamadı'))).toBe(true);
  });

  it('G-04: clears pending rollback when backend DNS sync succeeds', async () => {
    useEngineStore.getState().setDnsAdBlock(true);
    await vi.advanceTimersByTimeAsync(100);

    // Next failure shouldn't rollback to pre-G04 state
    mockIpc.registerError('sync_dns_settings', 'Failed');
    useEngineStore.getState().setDnsCache(false);
    await vi.advanceTimersByTimeAsync(100);

    expect(useEngineStore.getState().dnsAdBlock).toBe(true); // preserved from success
    expect(useEngineStore.getState().dnsCache).toBe(true); // rolled back
  });

  it('G-05: keeps Pattern and DNS timers independent when modified simultaneously', async () => {
    useEngineStore.getState().setWhitelistDomains(['example.com']);
    useEngineStore.getState().setDnsAdBlock(true);

    await vi.advanceTimersByTimeAsync(100);

    expect(mockIpc.getCallsForCommand('sync_bypass_config')).toHaveLength(1);
    expect(mockIpc.getCallsForCommand('sync_dns_settings')).toHaveLength(1);
  });

  it('G-06: flushes pending timers immediately during engine start', async () => {
    useEngineStore.getState().setWhitelistDomains(['immediate.com']);
    useEngineStore.getState().setDnsAdBlock(true);

    // Call startEngine before timer expires
    await useEngineStore.getState().startEngine('default');

    expect(mockIpc.getCallsForCommand('sync_bypass_config')).toHaveLength(1);
    expect(mockIpc.getCallsForCommand('sync_dns_settings')).toHaveLength(1);
  });
});

describe('Test Group L — Domain Helper Characterization', () => {
  it('L-01: normalizes legacy string domain lists with whitespace and empty lines', () => {
    const input = '  example.com \n\n api.example.org \n  ';
    expect(normalizePersistedDomains(input)).toEqual(['example.com', 'api.example.org']);
  });

  it('L-02: filters non-string elements from array domain lists', () => {
    const input = ['example.com', null, 123, 'valid.org', undefined];
    expect(normalizePersistedDomains(input)).toEqual(['example.com', 'valid.org']);
  });

  it('L-03: returns active pattern domains based on mode selection', () => {
    const wl = ['wl.com'];
    const bl = ['bl.com'];

    expect(activePatternDomains('all', wl, bl)).toEqual([]);
    expect(activePatternDomains('whitelist', wl, bl)).toEqual(['wl.com']);
    expect(activePatternDomains('blacklist', wl, bl)).toEqual(['bl.com']);
  });
});
