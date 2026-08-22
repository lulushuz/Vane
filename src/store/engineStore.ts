import { create } from 'zustand';
import { persist, createJSONStorage, type StateStorage } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import type { NetworkAdapter } from '../types/network';
import { DEFAULT_ADVANCED_CONFIG, type AdvancedConfig } from '../types/advanced';
import type { EngineStatus, Preset } from '../types/engine';
import {
  DEFAULT_HEALTH_CHECK_TARGET,
  normalizeIpcError,
  type BypassConfigStatus,
  type DnsConfigStatus,
} from '../types/ipc';
import { activePatternDomains, migratePersistedEngineState } from './persistence';

export { DEFAULT_ADVANCED_CONFIG } from '../types/advanced';
export type { AdvancedConfig } from '../types/advanced';

export interface LogLine {
  id: string;
  timestamp: Date;
  content: string;
  level: 'info' | 'warn' | 'error';
}

export type AppTab = 'home' | 'logs' | 'custom' | 'test' | 'dns';

export interface DnsProvider {
  id: string;
  name: string;
  primary: string;
  secondary: string;
}

/*
   Rust-owned settings adapter. Zustand remains the UI state model, while Rust
   serializes multi-window updates and atomically replaces settings.json with a
   last-known-good backup so a crash cannot silently reset user preferences.
*/

let storeWriteQueue: Promise<void> = Promise.resolve();

const enqueueStoreWrite = (operation: () => Promise<void>): Promise<void> => {
  const queued = storeWriteQueue.then(operation, operation);
  storeWriteQueue = queued.catch(() => undefined);
  return queued;
};

function createTauriStorage(): StateStorage {
  return {
    getItem: async (key: string): Promise<string | null> => {
      await storeWriteQueue;
      return await invoke<string | null>('settings_get', { key });
    },
    setItem: async (key: string, value: string): Promise<void> => {
      return enqueueStoreWrite(async () => {
        await invoke('settings_set', { key, value });
      });
    },
    removeItem: async (key: string): Promise<void> => {
      return enqueueStoreWrite(async () => {
        await invoke('settings_remove', { key });
      });
    },
  };
}

// ─── Store Interface ────────────────────────────────────────────────────────

interface EngineStore {
  // Kalıcı (persist edilecek) alanlar
  activePresetId: string | null;
  selectedDnsId: string;
  dnsCustomPrimary: string;
  dnsCustomSecondary: string;
  advancedConfig: AdvancedConfig;
  healthCheckTargets: string[];
  bypassMode: 'all' | 'whitelist' | 'blacklist';
  domainList: string;
  whitelistDomains: string[];
  blacklistDomains: string[];
  dnsProtocol: 'doh' | 'dot';
  dnsAdBlock: boolean;
  dnsCache: boolean;
  proxySocks5: string;
  killSwitch: boolean;
  watchdog: boolean;
  dnsForwarderEnabled: boolean;
  language: 'tr' | 'en';

  // Geçici (session) alanlar
  status: EngineStatus;
  presets: Preset[];
  logs: LogLine[];
  activeTab: AppTab;
  dnsProviders: DnsProvider[];
  dnsSynced: boolean;
  advancedDirty: boolean;  // kaydedilmemiş gelişmiş ayar var mı?

  setStatus: (status: EngineStatus) => void;
  setActivePreset: (presetId: string | null) => void;
  setPresets: (presets: Preset[]) => void;
  upsertPreset: (preset: Preset) => void;
  setAdvancedConfig: (config: Partial<AdvancedConfig>) => void;
  resetAdvancedConfig: () => void;
  setAdvancedDirty: (dirty: boolean) => void;
  appendLog: (content: string, level?: LogLine['level']) => void;
  appendLogs: (entries: { content: string, level: LogLine['level'] }[]) => void;
  clearLogs: () => void;
  setActiveTab: (tab: AppTab) => void;
  setHealthCheckTargets: (targets: string[]) => void;
  setBypassMode: (mode: 'all' | 'whitelist' | 'blacklist') => void;
  setDomainList: (list: string) => void;
  setWhitelistDomains: (list: string[]) => void;
  setBlacklistDomains: (list: string[]) => void;
  setDnsProtocol: (protocol: 'doh' | 'dot') => void;
  setDnsAdBlock: (enabled: boolean) => void;
  setDnsCache: (enabled: boolean) => void;

