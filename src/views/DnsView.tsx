import { useEffect, useCallback, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { Check, RefreshCw, ShieldCheck, X } from 'lucide-react';
import { useEngineStore } from '../store/engineStore';
import { translations } from '../utils/translations';
import styles from './DnsView.module.css';
import { CustomSelect } from '../components/CustomSelect/CustomSelect';

interface ApplyDnsResult {
  success: boolean;
  error: string | null;
}

interface ForwarderStatus {
  active: boolean;
  port: number;
  endpoint: string;
  protocol: 'doh' | 'dot';
  adblock: boolean;
  cache: boolean;
  watchdogEnabled: boolean;
}

const CloudflareIcon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="#F38020">
    <path d="M19.34 8C18.66 4.6 15.68 2 12 2 9.11 2 6.6 3.64 5.35 6.04 2.34 6.36 0 8.91 0 12c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5s-2.05-4.78-4.66-5z" />
  </svg>
);

const GoogleIcon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24">
    <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
    <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
    <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
    <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
  </svg>
);

const Quad9Icon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="#eb1f3d" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="4" />
    <path d="M12 15a3 3 0 100-6 3 3 0 000 6z" />
    <path d="M12 15L12 19" />
  </svg>
);

const AdguardIcon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="#65b169">
    <path d="M12 2L3 6v6.22C3 17.63 6.88 22.33 12 24c5.12-1.67 9-6.37 9-11.78V6l-9-4Zm-1 14.5l-4-4 1.41-1.41L11 13.67l6.59-6.59L19 8.5l-8 8Z"/>
  </svg>
);

const NextDnsIcon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="#ffffff">
    <path d="M4 4h4v16H4zm6 0h4l5.5 8L14 20h-4l5.5-8z" />
  </svg>
);

const YandexIcon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="#FF0000">
    <path d="M14.5 2H18v20h-3.5v-7.5L8.2 2H4l5.5 10v10H6v-6.5L14.5 2z" />
  </svg>
);

