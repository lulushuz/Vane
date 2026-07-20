import { describe, expect, it } from 'vitest';
import {
  activePatternDomains,
  migratePersistedEngineState,
  normalizePersistedDomains,
} from './persistence';

describe('persisted settings migration', () => {
  it('rejects missing, scalar, and array schemas', () => {
    for (const invalid of [null, undefined, 'state', 1, []]) {
      expect(() => migratePersistedEngineState(invalid)).toThrow(
        'Persisted settings schema is not an object.',
      );
    }
  });

  it('migrates legacy domain lists without losing unrelated settings', () => {
    const migrated = migratePersistedEngineState({
      activePresetId: 'general-alt4',
      bypassMode: 'whitelist',
      whitelistDomains: ' example.com\n\napi.example.org ',
      blacklistDomains: null,
      dnsCache: false,
      watchdog: true,
      killSwitch: true,
      advancedConfig: { splitPosition: 2 },
      dnsForwarderEnabled: true,
    });

    expect(migrated).toMatchObject({
      activePresetId: 'general-alt4',
      bypassMode: 'whitelist',
      whitelistDomains: ['example.com', 'api.example.org'],
      blacklistDomains: [],
      dnsCache: false,
      watchdog: true,
      killSwitch: true,
      advancedConfig: { splitPosition: 2 },
      dnsForwarderEnabled: true,
    });
  });

  it('does not enable a persisted forwarder value with the wrong type', () => {
    expect(migratePersistedEngineState({ dnsForwarderEnabled: 'true' }))
      .toMatchObject({ dnsForwarderEnabled: false });
  });
});

describe('Pattern domain selection', () => {
  const whitelist = ['allowed.example'];
  const blacklist = ['blocked.example'];

  it('uses only the whitelist in whitelist mode', () => {
    expect(activePatternDomains('whitelist', whitelist, blacklist)).toEqual(whitelist);
  });

  it('uses only the blacklist in blacklist mode', () => {
    expect(activePatternDomains('blacklist', whitelist, blacklist)).toEqual(blacklist);
  });

  it('returns no hostlist in all-sites mode', () => {
    expect(activePatternDomains('all', whitelist, blacklist)).toEqual([]);
  });

  it('normalizes legacy strings and ignores invalid values', () => {
    expect(normalizePersistedDomains(' one.example\n two.example ')).toEqual([
      'one.example',
      'two.example',
    ]);
    expect(normalizePersistedDomains({ domain: 'invalid.example' })).toEqual([]);
  });
});
