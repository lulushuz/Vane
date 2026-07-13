import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useEngineStore } from '../store/engineStore';
import type { EngineStatus } from '../types/engine';
import type { DnsConfigStatus } from '../store/engineStore';

interface BypassConfigStatus {
  mode: 'all' | 'whitelist' | 'blacklist';
  whitelistDomains: string[];
  blacklistDomains: string[];
}

/* 
   Central hook that registers all Tauri backend event listeners.
   Each window (Widget + Settings) sets up its own listener instance.
   The cleanup function unregisters all listeners on unmount to prevent leaks. 
*/
export function useEventListeners(): void {
  const { appendLog, appendLogs, setStatus, refreshDnsStatus, refreshPresets } = useEngineStore();

  useEffect(() => {
    let isMounted = true;
    const cleanupFns: Array<() => void> = [];

    const register = async <T>(
      event: string,
      handler: (payload: T) => void,
    ) => {
      const unlisten = await listen<T>(event, (e) => handler(e.payload));
      if (!isMounted) {
        unlisten();
      } else {
        cleanupFns.push(unlisten);
      }
    };

    // winws stdout/stderr lines forwarded from the backend in batches
    register<string[]>('log_batch', (lines) => {
      if (lines.length === 0) return;
      const language = useEngineStore.getState().language;
      const entries = lines.map(line => ({
        content: makeUnderstandable(line, language),
        level: classifyLogLevel(line)
      }));
      appendLogs(entries);
    });

    // Engine lifecycle changes (stopped / starting / running / error)
    register<EngineStatus>('engine_status', (status) => {
      setStatus(status);
    });

    // Emitted by apply_dns_settings, reset_dns_settings, start_engine_with_dns_guard
    register<void>('dns_status_changed', () => {
      refreshDnsStatus();
    });

    // Emitted when DNS Guard auto-applies Cloudflare on engine start
    register<string>('dns_auto_applied', (message) => {
      appendLog(`[DNS] ${message}`, 'warn');
    });

    // WM_DEVICECHANGE fired by network/watcher.rs on adapter changes
    register<void>('network_changed', () => {
      appendLog('[SYSTEM] Network change detected — refreshing DNS status...', 'warn');
      refreshDnsStatus();
    });

    // Keeps activePresetId in sync across the Widget and Settings windows
    register<string>('sync_active_preset', (presetId) => {
      useEngineStore.setState({ activePresetId: presetId });
    });

    // Keeps language in sync across windows
    register<'tr' | 'en'>('sync_language', (lang) => {
      useEngineStore.setState({ language: lang });
    });

    // Keep persisted settings identical in both the widget and settings windows.
    register<BypassConfigStatus>('bypass_config_synced', (config) => {
      useEngineStore.setState({
        bypassMode: config.mode,
        whitelistDomains: config.whitelistDomains,
        blacklistDomains: config.blacklistDomains,
      });
    });

    register<DnsConfigStatus>('dns_config_synced', (config) => {
      useEngineStore.setState({
        dnsProtocol: config.protocol,
        dnsAdBlock: config.adblock,
        dnsCache: config.cache,
        proxySocks5: config.socks5Proxy,
      });
    });

    return () => {
      isMounted = false;
      cleanupFns.forEach((fn) => fn());
    };
  // Zustand action references are stable — safe to list as deps.
  // Note: startEngine is intentionally excluded; it is never used inside this hook.
  }, [appendLog, appendLogs, setStatus, refreshDnsStatus, refreshPresets]);
}

