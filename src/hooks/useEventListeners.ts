import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useEngineStore } from '../store/engineStore';
import type { EngineStatus } from '../types/engine';
import type { BypassConfigStatus, DnsConfigStatus } from '../store/engineStore';
import { MonotonicRevisionGate } from '../store/revisionGate';

const bypassRevisionGate = new MonotonicRevisionGate();

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
      const tr = useEngineStore.getState().language === 'tr';
      appendLog(
        message === 'DNS_WATCHDOG_PREVIOUS_CONFIGURATION_RESTORED'
          ? (tr
            ? '[DNS] Üst DNS bağlantısı kesildi. İnternet erişimini kurtarmak için önceki adaptör DNS ayarları otomatik olarak geri yüklendi.'
            : '[DNS] The upstream resolver failed. The previous adapter DNS settings were restored automatically to recover connectivity.')
          : `[DNS] ${message}`,
        'warn',
      );
    });

    // WM_DEVICECHANGE fired by network/watcher.rs on adapter changes
    register<void>('network_changed', () => {
      appendLog(
        useEngineStore.getState().language === 'tr'
          ? '[SYSTEM] Ağ değişikliği algılandı; DNS durumu yeniden doğrulanıyor...'
          : '[SYSTEM] Network change detected; DNS status is being verified again...',
        'warn',
      );
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
      if (!bypassRevisionGate.accept(config.configRevision)) return;
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
  if (/previous DNS forwarder shutdown was incomplete.*restored and verified/i.test(clean)) return tr
    ? '[DNS] Önceki çalışmada DNS yönlendiricisi düzgün kapanmamış. Kaydedilen adaptör DNS ayarları açılışta geri yüklendi ve doğrulandı.'
    : '[DNS] The previous DNS forwarder session did not close cleanly. Saved adapter DNS settings were restored and verified at startup.';
  if (/System DNS was restored successfully after the upstream failure/i.test(clean)) return tr
    ? '[DNS] Üst DNS bağlantısı kesildi; bağlantıyı korumak için önceki sistem DNS ayarı başarıyla geri yüklendi.'
    : '[DNS] The upstream resolver failed; the previous system DNS setting was restored successfully to preserve connectivity.';
  if (/DNS settings were already active; no forwarder restart was needed/i.test(clean)) return tr
    ? '[DNS] Seçilen ayarlar zaten çalışıyordu; DNS yönlendiricisi gereksiz yere yeniden başlatılmadı.'
    : '[DNS] The selected settings were already active; no unnecessary forwarder restart was performed.';
  if (/Running DNS forwarder was restarted to verify the changed settings/i.test(clean)) return tr
    ? '[DNS] Değişen ayarların gerçekten kullanılması için çalışan DNS yönlendiricisi yeniden başlatıldı ve doğrulandı.'
    : '[DNS] The running DNS forwarder was restarted and verified so the changed settings are actually in use.';
  if (/DNS forwarder was started before applying Kill Switch/i.test(clean)) return tr
    ? '[SECURITY] DNS Kill Switch uygulanmadan önce şifreli yerel DNS yönlendiricisi başlatıldı.'
    : '[SECURITY] The encrypted local DNS forwarder was started before DNS Kill Switch was applied.';
  if (/Settings primary was (missing|damaged); using the last-known-good backup/i.test(clean)) return tr
    ? '[SYSTEM] Ana ayar dosyası okunamadı; kullanıcı ayarlarının sıfırlanmaması için son sağlam yedek kullanıldı.'
    : '[SYSTEM] The primary settings file was unreadable; the last-known-good backup was used instead of resetting user preferences.';
  match = clean.match(/DNS settings applied and verified: protocol=([^,]+), cache=(true|false), adblock=(true|false), proxy=([^,]+), health_target=(.+)/i);
  if (match) return tr
    ? `[DNS] Çalışma ayarı doğrulandı: protokol ${match[1]}, önbellek ${match[2] === 'true' ? 'açık' : 'kapalı'}, reklam/kötü amaçlı alan filtresi ${match[3] === 'true' ? 'açık' : 'kapalı'}, bağlantı ${match[4] === 'direct' ? 'doğrudan' : match[4]}, sağlık testi ${match[5]}.`
    : `[DNS] Runtime setting verified: protocol ${match[1]}, cache ${match[2] === 'true' ? 'on' : 'off'}, ad/malware domain filter ${match[3] === 'true' ? 'on' : 'off'}, connection ${match[4]}, health target ${match[5]}.`;
  if (/Smart DNS Cache is disabled and all RAM cache entries were cleared/i.test(clean)) return tr
    ? '[DNS] Smart DNS Cache gerçekten kapatıldı ve bellekteki tüm kayıtlar temizlendi.'
    : '[DNS] Smart DNS Cache was disabled and every in-memory entry was cleared.';
  if (/Smart DNS Cache is enabled with TTL aging/i.test(clean)) return tr
    ? '[DNS] Smart DNS Cache etkin: TTL süreleri yaşlandırılıyor ve en eski kullanılmayan kayıtlar kontrollü temizleniyor.'
    : '[DNS] Smart DNS Cache is active with TTL aging and controlled least-recently-used eviction.';
  if (/SOCKS5H client created/i.test(clean)) return tr
    ? '[DNS] SOCKS5H upstream doğrulandı; DNS sunucusu adı proxy üzerinden çözümlenecek ve doğrudan bağlantıya sessiz geçiş yapılmayacak.'
    : '[DNS] SOCKS5H upstream verified; resolver hostnames will be resolved through the proxy with no silent direct fallback.';
  match = clean.match(/Bypass pattern saved and verified: mode=([^,]+), domains=(\d+), engine_restarted=(true|false)/i);
  if (match) return tr
    ? `[PATTERN] Desen kaydedildi ve doğrulandı: mod ${match[1]}, ${match[2]} alan adı, motor ${match[3] === 'true' ? 'yeniden başlatıldı' : 'çalışmıyor'}.`
    : `[PATTERN] Pattern saved and verified: mode ${match[1]}, ${match[2]} domains, engine ${match[3] === 'true' ? 'restarted' : 'not running'}.`;
  if (/Bypass config changed while engine is running/i.test(clean)) return tr
    ? '[PATTERN] Desen değişti; çalışan motor yeni kuralı kullanmak üzere yeniden başlatılıyor.'
    : '[PATTERN] Pattern changed; the running engine is restarting with the new rule.';
  if (/Pattern list received from the interface did not match/i.test(clean)) return tr
    ? '[PATTERN] Arayüzdeki geçici liste ile doğrulanmış alan adları uyuşmadı; güvenli ve doğrulanmış liste kullanıldı.'
    : '[PATTERN] The temporary UI list differed from the verified domains; the safe canonical list was used.';
  if (/DNS watchdog was enabled and its health-check task was started/i.test(clean)) return tr
    ? '[DNS] Bağlantı gözlemcisinin gerçekten başlatıldığı doğrulandı.'
    : '[DNS] Verified that the connection watchdog task was started.';
  if (/DNS watchdog was disabled; no health-check task was started/i.test(clean)) return tr
    ? '[DNS] Bağlantı gözlemcisi kapalı; arka planda gözlem görevi başlatılmadı.'
    : '[DNS] Connection watchdog is off; no background health-check task was started.';
  match = clean.match(/DNS watchdog: ([A-Z]+) upstream could not resolve '([^']+)' \(failure (\d)\/3\)/i);
  if (match) return tr
    ? `[DNS] ${match[1]} sunucusu “${match[2]}” test alan adını çözemedi (${match[3]}/3). Sistem DNS henüz değiştirilmedi.`
    : `[DNS] The ${match[1]} upstream could not resolve “${match[2]}” (${match[3]}/3). System DNS has not been changed yet.`;
  if (/DNS upstream failed three real resolution checks/i.test(clean)) return tr
    ? '[DNS] Üç gerçek çözümleme denemesi başarısız oldu; internet erişimini korumak için önceki sistem DNS ayarı geri yükleniyor.'
    : '[DNS] Three real resolution checks failed; the previous system DNS setting is being restored to preserve connectivity.';
  if (/DNS kill switch verified: TCP and UDP port 53 block rules were applied/i.test(clean)) return tr
    ? '[SECURITY] DNS Kill Switch doğrulandı: TCP ve UDP 53 çıkış kuralları etkin.'
    : '[SECURITY] DNS Kill Switch verified: TCP and UDP port 53 outbound rules are active.';
  if (/DNS kill switch cleanup completed/i.test(clean)) return tr
    ? '[SECURITY] DNS Kill Switch temizliği tamamlandı.'
    : '[SECURITY] DNS Kill Switch cleanup completed.';
  if (/Engine startup failed; rolling back the DNS kill switch/i.test(clean)) return tr
    ? '[SECURITY] Motor başlayamadığı için DNS Kill Switch güvenli biçimde geri alınıyor.'
    : '[SECURITY] Engine startup failed, so the DNS Kill Switch is being rolled back safely.';
  if (/Engine stop continued, but DNS kill-switch cleanup failed/i.test(clean)) return tr
    ? clean.replace(/^\[[^\]]+]/, '[ERROR]').replace('Engine stop continued, but DNS kill-switch cleanup failed:', 'Motor durduruldu ancak DNS Kill Switch temizliği başarısız oldu:')
    : clean.replace(/^\[[^\]]+]/, '[ERROR]');
  return clean;
}

// Infers a log level from the message content.
function classifyLogLevel(content: string): 'info' | 'warn' | 'error' {
  const lower = content.toLowerCase();
  if (lower.includes('error') || lower.includes('fail') || lower.includes('hata') || lower.includes('başarısız')) return 'error';
  if (lower.includes('warn') || lower.includes('stderr') || lower.includes('uyarı') || lower.includes('could not') || lower.includes('çözemedi')) return 'warn';
  return 'info';
}
