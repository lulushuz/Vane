import type { EngineStatus, Preset } from './engine';

/** `start_engine` komutunun parametresi */
export interface StartEnginePayload {
  presetId: string;
}

/** `save_custom_preset` komutunun parametresi */
export interface SaveCustomPresetPayload {
  preset: Preset;
}

/** `delete_custom_preset` komutunun parametresi */
export interface DeleteCustomPresetPayload {
  presetId: string;
}

/** `get_engine_status` dönüş tipi */
export type GetEngineStatusResponse = EngineStatus;

/** `list_presets` dönüş tipi */
export type ListPresetsResponse = Preset[];

export interface IpcErrorPayload {
  code: string;
  message: string;
  operation?: string;
  retryable?: boolean;
  configRevision?: number;
}

export interface BypassConfigStatus {
  mode: 'all' | 'whitelist' | 'blacklist';
  domainCount: number;
  configRevision: number;
  stage: 'prepared' | 'process_started';
  engineRestarted: boolean;
  engineRunning: boolean;
  whitelistDomains: string[];
  blacklistDomains: string[];
  activePresetId: string;
}

export interface DnsConfigStatus {
  protocol: 'doh' | 'dot';
  adblock: boolean;
  cache: boolean;
  socks5Proxy: string;
  forwarderActive: boolean;
  configRevision: number;
  stage: 'persisted' | 'applied';
}

export function normalizeIpcError(error: unknown): IpcErrorPayload {
  if (typeof error === 'object' && error !== null) {
    const payload = error as Record<string, unknown>;
    return {
      code: typeof payload.code === 'string' ? payload.code : 'UNKNOWN',
      message: typeof payload.message === 'string' ? payload.message : String(error),
      operation: typeof payload.operation === 'string' ? payload.operation : undefined,
      retryable: typeof payload.retryable === 'boolean' ? payload.retryable : undefined,
      configRevision: typeof payload.configRevision === 'number' ? payload.configRevision : undefined,
    };
  }

  return {
    code: 'UNKNOWN',
    message: error instanceof Error ? error.message : String(error),
  };
}
