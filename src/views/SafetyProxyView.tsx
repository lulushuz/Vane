import { useState, useEffect } from 'react';
import { useEngineStore } from '../store/engineStore';
import { ShieldAlert, EyeOff, Network, Save } from 'lucide-react';
import { Toast } from '../components/Toast/Toast';
import { translations } from '../utils/translations';
import styles from './SafetyProxyView.module.css';
import { invoke } from '@tauri-apps/api/core';
import { normalizeIpcError } from '../types/ipc';

export function SafetyProxyView() {
  const {
    killSwitch,
    watchdog,
    proxySocks5,
    status,
    language,
    appendLog,
    dnsProtocol,
    dnsAdBlock,
    dnsCache,
    healthCheckTargets,
    bypassMode,
    whitelistDomains,
    blacklistDomains,
    activePresetId,
  } = useEngineStore();

  const t = translations[language];

  const [localKillSwitch, setLocalKillSwitch] = useState(killSwitch);
  const [localWatchdog, setLocalWatchdog] = useState(watchdog);
  const [localProxy, setLocalProxy] = useState(proxySocks5);
  const [showToast, setShowToast] = useState(false);

  const isRunning = status.variant === 'running';

  useEffect(() => {
    setLocalKillSwitch(killSwitch);
    setLocalWatchdog(watchdog);
    setLocalProxy(proxySocks5);
  }, [killSwitch, watchdog, proxySocks5]);

  const handleSave = async () => {
    let verifiedProxy = '';
    try {
      verifiedProxy = await invoke<string>('validate_socks5_proxy', { proxy: localProxy });
    } catch (error) {
      const ipcError = normalizeIpcError(error);
      appendLog(language === 'tr'
        ? `[ERROR] SOCKS5 proxy kaydedilmedi: ${ipcError.message} (${ipcError.code})`
        : `[ERROR] SOCKS5 proxy was not saved: ${ipcError.message} (${ipcError.code})`, 'error');
      return;
    }
    if (verifiedProxy && dnsProtocol === 'dot') {
      appendLog(language === 'tr'
        ? '[ERROR] SOCKS5 proxy kaydedilmedi: önce DNS protokolünü DoH olarak seçin.'
        : '[ERROR] SOCKS5 proxy was not saved: select DoH as the DNS protocol first.', 'error');
      return;
    }
    const oldSettings = { killSwitch, watchdog, proxySocks5 };
    const activeDomains = bypassMode === 'whitelist' ? whitelistDomains : blacklistDomains;
    try {
      await invoke('sync_dns_settings', {
        protocol: dnsProtocol === 'doq' ? 'doh' : dnsProtocol,
        adblock: dnsAdBlock,
        cache: dnsCache,
        socks5Proxy: verifiedProxy,
        healthCheckTargets,
        emitEvent: false,
      });
      await invoke('sync_bypass_config', {
        mode: bypassMode,
        list: activeDomains.join('\n'),
        proxy: verifiedProxy,
        killSwitch: localKillSwitch,
        whitelistDomains,
        blacklistDomains,
        activePresetId: activePresetId || 'default',
      });
      const watchdogStatus = await invoke<{ active: boolean; watchdogEnabled: boolean }>(
        'set_dns_watchdog',
        { enabled: localWatchdog },
      );
      useEngineStore.setState({
        killSwitch: localKillSwitch,
        watchdog: watchdogStatus.active ? watchdogStatus.watchdogEnabled : localWatchdog,
        proxySocks5: verifiedProxy,
      });
      appendLog(language === 'tr'
        ? `[SECURITY] Güvenlik ayarları doğrulandı: Kill Switch ${localKillSwitch ? 'açık' : 'kapalı'}, gözlemci ${localWatchdog ? 'açık' : 'kapalı'}, DNS bağlantısı ${verifiedProxy ? 'SOCKS5H proxy' : 'doğrudan'}.`
        : `[SECURITY] Safety settings verified: Kill Switch ${localKillSwitch ? 'on' : 'off'}, watchdog ${localWatchdog ? 'on' : 'off'}, DNS connection ${verifiedProxy ? 'SOCKS5H proxy' : 'direct'}.`, 'info');
      setShowToast(true);
    } catch (error) {
      const ipcError = normalizeIpcError(error);
      // Best-effort runtime rollback. Persistence is unchanged until every
      // backend step above succeeds.
      try {
        await invoke('sync_dns_settings', {
          protocol: dnsProtocol === 'doq' ? 'doh' : dnsProtocol,
          adblock: dnsAdBlock,
          cache: dnsCache,
          socks5Proxy: oldSettings.proxySocks5,
          healthCheckTargets,
          emitEvent: false,
        });
        await invoke('sync_bypass_config', {
          mode: bypassMode,
          list: activeDomains.join('\n'),
          proxy: oldSettings.proxySocks5,
          killSwitch: oldSettings.killSwitch,
          whitelistDomains,
          blacklistDomains,
          activePresetId: activePresetId || 'default',
        });
        await invoke('set_dns_watchdog', { enabled: oldSettings.watchdog });
      } catch {
        // The original error remains the actionable message; backend logs retain rollback detail.
      }
      appendLog(language === 'tr'
        ? `[ERROR] Güvenlik ayarları doğrulanamadı; kalıcı ayarlar değiştirilmedi: ${ipcError.message} (${ipcError.code})`
        : `[ERROR] Safety settings could not be verified; persisted settings were not changed: ${ipcError.message} (${ipcError.code})`, 'error');
    }
  };

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <h2 className={styles.title}>{t.safety}</h2>
        <p className={styles.subtitle}>
          {language === 'tr' ? 'Güvenlik duvarı koruması, DNS sızıntı koruması, otomatik kurtarma ve DoH için SOCKS5 upstream yönlendirme.' : 'Firewall protection, DNS leak prevention, auto-recovery, and SOCKS5 upstream routing for DoH.'}
        </p>
      </header>

      <div className={styles.section}>
        <h3 className={styles.sectionTitle}>{t.privacySecurity}</h3>

        {/* Kill Switch Card */}
        <div className={styles.card}>
          <div className={styles.cardHeader}>
            <div className={`${styles.iconWrapper} ${styles.redIcon}`}>
              <ShieldAlert size={20} />
            </div>
            <div className={styles.cardTitleInfo}>
              <span className={styles.cardTitle}>{t.dnsLeakProtection}</span>
              <span className={styles.cardDesc}>{t.dnsLeakProtectionDesc}</span>
            </div>
            <div className={styles.toggleWrapper}>
              <input
                type="checkbox"
                id="killswitch-toggle"
                className={styles.toggleInput}
                checked={localKillSwitch}
                onChange={(e) => setLocalKillSwitch(e.target.checked)}
              />
              <label htmlFor="killswitch-toggle" className={styles.toggleLabel} />
            </div>
          </div>
          <p className={styles.detailedDesc}>
            {t.dnsLeakProtectionDetailed}
          </p>
        </div>

        {/* Watchdog Card */}
        <div className={styles.card}>
          <div className={styles.cardHeader}>
            <div className={`${styles.iconWrapper} ${styles.blueIcon}`}>
              <EyeOff size={20} />
            </div>
            <div className={styles.cardTitleInfo}>
              <span className={styles.cardTitle}>{t.watchdog}</span>
              <span className={styles.cardDesc}>{t.watchdogDesc}</span>
            </div>
            <div className={styles.toggleWrapper}>
              <input
                type="checkbox"
                id="watchdog-toggle"
                className={styles.toggleInput}
                checked={localWatchdog}
                onChange={(e) => setLocalWatchdog(e.target.checked)}
              />
              <label htmlFor="watchdog-toggle" className={styles.toggleLabel} />
            </div>
          </div>
          <p className={styles.detailedDesc}>
            {t.watchdogDetailed}
          </p>
        </div>
      </div>

      {isRunning && (localKillSwitch !== killSwitch) && (
        <div className={styles.warningBox}>
          <span>{t.restartEngineWarning}</span>
        </div>
      )}

      <div className={styles.divider} />

      <div className={styles.section}>
        <h3 className={styles.sectionTitle}>{t.proxy}</h3>

        {/* SOCKS5 Proxy Card */}
        <div className={styles.card}>
          <div className={styles.cardHeader}>
            <div className={`${styles.iconWrapper} ${styles.proxyIcon}`}>
              <Network size={20} />
            </div>
            <div className={styles.cardTitleInfo}>
              <span className={styles.cardTitle}>{t.socks5Proxy}</span>
              <span className={styles.cardDesc}>{t.socks5ProxyDesc}</span>
            </div>
          </div>

          <input
            type="text"
            className={styles.input}
            value={localProxy}
            onChange={(e) => setLocalProxy(e.target.value)}
            placeholder="127.0.0.1:9050 (e.g. Tor)"
          />
          <p className={styles.detailedDesc}>
            {t.socks5ProxyDetailed}
          </p>
        </div>
      </div>

      <footer className={styles.footer}>
        <button className={styles.saveBtn} onClick={handleSave}>
          <Save size={16} />
          {t.saveAll}
        </button>
      </footer>

      {showToast && (
        <div className={styles.toastWrapper}>
          <Toast
            message={t.settingsSaved}
            type="success"
            onDismiss={() => setShowToast(false)}
          />
        </div>
      )}
    </div>
  );
}
