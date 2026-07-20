export type PersistedState = Record<string, unknown>;

export function normalizePersistedDomains(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.filter((item): item is string => typeof item === 'string');
  }
  if (typeof value === 'string') {
    return value
      .split('\n')
      .map((item) => item.trim())
      .filter(Boolean);
  }
  return [];
}

export function migratePersistedEngineState(persistedState: unknown): PersistedState {
  if (!persistedState || typeof persistedState !== 'object' || Array.isArray(persistedState)) {
    throw new Error('Persisted settings schema is not an object.');
  }

  const state = persistedState as PersistedState;
  return {
    ...state,
    whitelistDomains: normalizePersistedDomains(state.whitelistDomains),
    blacklistDomains: normalizePersistedDomains(state.blacklistDomains),
    dnsForwarderEnabled: state.dnsForwarderEnabled === true,
  };
}

export function activePatternDomains(
  mode: 'all' | 'whitelist' | 'blacklist',
  whitelistDomains: unknown,
  blacklistDomains: unknown,
): string[] {
  if (mode === 'all') return [];
  return normalizePersistedDomains(
    mode === 'whitelist' ? whitelistDomains : blacklistDomains,
  );
}