const MullvadIcon = ({ size = 24 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="#5C8567">
    <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 14h-2v-5l-1.5 2L8 11.5v4.5H6V8h2.5l2 3.5L12.5 8H15v8z" />
  </svg>
);

const DnsIcons: Record<string, JSX.Element> = {
  cloudflare: <CloudflareIcon />,
  google: <GoogleIcon />,
  quad9: <Quad9Icon />,
  adguard: <AdguardIcon />,
  nextdns: <NextDnsIcon />,
  yandex: <YandexIcon />,
  mullvad: <MullvadIcon />,
  custom: <ShieldCheck size={24} color="#5c7cfa" />,
};

export function DnsView() {
  // DNS state is read from the global store so it survives tab switches.
  const {
    dnsProviders: providers,
    selectedDnsId: selectedId,
    dnsCustomPrimary: customPrimary,
    dnsCustomSecondary: customSecondary,
    dnsSynced,
    setDnsSynced,
    refreshDnsStatus,
    setSelectedDnsId,
    setDnsCustom,
    dnsProtocol,
    dnsAdBlock,
    dnsCache,
    setDnsProtocol,
    setDnsAdBlock,
    setDnsCache,
    watchdog,
    setDnsForwarderEnabled,
    appendLog,
    language,
  } = useEngineStore();

  const t = translations[language];

  const [forwarder, setForwarder] = useState<ForwarderStatus | null>(null);
  const [isForwarderLoading, setIsForwarderLoading] = useState(false);
  const [customPrimaryDraft, setCustomPrimaryDraft] = useState(customPrimary);
  const [customSecondaryDraft, setCustomSecondaryDraft] = useState(customSecondary);
  const [customEditorOpen, setCustomEditorOpen] = useState(selectedId === 'custom');

  useEffect(() => {
    setCustomPrimaryDraft(customPrimary);
    setCustomSecondaryDraft(customSecondary);
  }, [customPrimary, customSecondary]);

  // Sync with the system once on mount.
  const syncWithSystem = useCallback(async () => {
    if (!dnsSynced) {
      await refreshDnsStatus();
    }
    try {
      const st = await invoke<ForwarderStatus>('get_doh_forwarder_status');
      setForwarder(st);
    } catch(e) {
      console.error(e);
    }
  }, [dnsSynced, refreshDnsStatus]);

  useEffect(() => {
    syncWithSystem();
  }, [syncWithSystem]);

  const toggleForwarder = async () => {
    if (isForwarderLoading) return;
    setIsForwarderLoading(true);
    try {
      if (forwarder?.active) {
        await invoke('stop_doh_forwarder');
        setDnsForwarderEnabled(false);
        const verified = await invoke<ForwarderStatus>('get_doh_forwarder_status');
        setForwarder(verified);
        appendLog(language === 'tr'
          ? '[DNS] DNS yönlendiricisinin durduğu ve sistem DNS ayarının geri yüklendiği doğrulandı.'
          : '[DNS] Verified that the DNS forwarder stopped and the system DNS setting was restored.', 'info');
      } else {
        const st = await invoke<ForwarderStatus>('start_doh_forwarder', { watchdog });
        setDnsForwarderEnabled(true);
        setForwarder(st);
        appendLog(language === 'tr'
          ? `[DNS] DNS yönlendiricisi doğrulandı: ${st.protocol.toUpperCase()}, önbellek ${st.cache ? 'açık' : 'kapalı'}, reklam filtresi ${st.adblock ? 'açık' : 'kapalı'}, bağlantı gözlemcisi ${st.watchdogEnabled ? 'çalışıyor' : 'kapalı'}.`
          : `[DNS] DNS forwarder verified: ${st.protocol.toUpperCase()}, cache ${st.cache ? 'on' : 'off'}, ad filter ${st.adblock ? 'on' : 'off'}, connection watchdog ${st.watchdogEnabled ? 'running' : 'off'}.`, 'info');
      }
    } catch (e: any) {
      appendLog(language === 'tr' ? `[ERROR] DNS yönlendiricisi işlemi başarısız: ${e}` : `[ERROR] DNS forwarder operation failed: ${e}`, 'error');
    } finally {
      setIsForwarderLoading(false);
    }
  };

  // Apply the selected DNS to the backend.
  const saveToBackend = async (id: string, primary?: string, secondary?: string): Promise<boolean> => {
    try {
      const targetProvider = providers.find(p => p.id === id);
      const p = (primary ?? targetProvider?.primary)?.trim();
      const s = secondary?.trim() || targetProvider?.secondary || p;

      if (!p) return false;

      const res = await invoke<ApplyDnsResult>('apply_dns_settings', {
        primary: p,
        secondary: s,
      });

      if (!res.success) {
        console.error('DNS apply failed:', res.error);
        // Re-sync on failure to restore correct UI state.
        setDnsSynced(false);
        await syncWithSystem();
        appendLog(language === 'tr' ? `[ERROR] Sistem DNS ayarı uygulanamadı: ${res.error}` : `[ERROR] System DNS setting could not be applied: ${res.error}`, 'error');
        return false;
      } else {
        if (id === 'custom') setDnsCustom(p, s ?? p);
        setSelectedDnsId(id);
        appendLog(language === 'tr'
          ? `[DNS] Sistem DNS ayarı başarıyla uygulandı ve işletim sistemi tarafından kabul edildi: ${p}${s ? ` / ${s}` : ''}.`
          : `[DNS] System DNS setting was applied and accepted by the operating system: ${p}${s ? ` / ${s}` : ''}.`, 'info');
        return true;
      }
    } catch (err) {
      console.error('Invoke error:', err);
      appendLog(language === 'tr' ? `[ERROR] Sistem DNS ayarı doğrulanamadı: ${err}` : `[ERROR] System DNS setting could not be verified: ${err}`, 'error');
      return false;
    }
  };

  const handleSelect = async (id: string) => {
    if (id === selectedId && id !== 'custom') return;
    if (forwarder?.active) {
      try {
        await invoke('stop_doh_forwarder');
        setDnsForwarderEnabled(false);
        setForwarder(await invoke<ForwarderStatus>('get_doh_forwarder_status'));
        appendLog(language === 'tr'
          ? '[DNS] Sistem DNS sağlayıcısı değiştirilmeden önce yerel yönlendirici durduruldu ve önceki DNS geri yüklendi.'
          : '[DNS] The local forwarder was stopped and the previous DNS restored before changing the system DNS provider.', 'info');
      } catch (error) {
        appendLog(language === 'tr'
          ? `[ERROR] DNS sağlayıcısı değiştirilmedi; yönlendirici güvenli biçimde durdurulamadı: ${error}`
          : `[ERROR] DNS provider was not changed because the forwarder could not be stopped safely: ${error}`, 'error');
        return;
      }
    }
    if (id === 'custom') {
      setCustomEditorOpen(true);
      return;
    }
    if (await saveToBackend(id)) setCustomEditorOpen(false);
  };

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
      className={styles.view}
    >
      <header className={styles.header}>
        <div className={styles.titleRow}>
          <h2 className={styles.title}>{t.dnsProvider}</h2>
          {!dnsSynced && providers.length === 0 && (
            <RefreshCw className={styles.spin} size={16} color="#5c7cfa" />
          )}
        </div>
        <p className={styles.subtitle}>{t.dnsAutoSaveDesc}</p>
      </header>

      {/* F8: DoH Forwarder Banner */}
      <div className={styles.forwarderBanner}>
        <div className={styles.fwInfo}>
          <ShieldCheck size={18} color={forwarder?.active ? "#4ade80" : "#a1a1aa"} />
          <div>
            <strong>{t.localDnsForwarder}</strong>
            <span>{forwarder?.active ? t.forwarderActiveDesc.replace('{protocol}', dnsProtocol.toUpperCase()) : t.forwarderInactiveDesc}</span>
          </div>
        </div>
        <button 
          className={`${styles.fwToggle} ${forwarder?.active ? styles.fwActive : ""}`} 
          onClick={toggleForwarder}
          disabled={isForwarderLoading}
        >
          {isForwarderLoading ? "..." : forwarder?.active ? t.stopForwarder : t.startForwarder}
        </button>
      </div>

      {/* Forwarder Configuration Panel */}
      <div className={styles.configPanel}>
        <h3 className={styles.panelTitle}>{t.forwarderConfig}</h3>
        <div className={styles.configGrid}>
          {/* Protocol Selection */}
          <div className={styles.configItem}>
            <label className={styles.configLabel}>{t.transportProtocol}</label>
            <CustomSelect
              value={dnsProtocol}
              ariaLabel={t.transportProtocol}
              options={[
                { value: 'doh', label: 'DNS over HTTPS (DoH)', description: language === 'tr' ? 'HTTPS üzerinden şifreli DNS' : 'Encrypted DNS over HTTPS' },
                { value: 'dot', label: 'DNS over TLS (DoT)', description: language === 'tr' ? 'TLS üzerinden şifreli DNS' : 'Encrypted DNS over TLS' },
                { value: 'doq', label: 'DNS over QUIC (DoQ)', description: language === 'tr' ? 'Bu sürümde henüz desteklenmiyor' : 'Not supported in this version yet', disabled: true },
              ]}
              onChange={setDnsProtocol}
            />
          </div>

          {/* AdBlock Toggle */}
          <div className={styles.configItemRow}>
            <div className={styles.toggleText}>
              <span className={styles.toggleTitle}>{t.localAdblock}</span>
              <span className={styles.toggleDesc}>{t.localAdblockDesc}</span>
            </div>
            <div className={styles.switchWrapper}>
              <input
                type="checkbox"
                id="adblock-toggle"
                className={styles.switchInput}
                checked={dnsAdBlock}
                onChange={(e) => setDnsAdBlock(e.target.checked)}
              />
              <label htmlFor="adblock-toggle" className={styles.switchLabel} />
            </div>
          </div>

          {/* DNS Cache Toggle */}
          <div className={styles.configItemRow}>
            <div className={styles.toggleText}>
              <span className={styles.toggleTitle}>{t.dnsCache}</span>
              <span className={styles.toggleDesc}>{t.dnsCacheDesc}</span>
            </div>
            <div className={styles.switchWrapper}>
              <input
                type="checkbox"
                id="cache-toggle"
                className={styles.switchInput}
                checked={dnsCache}
                onChange={(e) => setDnsCache(e.target.checked)}
              />
              <label htmlFor="cache-toggle" className={styles.switchLabel} />
            </div>
          </div>
        </div>
      </div>

      <div className={styles.grid}>
        {providers.map((p) => (
          <button
            key={p.id}
            className={`${styles.card} ${selectedId === p.id ? styles.selected : ''}`}
            onClick={() => handleSelect(p.id)}
          >
            {selectedId === p.id && (
              <motion.div layoutId="activeCheck" className={styles.badge}>
                <Check size={12} strokeWidth={3} />
              </motion.div>
            )}
            <div className={styles.icon}>{DnsIcons[p.id.toLowerCase()] ?? DnsIcons.google}</div>
            <div className={styles.name}>{p.name}</div>
            <div className={styles.ip}>{p.primary}</div>
          </button>
        ))}

        <button
          className={`${styles.card} ${selectedId === 'custom' || customEditorOpen ? styles.selected : ''}`}
          onClick={() => handleSelect('custom')}
        >
          {selectedId === 'custom' && (
            <div className={styles.badge}><Check size={12} strokeWidth={3} /></div>
          )}
          <div className={styles.icon}>{DnsIcons.custom}</div>
          <div className={styles.name}>{t.customDnsEntry}</div>
          <div className={styles.ip}>{t.manualEntry}</div>
        </button>
      </div>

      <AnimatePresence>
        {customEditorOpen && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            transition={{ type: 'spring', stiffness: 400, damping: 28 }}
            className={styles.customBanner}
          >
            <div className={styles.customLeft}>
              <div className={styles.customDot} />
              <div className={styles.customText}>
                <span className={styles.customTitle}>{t.customDnsSetup}</span>
                <div className={styles.customInputs}>
                  <input
                    type="text"
                    placeholder={language === 'tr' ? 'Birincil DNS' : 'Primary DNS'}
                    value={customPrimaryDraft}
                    onChange={(e) => setCustomPrimaryDraft(e.target.value)}
                  />
                  <input
                    type="text"
                    placeholder={language === 'tr' ? 'İkincil DNS' : 'Secondary DNS'}
                    value={customSecondaryDraft}
                    onChange={(e) => setCustomSecondaryDraft(e.target.value)}
                  />
                </div>
              </div>
            </div>
            
            <div className={styles.customActions}>
              <button
                className={styles.cancelBtn}
                onClick={async () => {
                  if (await saveToBackend('google')) setCustomEditorOpen(false);
                }}
              >
                <X size={14} /> {t.cancel}
              </button>
              <button
                className={styles.saveBtn}
                onClick={async () => {
                  if (await saveToBackend('custom', customPrimaryDraft, customSecondaryDraft)) {
                    setCustomEditorOpen(false);
                  }
                }}
                disabled={!customPrimaryDraft}
              >
                <Check size={14} /> {t.save}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
