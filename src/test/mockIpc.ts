if (typeof window === 'undefined') {
  (globalThis as any).window = globalThis;
}
if (!(globalThis as any).window.__TAURI_INTERNALS__) {
  (globalThis as any).window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args: any) => (globalThis as any).__mockIpc?.handleInvoke(cmd, args) ?? Promise.resolve(null),
    plugins: {},
  };
}

import { vi } from 'vitest';
import { useEngineStore } from '../store/engineStore';
import { DEFAULT_ADVANCED_CONFIG } from '../types/advanced';
import { DEFAULT_HEALTH_CHECK_TARGET } from '../types/ipc';

export interface InvocationCall {
  command: string;
  payload?: Record<string, any>;
  timestamp: number;
}

export interface EventCall {
  event: string;
  payload?: any;
  timestamp: number;
}

export type CommandHandler = (payload?: Record<string, any>) => any | Promise<any>;

class MockIpcController {
  public calls: InvocationCall[] = [];
  public events: EventCall[] = [];
  private handlers: Map<string, CommandHandler> = new Map();
  private defaultHandler: CommandHandler | null = null;

  constructor() {
    (globalThis as any).__mockIpc = this;
    this.setupDefaults();
  }

  public reset() {
    this.calls = [];
    this.events = [];
    this.handlers.clear();
    this.setupDefaults();
  }

  public registerHandler(command: string, handler: CommandHandler) {
    this.handlers.set(command, handler);
  }

  public registerResponses(command: string, responses: any[]) {
    let index = 0;
    this.handlers.set(command, () => {
      const res = responses[Math.min(index, responses.length - 1)];
      index++;
      if (res instanceof Error) throw res;
      return res;
    });
  }

  public registerError(command: string, error: any) {
    this.handlers.set(command, () => {
      throw error;
    });
  }

  public setupDefaults() {
    this.handlers.set('sync_bypass_config', (payload) => ({
      configRevision: 1,
      mode: payload?.mode || 'all',
      domainCount: Array.isArray(payload?.whitelistDomains) ? payload?.whitelistDomains.length : 0,
      stage: 'prepared',
      engineRunning: false,
      whitelistDomains: payload?.whitelistDomains || [],
      blacklistDomains: payload?.blacklistDomains || [],
    }));

    this.handlers.set('sync_dns_settings', (payload) => ({
      configRevision: 1,
      protocol: payload?.protocol || 'doh',
      adblock: Boolean(payload?.adblock),
      cache: Boolean(payload?.cache),
      stage: 'saved',
    }));

    this.handlers.set('start_engine_with_dns_guard', () => ({
      variant: 'ready', generation: 1, revision: 1, fingerprint: 'fixture',
      pid: 1234,
    }));

    this.handlers.set('stop_engine', () => ({}));
    this.handlers.set('get_doh_forwarder_status', () => ({ active: false }));
    this.handlers.set('start_doh_forwarder', () => ({ active: true }));
    this.handlers.set('set_dns_watchdog', (payload) => ({ active: true, watchdogEnabled: payload?.enabled ?? true }));
    this.handlers.set('list_presets', () => []);
    this.handlers.set('list_dns_providers', () => [
      { id: 'cloudflare', name: 'Cloudflare', primary: '1.1.1.1', secondary: '1.0.0.1' },
      { id: 'google', name: 'Google', primary: '8.8.8.8', secondary: '8.8.4.4' },
    ]);
    this.handlers.set('get_network_adapters', () => []);
    this.handlers.set('apply_dns_settings', () => ({ success: true }));
    this.handlers.set('settings_get', () => null);
    this.handlers.set('settings_set', () => ({}));
    this.handlers.set('settings_remove', () => ({}));
    this.handlers.set('export_preset', () => ({}));
    this.handlers.set('delete_custom_preset', () => ({}));
  }

  public async handleInvoke(command: string, payload?: Record<string, any>): Promise<any> {
    this.calls.push({ command, payload, timestamp: Date.now() });
    const handler = this.handlers.get(command) || this.defaultHandler;
    if (handler) {
      return handler(payload);
    }
    return null;
  }

  public handleEmit(event: string, payload?: any): void {
    this.events.push({ event, payload, timestamp: Date.now() });
  }

  public getCallsForCommand(command: string): InvocationCall[] {
    return this.calls.filter((c) => c.command === command);
  }

  public getCommandNames(): string[] {
    return this.calls.map((c) => c.command);
  }
}

export const mockIpc = new MockIpcController();

export function resetStoreToDefaults() {
  useEngineStore.setState({
    activePresetId: 'default',
    selectedDnsId: '',
    dnsCustomPrimary: '',
    dnsCustomSecondary: '',
    advancedConfig: DEFAULT_ADVANCED_CONFIG,
    bypassMode: 'all',
    domainList: '',
    whitelistDomains: [],
    blacklistDomains: [],
    dnsProtocol: 'doh',
    dnsAdBlock: false,
    dnsCache: true,
    proxySocks5: '',
    killSwitch: false,
    watchdog: true,
    dnsForwarderEnabled: false,
    language: 'en',
    status: { variant: 'stopped' },
    presets: [],
    logs: [],
    activeTab: 'home',
    dnsProviders: [],
    dnsSynced: false,
    advancedDirty: false,
    healthCheckTargets: [DEFAULT_HEALTH_CHECK_TARGET],
    hasHydrated: true,
    persistenceError: null,
  });
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, payload?: Record<string, any>) =>
    (globalThis as any).__mockIpc?.handleInvoke(command, payload) ?? Promise.resolve(null),
}));

vi.mock('@tauri-apps/api/event', () => ({
  emit: (event: string, payload?: any) =>
    Promise.resolve((globalThis as any).__mockIpc?.handleEmit(event, payload)),
  listen: () => Promise.resolve(() => {}),
}));