  setProxySocks5: (addr: string) => Promise<boolean>;
  setKillSwitch: (enabled: boolean) => void;
  setWatchdog: (enabled: boolean) => void;
  setDnsForwarderEnabled: (enabled: boolean) => void;
  setLanguage: (lang: 'tr' | 'en') => void;
  syncBypassToBackend: () => void;
  syncDnsToBackend: () => void;

  refreshPresets: () => Promise<void>;
  deletePreset: (presetId: string) => Promise<void>;
  startEngine: (presetId?: string) => Promise<void>;
  stopEngine: () => Promise<void>;
  refreshDnsStatus: () => Promise<void>;

  setDnsProviders: (providers: DnsProvider[]) => void;
  setSelectedDnsId: (id: string) => void;
  setDnsCustom: (primary: string, secondary: string) => void;
  setDnsSynced: (synced: boolean) => void;
  hasHydrated: boolean;
  persistenceError: string | null;
  setHasHydrated: (val: boolean) => void;
}

let logCounter = 0;
let bypassSyncTimeout: ReturnType<typeof setTimeout> | null = null;
let dnsSyncTimeout: ReturnType<typeof setTimeout> | null = null;
let bypassSyncRevision = 0;
let dnsSyncRevision = 0;
let lifecycleToken = 0;
let pendingDnsRollback: Partial<Pick<
  EngineStore,
  'dnsProtocol' | 'dnsAdBlock' | 'dnsCache' | 'proxySocks5' | 'healthCheckTargets'
>> = {};

const rememberDnsRollback = (
  key: keyof typeof pendingDnsRollback,
  value: EngineStore[keyof typeof pendingDnsRollback],
) => {
  if (!(key in pendingDnsRollback)) {
    Object.assign(pendingDnsRollback, { [key]: value });
  }
};