function makeUnderstandable(content: string, language: 'tr' | 'en'): string {
  const tr = language === 'tr';
  const clean = content.replace(/^\[([^\]]+)]\s*"(.*)"$/, '[$1] $2');
  let match = clean.match(/Engine started: preset='([^']+)', pid=(\d+)/i);
  if (match) return tr
    ? `[ENGINE] DPI bypass etkin. “${match[1]}” profili başarıyla çalışıyor (işlem ${match[2]}).`
    : `[ENGINE] DPI bypass is active. Profile “${match[1]}” is running successfully (process ${match[2]}).`;
  match = clean.match(/Engine process spawned successfully, PID: (\d+)/i);
  if (match) return tr
    ? `[ENGINE] Bypass işlemi Windows tarafından başlatıldı (işlem ${match[1]}).`
    : `[ENGINE] The bypass process was started by Windows (process ${match[1]}).`;
  if (/Engine stopped\.?/i.test(clean)) return tr
    ? '[ENGINE] DPI bypass tamamen durduruldu.'
    : '[ENGINE] DPI bypass stopped completely.';
  if (/Pattern verified: DPI bypass will run for all sites/i.test(clean)) return tr
    ? '[PATTERN] Doğrulandı: DPI bypass tüm sitelere uygulanıyor.'
    : '[PATTERN] Verified: DPI bypass is applied to all sites.';
  match = clean.match(/Pattern verified: DPI bypass will run only for (\d+) whitelisted domains/i);
  if (match) return tr
    ? `[PATTERN] Doğrulandı: DPI bypass yalnızca beyaz listedeki ${match[1]} alan adına uygulanıyor.`
    : `[PATTERN] Verified: DPI bypass is applied only to ${match[1]} whitelisted domains.`;
  match = clean.match(/Pattern verified: (\d+) blacklisted domains will be excluded/i);
  if (match) return tr
    ? `[PATTERN] Doğrulandı: Kara listedeki ${match[1]} alan adı DPI bypass dışında tutuluyor.`
    : `[PATTERN] Verified: ${match[1]} blacklisted domains are excluded from DPI bypass.`;
  if (/DNS forwarder is running and verified/i.test(clean)) return tr
    ? clean.replace(/^\[[^\]]+]/, '[DNS]').replace('DNS forwarder is running and verified:', 'DNS yönlendiricisi çalışıyor ve doğrulandı:')
    : clean.replace(/^\[[^\]]+]/, '[DNS]');
  if (/DNS forwarder stopped and system DNS was restored/i.test(clean)) return tr
    ? '[DNS] DNS yönlendiricisi durduruldu; sistem DNS ayarı otomatik olarak geri yüklendi.'
    : '[DNS] DNS forwarder stopped; the system DNS setting was restored automatically.';
  match = clean.match(/DNS settings applied and verified: protocol=([^,]+), cache=(true|false), adblock=(true|false), proxy=(.+)/i);
  if (match) return tr
    ? `[DNS] Çalışma ayarı doğrulandı: protokol ${match[1]}, önbellek ${match[2] === 'true' ? 'açık' : 'kapalı'}, reklam filtresi ${match[3] === 'true' ? 'açık' : 'kapalı'}, bağlantı ${match[4] === 'direct' ? 'doğrudan' : match[4]}.`
    : `[DNS] Runtime setting verified: protocol ${match[1]}, cache ${match[2] === 'true' ? 'on' : 'off'}, ad filter ${match[3] === 'true' ? 'on' : 'off'}, connection ${match[4]}.`;
  match = clean.match(/Bypass pattern saved and verified: mode=([^,]+), domains=(\d+), engine_restarted=(true|false)/i);
  if (match) return tr
    ? `[PATTERN] Desen kaydedildi ve doğrulandı: mod ${match[1]}, ${match[2]} alan adı, motor ${match[3] === 'true' ? 'yeniden başlatıldı' : 'çalışmıyor'}.`
    : `[PATTERN] Pattern saved and verified: mode ${match[1]}, ${match[2]} domains, engine ${match[3] === 'true' ? 'restarted' : 'not running'}.`;
  if (/Bypass config changed while engine is running/i.test(clean)) return tr
    ? '[PATTERN] Desen değişti; çalışan motor yeni kuralı kullanmak üzere yeniden başlatılıyor.'
    : '[PATTERN] Pattern changed; the running engine is restarting with the new rule.';
  return clean;
}

// Infers a log level from the message content.
function classifyLogLevel(content: string): 'info' | 'warn' | 'error' {
  const lower = content.toLowerCase();
  if (lower.includes('error') || lower.includes('fail') || lower.includes('hata') || lower.includes('başarısız')) return 'error';
  if (lower.includes('warn') || lower.includes('stderr') || lower.includes('uyarı')) return 'warn';
  return 'info';
}
