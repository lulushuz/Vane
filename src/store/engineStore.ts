import { create } from 'zustand';
import { persist, createJSONStorage, type StateStorage } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { load } from '@tauri-apps/plugin-store';
import type { NetworkAdapter } from '../types/network';

export type EngineStatus =
  | { variant: 'stopped' }
  | { variant: 'starting' }
  | { variant: 'running'; pid: number }
  | { variant: 'error'; message: string; code?: string };

export interface Preset {
  id: string;
  label: string;
  description: string;
  icon: string;
  args: string[];
  isCustom: boolean;
  priority?: number;
  category?: string;
}

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

interface BypassConfigStatus {
  mode: 'all' | 'whitelist' | 'blacklist';
  domainCount: number;
  engineRestarted: boolean;
  engineRunning: boolean;
  whitelistDomains: string[];
  blacklistDomains: string[];
}

export interface DnsConfigStatus {
  protocol: 'doh' | 'dot';
  adblock: boolean;
  cache: boolean;
  socks5Proxy: string;
  forwarderActive: boolean;
}

/* 
   Advanced Config
   Gelişmiş ayarların tek obje olarak tutulduğu şema.
   Tüm alanlar persist edilerek settings.json dosyasına yazılır. 
*/
export interface AdvancedConfig {
  // DPI Desynchronization
  desyncMethod: string;       // 'split' | 'split2' | 'disorder' | 'fake' | 'oob' | 'custom' | 'none'
  customDesyncMethod: string; // desyncMethod === 'custom' ise bu kullanılır
  splitPosition: number;      // --dpi-desync-split-pos
  desyncRepeats: number;      // --dpi-desync-repeats
  desyncFooling: string[];    // --dpi-desync-fooling
  anyProtocol: boolean;       // --dpi-desync-any-protocol

  // Packet & Traffic
  autoTtl: boolean;           // --dpi-desync-autottl
  fakeTtl: number;            // --dpi-desync-ttl
  mssFix: number;             // --mss

  // Protocol & Ports
  quicUdpHandling: boolean;   // --wf-udp=443
  httpPorts: string;          // --wf-tcp=

  // --- NEW ZAPRET FIELDS ---
  desyncHttp: string;         // --dpi-desync-http
  desyncHttps: string;        // --dpi-desync-https
  desyncQuic: string;         // --dpi-desync-quic
  desyncCutoff: string;       // --dpi-desync-cutoff
  splitHttpReq: string;       // --dpi-desync-split-http-req (none, method, host)
  splitPosHttpReq: number;    // --dpi-desync-split-pos-http-req
  splitTls: string;           // --dpi-desync-split-tls (none, sni, snh)
  splitPosTls: number;        // --dpi-desync-split-pos-tls
  fakeTtlExt: number;         // --dpi-desync-ttl-ext
  fakeTlsSni: string;         // --dpi-desync-fake-tls-sni
  fakeHttpPayload: string;    // --dpi-desync-fake-http (string/filepath)
  fakeTlsPayload: string;     // --dpi-desync-fake-tls (string/filepath)
  fakeQuicPayload: string;    // --dpi-desync-fake-quic (string/filepath)
  desync2: string;            // --dpi-desync2
  tcpWindowSize: number;      // --tcp-window-size
  ipsetPath: string;          // --ipset
  tpwsMode: boolean;          // Runs tpws instead of nfqws/winws
  bindInterface: string;      // --bind-addr
}

export const DEFAULT_ADVANCED_CONFIG: AdvancedConfig = {
  desyncMethod: 'custom',
  customDesyncMethod: 'fake,multidisorder',
  splitPosition: 1,
  desyncRepeats: 1,
  desyncFooling: ['badseq'],
  anyProtocol: true,
  autoTtl: true,
  fakeTtl: 4,
  mssFix: 1300,
  quicUdpHandling: true,
  httpPorts: '80, 443',
  desyncHttp: 'none',
  desyncHttps: 'none',
  desyncQuic: 'none',
  desyncCutoff: 'd3',
  splitHttpReq: 'none',
  splitPosHttpReq: 0,
  splitTls: 'none',
  splitPosTls: 0,
  fakeTtlExt: 0,
  fakeTlsSni: '',
  fakeHttpPayload: '',
  fakeTlsPayload: '',
  fakeQuicPayload: '',
  desync2: 'none',
  tcpWindowSize: 0,
  ipsetPath: '',
  tpwsMode: false,
  bindInterface: '',
};