export const useEngineStore = create<EngineStore>()(
  persist(
    (set, get) => ({
      // Persist edilecek başlangıç değerleri
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

      // Session değerleri (persist edilmez)
      status: { variant: 'stopped' },
      presets: [],
      logs: [],
      activeTab: 'home',
      dnsProviders: [],
      dnsSynced: false,
      advancedDirty: false,
      healthCheckTargets: [DEFAULT_HEALTH_CHECK_TARGET],
      hasHydrated: false,
      persistenceError: null,

      setStatus: (status) => set({ status }),
      setActivePreset: async (presetId) => {
        set({ activePresetId: presetId });
        try {
          await emit('sync_active_preset', presetId);
        } catch (err) { /* ignore in dev */ }
      },
      setPresets: (presets) => set({ presets }),
      upsertPreset: (preset) => set((state) => {
        const exists = state.presets.some(p => p.id === preset.id);
        return {
          presets: exists
            ? state.presets.map(p => p.id === preset.id ? preset : p)
            : [...state.presets, preset],
        };
      }),

      setAdvancedConfig: (partial) => set((state) => ({
        advancedConfig: { ...state.advancedConfig, ...partial },
      })),

      resetAdvancedConfig: () => set({ advancedConfig: DEFAULT_ADVANCED_CONFIG }),
      setAdvancedDirty: (advancedDirty) => set({ advancedDirty }),
      setActiveTab: (tab) => set({ activeTab: tab }),
      setHealthCheckTargets: (healthCheckTargets) => {
        rememberDnsRollback('healthCheckTargets', get().healthCheckTargets);
        set({ healthCheckTargets });
        get().syncDnsToBackend();
      },
      clearLogs: () => set({ logs: [] }),
      setBypassMode: (bypassMode) => {
        set({ bypassMode });
        get().syncBypassToBackend();
      },
      setDomainList: (domainList) => {
        set({ domainList });
        get().syncBypassToBackend();
      },
      setWhitelistDomains: (whitelistDomains) => {
        set({ whitelistDomains });
        get().syncBypassToBackend();
      },
      setBlacklistDomains: (blacklistDomains) => {
        set({ blacklistDomains });
        get().syncBypassToBackend();
      },
      setDnsProtocol: (dnsProtocol) => {
        if (dnsProtocol === 'dot' && get().proxySocks5) {
          get().appendLog(
            get().language === 'tr'
              ? '[ERROR] SOCKS5 proxy etkinken DoT seçilemez; DNS sızıntısını önlemek için değişiklik uygulanmadı.'
              : '[ERROR] DoT cannot be selected while SOCKS5 is configured; the change was rejected to prevent a DNS leak.',
            'error',
          );
          return;
        }
        rememberDnsRollback('dnsProtocol', get().dnsProtocol);
        set({ dnsProtocol });
        get().syncDnsToBackend();
      },
      setDnsAdBlock: (dnsAdBlock) => {
        rememberDnsRollback('dnsAdBlock', get().dnsAdBlock);
        set({ dnsAdBlock });
        get().syncDnsToBackend();
      },
      setDnsCache: (dnsCache) => {
        rememberDnsRollback('dnsCache', get().dnsCache);
        set({ dnsCache });
        get().syncDnsToBackend();
      },
      setProxySocks5: (proxySocks5) => {
        if (proxySocks5 && get().dnsProtocol === 'dot') {
          get().appendLog(
            get().language === 'tr'
              ? '[ERROR] DoT etkinken SOCKS5 proxy kaydedilemez; önce DoH seçin.'
              : '[ERROR] SOCKS5 cannot be saved while DoT is active; select DoH first.',
            'error',
          );
          return Promise.resolve(false);
        }
        const previous = get().proxySocks5;
        const state = get();
        const wl = Array.isArray(state.whitelistDomains) ? state.whitelistDomains : [];
        const bl = Array.isArray(state.blacklistDomains) ? state.blacklistDomains : [];
        const activeDomains = activePatternDomains(state.bypassMode, wl, bl);
        return (async () => {
          try {
            await invoke('sync_dns_settings', {
              protocol: state.dnsProtocol,
              adblock: state.dnsAdBlock,
              cache: state.dnsCache,
              socks5Proxy: proxySocks5,
              healthCheckTargets: state.healthCheckTargets,
              emitEvent: false,
              enabled: state.dnsForwarderEnabled,
            });
            await invoke('sync_bypass_config', {
              mode: state.bypassMode,
              list: activeDomains.join('\n'),
              proxy: proxySocks5,
              killSwitch: state.killSwitch,
              whitelistDomains: wl,
              blacklistDomains: bl,
              activePresetId: state.activePresetId || 'default',
            });
            set({ proxySocks5 });
            get().appendLog(
              get().language === 'tr'
                ? `[DNS] SOCKS5H proxy iki çalışma katmanında doğrulandı: ${proxySocks5 || 'doğrudan bağlantı'}.`
                : `[DNS] SOCKS5H proxy was verified in both runtime layers: ${proxySocks5 || 'direct connection'}.`,
              'info',
            );
            return true;
          } catch (error) {
            const ipcError = normalizeIpcError(error);
            try {
              await invoke('sync_dns_settings', {
                protocol: state.dnsProtocol,
                adblock: state.dnsAdBlock,
                cache: state.dnsCache,
                socks5Proxy: previous,
                healthCheckTargets: state.healthCheckTargets,
                emitEvent: false,
                enabled: state.dnsForwarderEnabled,
              });
              await invoke('sync_bypass_config', {
                mode: state.bypassMode,
                list: activeDomains.join('\n'),
                proxy: previous,
                killSwitch: state.killSwitch,
                whitelistDomains: wl,
                blacklistDomains: bl,
                activePresetId: state.activePresetId || 'default',
              });
            } catch { /* backend logs contain rollback details */ }
            get().appendLog(
              get().language === 'tr'
                ? `[ERROR] SOCKS5 proxy doğrulanamadı; önceki ayar korundu: ${ipcError.message} (${ipcError.code})`
                : `[ERROR] SOCKS5 proxy could not be verified; the previous setting was preserved: ${ipcError.message} (${ipcError.code})`,
              'error',
            );
            return false;
          }
        })();
      },
      setKillSwitch: (killSwitch) => {
        const state = get();
        const wl = Array.isArray(state.whitelistDomains) ? state.whitelistDomains : [];
        const bl = Array.isArray(state.blacklistDomains) ? state.blacklistDomains : [];
        const activeDomains = activePatternDomains(state.bypassMode, wl, bl);
        void invoke<BypassConfigStatus>('sync_bypass_config', {
          mode: state.bypassMode,
          list: activeDomains.join('\n'),
          proxy: state.proxySocks5,
          killSwitch,
          whitelistDomains: wl,
          blacklistDomains: bl,
          activePresetId: state.activePresetId || 'default',
        }).then((verified) => {
          set({ killSwitch });
          get().appendLog(
            get().language === 'tr'
              ? `[SECURITY] DNS Kill Switch ${killSwitch ? 'açık' : 'kapalı'} olarak doğrulandı; motor ${verified.engineRunning ? 'çalışıyor' : 'kapalı'}.`
              : `[SECURITY] DNS Kill Switch was verified ${killSwitch ? 'on' : 'off'}; engine is ${verified.engineRunning ? 'running' : 'stopped'}.`,
            'info',
          );
        }).catch((cause) => {
          const error = normalizeIpcError(cause);
          get().appendLog(
            get().language === 'tr'
              ? `[ERROR] DNS Kill Switch değiştirilemedi; önceki ayar korundu: ${error.message} (${error.code})`
              : `[ERROR] DNS Kill Switch could not be changed; the previous setting was preserved: ${error.message} (${error.code})`,
            'error',
          );
        });
      },
      setWatchdog: (watchdog) => {
        invoke<{ active: boolean; watchdogEnabled: boolean }>('set_dns_watchdog', { enabled: watchdog })
          .then((status) => {
            set({ watchdog: status.active ? status.watchdogEnabled : watchdog });
            const tr = get().language === 'tr';
            get().appendLog(status.active
              ? (tr
                ? `[DNS] Bağlantı gözlemcisi çalışma sırasında doğrulandı: ${status.watchdogEnabled ? 'açık' : 'kapalı'}.`
                : `[DNS] Connection watchdog runtime state verified: ${status.watchdogEnabled ? 'on' : 'off'}.`)
              : (tr
                ? '[DNS] Bağlantı gözlemcisi ayarı kaydedildi; DNS yönlendiricisi başladığında uygulanacak.'
                : '[DNS] Connection watchdog setting saved; it will apply when the DNS forwarder starts.'), 'info');
          })
          .catch((error) => get().appendLog(
            get().language === 'tr'
              ? `[ERROR] Bağlantı gözlemcisi değiştirilemedi: ${error}`
              : `[ERROR] Connection watchdog could not be changed: ${error}`,
            'error',
          ));
      },
      setDnsForwarderEnabled: (dnsForwarderEnabled) => {
        if (dnsSyncTimeout) {
          clearTimeout(dnsSyncTimeout);
          dnsSyncTimeout = null;
        }
        set({ dnsForwarderEnabled });
        emit('sync_dns_forwarder_enabled', dnsForwarderEnabled).catch(() => {});
      },
      setHasHydrated: (hasHydrated) => set({ hasHydrated }),
      setLanguage: async (language) => {
        set({ language });
        try {
          await emit('sync_language', language);
        } catch (err) { /* ignore */ }
      },
      syncBypassToBackend: () => {
        const revision = ++bypassSyncRevision;
        if (bypassSyncTimeout) clearTimeout(bypassSyncTimeout);
        bypassSyncTimeout = setTimeout(() => {
          const state = get();
          
          const wl = Array.isArray(state.whitelistDomains) ? state.whitelistDomains : [];
          const bl = Array.isArray(state.blacklistDomains) ? state.blacklistDomains : [];
          const activeDomains = activePatternDomains(state.bypassMode, wl, bl);

          invoke<BypassConfigStatus>('sync_bypass_config', {
            mode: state.bypassMode,
            list: activeDomains.join('\n'),
            proxy: state.proxySocks5,
            killSwitch: state.killSwitch,
            whitelistDomains: wl,
            blacklistDomains: bl,
            activePresetId: state.activePresetId,
          }).then((verified) => {
            if (revision !== bypassSyncRevision) return;
            const verifiedActiveDomains = activePatternDomains(
              verified.mode,
              verified.whitelistDomains,
              verified.blacklistDomains,
            );
            set({
              whitelistDomains: verified.whitelistDomains,
              blacklistDomains: verified.blacklistDomains,
              domainList: verifiedActiveDomains.join('\n'),
            });
            const tr = get().language === 'tr';
            const modeKey = (verified.mode || 'all') as 'all' | 'whitelist' | 'blacklist';
            const modeText = tr
              ? ({ all: 'tüm siteler', whitelist: 'yalnızca beyaz liste', blacklist: 'kara liste hariç' } as const)[modeKey]
              : ({ all: 'all sites', whitelist: 'whitelist only', blacklist: 'except blacklist' } as const)[modeKey];
            const applyText = verified.stage === 'process_started'
              ? (tr ? 'Yeni kurallarla motor prosesi başlatıldı; trafik sağlığı ayrıca izlenecek.' : 'The engine process started with the new rules; traffic health is monitored separately.')
              : (tr ? 'Kural motora hazırlandı ve bir sonraki başlangıçta kullanılacak.' : 'The rule was prepared for the engine and will be used on its next start.');
            get().appendLog(
              tr
                ? `[PATTERN] Yapılandırma #${verified.configRevision} kabul edildi: ${modeText}; ${verified.domainCount} alan adı. ${applyText}`
                : `[PATTERN] Configuration #${verified.configRevision} accepted: ${modeText}; ${verified.domainCount} domains. ${applyText}`,
              'info',
            );
          }).catch(cause => {
            const tr = get().language === 'tr';
            const error = normalizeIpcError(cause);
            get().appendLog(tr
              ? `[ERROR] Desen ayarı uygulanamadı: ${error.message} (${error.code})`
              : `[ERROR] Pattern setting could not be applied: ${error.message} (${error.code})`, 'error');
          });
        }, 100);
      },
      syncDnsToBackend: () => {
        const revision = ++dnsSyncRevision;
        if (dnsSyncTimeout) clearTimeout(dnsSyncTimeout);
        dnsSyncTimeout = setTimeout(() => {
          const state = get();
          const protocol = state.dnsProtocol;
          invoke<DnsConfigStatus>('sync_dns_settings', {
            protocol,
            adblock: state.dnsAdBlock,
            cache: state.dnsCache,
            socks5Proxy: state.proxySocks5,
            healthCheckTargets: state.healthCheckTargets,
            emitEvent: true,
            enabled: state.dnsForwarderEnabled,
          }).then((verified) => {
            if (revision !== dnsSyncRevision || verified.stage === 'superseded' || (verified as any).superseded) return;
            pendingDnsRollback = {};
            const tr = get().language === 'tr';
            const activeText = verified.stage === 'applied'
              ? (tr ? 'Çalışan yönlendirici yeni ayarı kullanıyor.' : 'The running forwarder is using the new setting.')
              : (tr ? 'Ayar kaydedildi; yönlendirici başlatıldığında kullanılacak.' : 'Saved; it will be used when the forwarder starts.');
            get().appendLog(
              tr
                ? `[DNS] Yapılandırma #${verified.configRevision} kabul edildi: ${verified.protocol.toUpperCase()}, önbellek ${verified.cache ? 'açık' : 'kapalı'}, reklam filtresi ${verified.adblock ? 'açık' : 'kapalı'}. ${activeText}`
                : `[DNS] Configuration #${verified.configRevision} accepted: ${verified.protocol.toUpperCase()}, cache ${verified.cache ? 'on' : 'off'}, ad filter ${verified.adblock ? 'on' : 'off'}. ${activeText}`,
              'info',
            );
          }).catch(cause => {
            if (revision !== dnsSyncRevision) return;
            const error = normalizeIpcError(cause);
            const rollback = pendingDnsRollback;
            pendingDnsRollback = {};
            if (Object.keys(rollback).length > 0) set(rollback);
            const tr = get().language === 'tr';
            get().appendLog(
              tr
                ? `[ERROR] DNS ayarı doğrulanamadı; arayüz son doğrulanmış değere geri döndü: ${error.message} (${error.code})`
                : `[ERROR] DNS setting could not be verified; the UI reverted to the last verified value: ${error.message} (${error.code})`,
              'error',
            );
          });
        }, 100);
      },

      appendLog: (content, level = 'info') => set((state) => {
        const newLine: LogLine = {
          id: String(++logCounter),
          timestamp: new Date(),
          content,
          level,
        };
        return { logs: [newLine, ...state.logs].slice(0, 500) };
      }),
      appendLogs: (entries) => set((state) => {
        const newLines = entries.map(e => ({
          id: String(++logCounter),
          timestamp: new Date(),
          content: e.content,
          level: e.level,
        })).reverse(); // En yeni log en üstte (index 0) olsun diye.
        
        return { logs: [...newLines, ...state.logs].slice(0, 500) };
      }),

      setDnsProviders: (dnsProviders) => set({ dnsProviders }),
      setSelectedDnsId: (selectedDnsId) => set({ selectedDnsId }),
      setDnsCustom: (dnsCustomPrimary, dnsCustomSecondary) => set({ dnsCustomPrimary, dnsCustomSecondary }),
      setDnsSynced: (dnsSynced) => set({ dnsSynced }),

      refreshPresets: async () => {
        try {
          const fetched = await invoke<Preset[]>('list_presets');
          set({ presets: fetched });
        } catch (err) {
          console.error('Preset listesi çekilemedi:', err);
        }
      },

      deletePreset: async (presetId: string) => {
        try {
          await invoke('delete_custom_preset', { presetId });
          await get().refreshPresets();
          if (get().activePresetId === presetId) {
            await get().setActivePreset('default');
          }
        } catch (err) {
          console.error("Preset silinemedi:", err);
          get().appendLog(`[ERROR] Preset deletion failed: ${err}`, 'error');
        }
      },

      startEngine: async (presetId) => {
        const token = ++lifecycleToken;
        const id = presetId || get().activePresetId;
        if (!id) return;

        // Seçilen preseti kalıcı olarak kaydet (persist aracılığıyla)
        set({ activePresetId: id, status: { variant: 'starting' } });
        get().appendLog(get().language === 'tr' ? `[ENGINE] “${id}” profiliyle DPI bypass başlatılıyor...` : `[ENGINE] Starting DPI bypass with profile “${id}”...`, 'info');

        try {
          ++bypassSyncRevision;
          ++dnsSyncRevision;
          if (bypassSyncTimeout) clearTimeout(bypassSyncTimeout);
          if (dnsSyncTimeout) clearTimeout(dnsSyncTimeout);
          const current = get();
          const wl = Array.isArray(current.whitelistDomains) ? current.whitelistDomains : [];
          const bl = Array.isArray(current.blacklistDomains) ? current.blacklistDomains : [];
          const activeDomains = activePatternDomains(current.bypassMode, wl, bl);
          await invoke<BypassConfigStatus>('sync_bypass_config', {
            mode: current.bypassMode,
            list: activeDomains.join('\n'),
            proxy: current.proxySocks5,
            killSwitch: current.killSwitch,
            whitelistDomains: wl,
            blacklistDomains: bl,
            activePresetId: current.activePresetId,
          });
          await invoke<DnsConfigStatus>('sync_dns_settings', {
            protocol: current.dnsProtocol,
            adblock: current.dnsAdBlock,
            cache: current.dnsCache,
            socks5Proxy: current.proxySocks5,
            healthCheckTargets: current.healthCheckTargets,
            emitEvent: true,
            enabled: current.dnsForwarderEnabled,
          });
          pendingDnsRollback = {};
          if (current.dnsForwarderEnabled) {
            const forwarder = await invoke<{ active: boolean }>('get_doh_forwarder_status');
            if (!forwarder.active) {
              await invoke('start_doh_forwarder', { watchdog: current.watchdog });
            }
          }
          const result = await invoke<EngineStatus>('start_engine_with_dns_guard', { presetId: id });
          if (token !== lifecycleToken) return;
          set({ status: result });

          if (result.variant === 'ready') {
            get().appendLog(get().language === 'tr' ? `[ENGINE] DPI bypass etkin ve çalışıyor (işlem ${result.pid}).` : `[ENGINE] DPI bypass is active and running (process ${result.pid}).`, 'info');
          } else if (result.variant === 'error') {
            get().appendLog(get().language === 'tr' ? `[ERROR] DPI motoru hatası: ${result.message}` : `[ERROR] DPI engine error: ${result.message}`, 'error');
          }
        } catch (err) {
          if (token !== lifecycleToken) return;
          const rollback = pendingDnsRollback;
          pendingDnsRollback = {};
          if (Object.keys(rollback).length > 0) set(rollback);
          const error = normalizeIpcError(err);
          set({ status: { variant: 'error', message: error.message, code: error.code } });
          get().appendLog(get().language === 'tr' ? `[ERROR] DPI bypass başlatılamadı: ${error.message}` : `[ERROR] DPI bypass could not start: ${error.message}`, 'error');
        }
      },

      stopEngine: async () => {
        const token = ++lifecycleToken;
        try {
          await invoke('stop_engine');
          if (token !== lifecycleToken) return;
          set({ status: { variant: 'stopped' } });
          get().appendLog(get().language === 'tr' ? '[ENGINE] DPI bypass durduruldu.' : '[ENGINE] DPI bypass stopped.', 'warn');
        } catch (err) {
          console.error('Durdurma hatası:', err);
        }
      },

      refreshDnsStatus: async () => {
        try {
          const [provs, forwarder] = await Promise.all([
            invoke<DnsProvider[]>('list_dns_providers'),
            invoke<{ active: boolean }>('get_doh_forwarder_status'),
          ]);

          set({ dnsProviders: provs, dnsForwarderEnabled: forwarder.active });
          if (forwarder.active) {
            set({ dnsSynced: true });
            return;
          }

          const adaps = await invoke<NetworkAdapter[]>('get_network_adapters').catch(() => []);

          // Statik DNS ayarlı adaptörü tercih et, yoksa ilkini al
          const staticAdapter = adaps.find((a) => !a.isDhcp);
          const activeDns = staticAdapter?.currentPrimaryDns ?? adaps[0]?.currentPrimaryDns;
          const desiredId = get().selectedDnsId;
          if (desiredId) {
            const desiredPrimary = desiredId === 'custom'
              ? get().dnsCustomPrimary
              : provs.find((provider) => provider.id === desiredId)?.primary;
            set({ dnsSynced: Boolean(activeDns && desiredPrimary && activeDns === desiredPrimary) });
            return;
          }

          if (activeDns) {
            const match = provs.find((p) => p.primary === activeDns);
            if (match) {
              set({ selectedDnsId: match.id, dnsSynced: true });
            } else {
              const secondaryAdapter = adaps.find((a) => a.currentPrimaryDns === activeDns);
              set({
                selectedDnsId: 'custom',
                dnsCustomPrimary: activeDns,
                dnsCustomSecondary: secondaryAdapter?.currentSecondaryDns ?? '',
                dnsSynced: true,
              });
            }
          } else {
            set({ dnsSynced: true });
          }
        } catch (err) {
          console.error('DNS Sync Hatası:', err);
        }
      },
    }),
    {
      name: 'vane-settings',           // Rust settings repository key
      storage: createJSONStorage(createTauriStorage), // Atomic Rust persistence adapter
      version: 1,
      migrate: (persistedState: unknown) =>
        migratePersistedEngineState(persistedState) as unknown as EngineStore,
      onRehydrateStorage: () => (state, error) => {
        if (error) {
          useEngineStore.setState({
            hasHydrated: true,
            persistenceError: String(error),
          });
          return;
        }
        if (state) {
          let dirty = false;
          let wl = state.whitelistDomains;
          let bl = state.blacklistDomains;
          
          if (typeof wl === 'string') {
            wl = (wl as string).split('\n').map(d => d.trim()).filter(d => d.length > 0);
            dirty = true;
          } else if (!Array.isArray(wl)) {
            wl = [];
            dirty = true;
          }

          if (typeof bl === 'string') {
            bl = (bl as string).split('\n').map(d => d.trim()).filter(d => d.length > 0);
            dirty = true;
          } else if (!Array.isArray(bl)) {
            bl = [];
            dirty = true;
          }

          if (dirty) {
            state.whitelistDomains = wl;
            state.blacklistDomains = bl;
          }

          state.setHasHydrated(true);
          state.syncBypassToBackend();
          state.syncDnsToBackend();
        }
      },
      // Sadece bu alanlar diske yazılır; session verileri (logs, status vb.) yazılmaz.
      partialize: (state) => ({
        activePresetId: state.activePresetId,
        selectedDnsId: state.selectedDnsId,
        dnsCustomPrimary: state.dnsCustomPrimary,
        dnsCustomSecondary: state.dnsCustomSecondary,
        advancedConfig: state.advancedConfig,
        healthCheckTargets: state.healthCheckTargets,
        bypassMode: state.bypassMode,
        domainList: state.domainList,
        whitelistDomains: state.whitelistDomains,
        blacklistDomains: state.blacklistDomains,
        dnsProtocol: state.dnsProtocol,
        dnsAdBlock: state.dnsAdBlock,
        dnsCache: state.dnsCache,
        proxySocks5: state.proxySocks5,
        killSwitch: state.killSwitch,
        watchdog: state.watchdog,
        dnsForwarderEnabled: state.dnsForwarderEnabled,
        language: state.language,
      }),
    }
  )
);