/* 
   Tauri Store Adapter
   Zustand'ın persist middleware'i için tauri-plugin-store'u depolama motoru
   olarak bağlar. Böylece veriler AppData/com.vane.dpi/settings.json dosyasına
   yazılır — PC yeniden başlasa bile korunur. 
*/

const STORE_FILE = 'settings.json';

function createTauriStorage(): StateStorage {
  return {
    getItem: async (key: string): Promise<string | null> => {
      try {
        const store = await load(STORE_FILE, { autoSave: false } as any);
        const value = await store.get<string>(key);
        return value ?? null;
      } catch {
        return null;
      }
    },
    setItem: async (key: string, value: string): Promise<void> => {
      try {
        const store = await load(STORE_FILE, { autoSave: false } as any);
        await store.set(key, value);
        await store.save();
      } catch (e) {
        console.error('Tauri Store yazma hatası:', e);
      }
    },
    removeItem: async (key: string): Promise<void> => {
      try {
        const store = await load(STORE_FILE, { autoSave: false } as any);
        await store.delete(key);
        await store.save();
      } catch { /* ignore */ }
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
  dnsProtocol: 'doh' | 'dot' | 'doq';
  dnsAdBlock: boolean;
  dnsCache: boolean;
  proxySocks5: string;
  killSwitch: boolean;
  watchdog: boolean;
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
  setDnsProtocol: (protocol: 'doh' | 'dot' | 'doq') => void;
  setDnsAdBlock: (enabled: boolean) => void;
  setDnsCache: (enabled: boolean) => void;
  setProxySocks5: (addr: string) => void;
  setKillSwitch: (enabled: boolean) => void;
  setWatchdog: (enabled: boolean) => void;
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
  setHasHydrated: (val: boolean) => void;
}

let logCounter = 0;
let bypassSyncTimeout: any = null;
let dnsSyncTimeout: any = null;

const DOMAIN_ALIASES: Record<string, string[]> = {
  'discord.com': ['discordapp.com', 'discordapp.net', 'discord.gg'],
  'roblox.com': ['robloxlabs.com', 'rbxcdn.com'],
  'youtube.com': ['youtu.be', 'ytimg.com', 'ggpht.com']
};

const VALID_DOMAIN = /^(?:\*\.)?[a-zA-Z0-9][-a-zA-Z0-9]{0,62}(?:\.[a-zA-Z0-9][-a-zA-Z0-9]{0,62})+$/;

const cleanDomainsHelper = (domains: string[]): string => {
  const validDomains = domains.map((domain) => domain.trim().toLowerCase()).filter((domain) => VALID_DOMAIN.test(domain));
  const resultSet = new Set<string>(validDomains);
  for (const domain of validDomains) {
    const cleanDomain = domain.replace(/^\*\./, '');
    if (DOMAIN_ALIASES[cleanDomain]) {
      for (const alias of DOMAIN_ALIASES[cleanDomain]) {
        resultSet.add(alias);
        resultSet.add(`*.${alias}`);
      }
    }
  }
  return Array.from(resultSet).join('\n');
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
      language: 'en',

      // Session değerleri (persist edilmez)
      status: { variant: 'stopped' },
      presets: [],
      logs: [],
      activeTab: 'home',
      dnsProviders: [],
      dnsSynced: false,
      advancedDirty: false,
      healthCheckTargets: ['discord.com'],
      hasHydrated: false,

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
      setHealthCheckTargets: (targets) => set({ healthCheckTargets: targets }),
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
        set({ dnsProtocol });
        get().syncDnsToBackend();
      },
      setDnsAdBlock: (dnsAdBlock) => {
        set({ dnsAdBlock });
        get().syncDnsToBackend();
      },
      setDnsCache: (dnsCache) => {
        set({ dnsCache });
        get().syncDnsToBackend();
      },
      setProxySocks5: (proxySocks5) => {
        set({ proxySocks5 });
        get().syncBypassToBackend();
        get().syncDnsToBackend();
      },
      setKillSwitch: (killSwitch) => {
        set({ killSwitch });
        get().syncBypassToBackend();
      },
      setWatchdog: (watchdog) => set({ watchdog }),
      setHasHydrated: (hasHydrated) => set({ hasHydrated }),
      setLanguage: async (language) => {
        set({ language });
        try {
          await emit('sync_language', language);
        } catch (err) { /* ignore */ }
      },
      syncBypassToBackend: () => {
        if (bypassSyncTimeout) clearTimeout(bypassSyncTimeout);
        bypassSyncTimeout = setTimeout(() => {
          const state = get();
          
          const activeDomains = state.bypassMode === 'whitelist' ? state.whitelistDomains : state.blacklistDomains;
          const cleanedList = cleanDomainsHelper(activeDomains);
          
          set({ domainList: cleanedList });

          invoke<BypassConfigStatus>('sync_bypass_config', {
            mode: state.bypassMode,
            list: cleanedList,
            proxy: state.proxySocks5,
            killSwitch: state.killSwitch,
            whitelistDomains: state.whitelistDomains,
            blacklistDomains: state.blacklistDomains,
          }).then((verified) => {
            const tr = get().language === 'tr';
            const modeText = tr
              ? ({ all: 'tüm siteler', whitelist: 'yalnızca beyaz liste', blacklist: 'kara liste hariç' } as const)[verified.mode]
              : ({ all: 'all sites', whitelist: 'whitelist only', blacklist: 'except blacklist' } as const)[verified.mode];
            const applyText = verified.engineRestarted
              ? (tr ? 'Çalışan motor yeni kurallarla yeniden başlatıldı.' : 'The running engine was restarted with the new rules.')
              : (tr ? 'Kural bir sonraki motor başlangıcında uygulanacak.' : 'The rule will be applied on the next engine start.');
            get().appendLog(
              tr
                ? `[PATTERN] Ayar doğrulandı: ${modeText}; ${verified.domainCount} alan adı. ${applyText}`
                : `[PATTERN] Setting verified: ${modeText}; ${verified.domainCount} domains. ${applyText}`,
              'info',
            );
          }).catch(err => {
            const tr = get().language === 'tr';
            get().appendLog(tr ? `[ERROR] Desen ayarı uygulanamadı: ${err}` : `[ERROR] Pattern setting could not be applied: ${err}`, 'error');
          });
        }, 100);
      },
      syncDnsToBackend: () => {
        if (dnsSyncTimeout) clearTimeout(dnsSyncTimeout);
        dnsSyncTimeout = setTimeout(() => {
          const state = get();
          const protocol = state.dnsProtocol === 'doq' ? 'doh' : state.dnsProtocol;
          if (state.dnsProtocol === 'doq') set({ dnsProtocol: 'doh' });
          invoke<DnsConfigStatus>('sync_dns_settings', {
            protocol,
            adblock: state.dnsAdBlock,
            cache: state.dnsCache,
            socks5Proxy: state.proxySocks5,
          }).then((verified) => {
            const tr = get().language === 'tr';
            const activeText = verified.forwarderActive
              ? (tr ? 'Çalışan yönlendirici yeni ayarı kullanıyor.' : 'The running forwarder is using the new setting.')
              : (tr ? 'Ayar kaydedildi; yönlendirici başlatıldığında kullanılacak.' : 'Saved; it will be used when the forwarder starts.');
            get().appendLog(
              tr
                ? `[DNS] Ayarlar doğrulandı: ${verified.protocol.toUpperCase()}, önbellek ${verified.cache ? 'açık' : 'kapalı'}, reklam filtresi ${verified.adblock ? 'açık' : 'kapalı'}. ${activeText}`
                : `[DNS] Settings verified: ${verified.protocol.toUpperCase()}, cache ${verified.cache ? 'on' : 'off'}, ad filter ${verified.adblock ? 'on' : 'off'}. ${activeText}`,
              'info',
            );
          }).catch(err => {
            const tr = get().language === 'tr';
            get().appendLog(tr ? `[ERROR] DNS ayarı doğrulanamadı: ${err}` : `[ERROR] DNS setting could not be verified: ${err}`, 'error');
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
        const id = presetId || get().activePresetId;
        if (!id) return;

        // Seçilen preseti kalıcı olarak kaydet (persist aracılığıyla)
        set({ activePresetId: id, status: { variant: 'starting' } });
        get().appendLog(get().language === 'tr' ? `[ENGINE] “${id}” profiliyle DPI bypass başlatılıyor...` : `[ENGINE] Starting DPI bypass with profile “${id}”...`, 'info');

        try {
          if (bypassSyncTimeout) clearTimeout(bypassSyncTimeout);
          if (dnsSyncTimeout) clearTimeout(dnsSyncTimeout);
          const current = get();
          const activeDomains = current.bypassMode === 'whitelist' ? current.whitelistDomains : current.blacklistDomains;
          await invoke<BypassConfigStatus>('sync_bypass_config', {
            mode: current.bypassMode,
            list: cleanDomainsHelper(activeDomains),
            proxy: current.proxySocks5,
            killSwitch: current.killSwitch,
            whitelistDomains: current.whitelistDomains,
            blacklistDomains: current.blacklistDomains,
          });
          await invoke<DnsConfigStatus>('sync_dns_settings', {
            protocol: current.dnsProtocol === 'doq' ? 'doh' : current.dnsProtocol,
            adblock: current.dnsAdBlock,
            cache: current.dnsCache,
            socks5Proxy: current.proxySocks5,
          });
          const result = await invoke<EngineStatus>('start_engine_with_dns_guard', { presetId: id });
          set({ status: result });

          if (result.variant === 'running') {
            get().appendLog(get().language === 'tr' ? `[ENGINE] DPI bypass etkin ve çalışıyor (işlem ${result.pid}).` : `[ENGINE] DPI bypass is active and running (process ${result.pid}).`, 'info');
          } else if (result.variant === 'error') {
            get().appendLog(get().language === 'tr' ? `[ERROR] DPI motoru hatası: ${result.message}` : `[ERROR] DPI engine error: ${result.message}`, 'error');
          }
        } catch (err: any) {
          const errorCode = typeof err === 'object' && err !== null && 'code' in err ? err.code : 'UNKNOWN';
          const errorMsg = typeof err === 'object' && err !== null && 'message' in err ? err.message : String(err);
          set({ status: { variant: 'error', message: errorMsg, code: errorCode } });
          get().appendLog(get().language === 'tr' ? `[ERROR] DPI bypass başlatılamadı: ${errorMsg}` : `[ERROR] DPI bypass could not start: ${errorMsg}`, 'error');
        }
      },

      stopEngine: async () => {
        try {
          await invoke('stop_engine');
          set({ status: { variant: 'stopped' } });
          get().appendLog(get().language === 'tr' ? '[ENGINE] DPI bypass durduruldu.' : '[ENGINE] DPI bypass stopped.', 'warn');
        } catch (err) {
          console.error('Durdurma hatası:', err);
        }
      },

      refreshDnsStatus: async () => {
        try {
          const [provs, adaps] = await Promise.all([
            invoke<DnsProvider[]>('list_dns_providers'),
            invoke<NetworkAdapter[]>('get_network_adapters'),
          ]);

          set({ dnsProviders: provs });

          // Statik DNS ayarlı adaptörü tercih et, yoksa ilkini al
          const staticAdapter = adaps.find((a) => !a.isDhcp);
          const activeDns = staticAdapter?.currentPrimaryDns ?? adaps[0]?.currentPrimaryDns;

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
      name: 'vane-settings',           // Tauri Store'daki anahtar adı
      storage: createJSONStorage(createTauriStorage), // Depolama motoru
      onRehydrateStorage: () => (state) => {
        if (state) {
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
        language: state.language,
      }),
    }
  )
);
