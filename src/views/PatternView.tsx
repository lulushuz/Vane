import { useState, useEffect } from 'react';
import { useEngineStore } from '../store/engineStore';
import { invoke } from '@tauri-apps/api/core';
import { Save, X, Plus, CheckCircle, AlertTriangle } from 'lucide-react';
import { Toast } from '../components/Toast/Toast';
import { translations } from '../utils/translations';
import { motion, AnimatePresence } from 'framer-motion';
import styles from './PatternView.module.css';

const DOMAIN_ALIASES: Record<string, string[]> = {
  'discord.com': ['discordapp.com', 'discordapp.net', 'discord.gg'],
  'roblox.com': ['robloxlabs.com', 'rbxcdn.com'],
  'youtube.com': ['youtu.be', 'ytimg.com', 'ggpht.com']
};

const isDomainValid = (domain: string): boolean => {
  const regex = /^(?:\*\.)?[a-zA-Z0-9][-a-zA-Z0-9]{0,62}(?:\.[a-zA-Z0-9][-a-zA-Z0-9]{0,62})+$/;
  return regex.test(domain);
};

export function PatternView() {
  const {
    bypassMode,
    whitelistDomains,
    blacklistDomains,
    setBypassMode,
    setWhitelistDomains,
    setBlacklistDomains,
    setDomainList,
    status,
    startEngine,
    stopEngine,
    language,
    hasHydrated,
  } = useEngineStore();

  const t = translations[language];

  const [localMode, setLocalMode] = useState<'all' | 'whitelist' | 'blacklist'>(bypassMode);
  
  // Convert newline-separated strings from store to arrays
  const getArrayFromStore = (str: string) => {
    return str.split('\n').map(d => d.trim()).filter(d => d.length > 0);
  };

  const [localWhitelist, setLocalWhitelist] = useState<string[]>(() => getArrayFromStore(whitelistDomains));
  const [localBlacklist, setLocalBlacklist] = useState<string[]>(() => getArrayFromStore(blacklistDomains));
  const [newDomain, setNewDomain] = useState('');
  
  const [showToast, setShowToast] = useState(false);
  const [toastMessage, setToastMessage] = useState('');
  const [toastType, setToastType] = useState<'success' | 'error' | 'warning'>('success');

  const isEngineRunning = status.variant === 'running';

  // Sync state only when store rehydration completes
  useEffect(() => {
    if (hasHydrated) {
      setLocalWhitelist(getArrayFromStore(whitelistDomains));
      setLocalBlacklist(getArrayFromStore(blacklistDomains));
      setLocalMode(bypassMode);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasHydrated]);

  const cleanDomains = (domains: string[]) => {
    const resultSet = new Set<string>(domains);

    for (const domain of domains) {
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

  const handleSave = async () => {
    const whitelistString = localWhitelist.join('\n');
    const blacklistString = localBlacklist.join('\n');
    
    const cleanedWhitelist = cleanDomains(localWhitelist);
    const cleanedBlacklist = cleanDomains(localBlacklist);

    // Save separately to store
    setBypassMode(localMode);
    setWhitelistDomains(whitelistString);
    setBlacklistDomains(blacklistString);
    
    // Sync with the backend's expected single active domainList
    let activeList = '';
    if (localMode === 'whitelist') activeList = cleanedWhitelist;
    else if (localMode === 'blacklist') activeList = cleanedBlacklist;
    setDomainList(activeList);

    // Force immediate sync to Rust memory cache before engine restart
    try {
      await invoke('sync_bypass_config', {
        mode: localMode,
        list: activeList,
        proxy: useEngineStore.getState().proxySocks5,
        killSwitch: useEngineStore.getState().killSwitch,
      });
    } catch (e) {
      console.error("Direct sync_bypass_config failed:", e);
    }

    if (isEngineRunning) {
      setToastType('warning');
      setToastMessage(t.restartingEngine);
      setShowToast(true);
      try {
        await stopEngine();
        await new Promise(r => setTimeout(r, 600)); // wait for process cleanup
        await startEngine();
        setToastType('success');
        setToastMessage(t.savedAndRestarted);
      } catch (err) {
        setToastType('error');
        setToastMessage(language === 'tr' ? `Motor yeniden başlatılamadı: ${err}` : `Failed to restart engine: ${err}`);
      }
    } else {
      setToastType('success');
      setToastMessage(t.savedSuccessfully);
      setShowToast(true);
    }
  };

  const handleCancel = () => {
    setLocalMode(bypassMode);
    setLocalWhitelist(getArrayFromStore(whitelistDomains));
    setLocalBlacklist(getArrayFromStore(blacklistDomains));
    setNewDomain('');
  };

  const addDomain = () => {
    const trimmed = newDomain.trim().toLowerCase();
    if (!trimmed) return;
    
    if (localMode === 'whitelist') {
      if (localWhitelist.includes(trimmed)) return;
      setLocalWhitelist([...localWhitelist, trimmed]);
    } else {
      if (localBlacklist.includes(trimmed)) return;
      setLocalBlacklist([...localBlacklist, trimmed]);
    }
    setNewDomain('');
  };

  const removeDomain = (idx: number) => {
    if (localMode === 'whitelist') {
      setLocalWhitelist(localWhitelist.filter((_, i) => i !== idx));
    } else {
      setLocalBlacklist(localBlacklist.filter((_, i) => i !== idx));
    }
  };

  const storeWhitelistArr = getArrayFromStore(whitelistDomains);
  const storeBlacklistArr = getArrayFromStore(blacklistDomains);

  const hasChanges = localMode !== bypassMode ||
    JSON.stringify(localWhitelist) !== JSON.stringify(storeWhitelistArr) ||
    JSON.stringify(localBlacklist) !== JSON.stringify(storeBlacklistArr);

  const activeDomains = localMode === 'whitelist' ? localWhitelist : localBlacklist;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
      className={styles.container}
    >
      <header className={styles.header}>
        <h2 className={styles.title}>{t.bypassPatternControl}</h2>
        <p className={styles.subtitle}>
          {t.bypassPatternDesc}
        </p>
      </header>

      {/* Mode Selection Dropdown */}
      <div className={styles.selectWrapper}>
        <label className={styles.selectLabel}>{language === 'tr' ? 'Bypass Modu Seçin' : 'Select Bypass Mode'}</label>
        <select
          className={styles.select}
          value={localMode}
          onChange={(e) => setLocalMode(e.target.value as any)}
        >
          <option value="all">{t.bypassAll}</option>
          <option value="whitelist">{t.onlyWhitelist}</option>
          <option value="blacklist">{t.excludeBlacklist}</option>
        </select>
      </div>

      {/* Domain List Manager Section */}
      <AnimatePresence mode="wait">
        {localMode !== 'all' && (
          <motion.div 
            key={localMode}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2 }}
            className={styles.managerSection}
          >
            <div className={styles.managerHeader}>
              <span className={styles.managerLabel}>
                {localMode === 'whitelist' ? t.whitelistDomains : t.blacklistDomains}
              </span>
              <span className={styles.badge}>
                {activeDomains.length} {language === 'tr' ? 'alan adı listelendi' : `domain${activeDomains.length !== 1 ? 's' : ''} listed`}
              </span>
            </div>

            {/* Input to add domain */}
            <div className={styles.inputRow}>
              <input
                type="text"
                className={styles.input}
                value={newDomain}
                onChange={(e) => setNewDomain(e.target.value)}
                placeholder="example.com"
                onKeyDown={(e) => { if (e.key === 'Enter') addDomain(); }}
              />
              <button className={styles.addBtn} onClick={addDomain}>
                <Plus size={16} />
                <span>{language === 'tr' ? 'Ekle' : 'Add'}</span>
              </button>
            </div>

            {/* Domains List View (Scrollable) */}
            <div className={styles.listContainer}>
              {activeDomains.length === 0 ? (
                <div className={styles.emptyList}>
                  {language === 'tr' ? 'Henüz hiçbir alan adı eklenmemiş.' : 'No domains added yet.'}
                </div>
              ) : (
                activeDomains.map((domain, idx) => {
                  const isValid = isDomainValid(domain);
                  return (
                    <div key={`${domain}-${idx}`} className={styles.listItem}>
                      <div className={styles.domainInfo}>
                        {isValid ? (
                          <div className={`${styles.statusBadge} ${styles.valid}`}>
                            <CheckCircle size={12} />
                            <span>{language === 'tr' ? 'Geçerli' : 'Valid'}</span>
                          </div>
                        ) : (
                          <div className={`${styles.statusBadge} ${styles.invalid}`}>
                            <AlertTriangle size={12} />
                            <span>{language === 'tr' ? 'Geçersiz' : 'Invalid'}</span>
                          </div>
                        )}
                        <span className={styles.domainName}>{domain}</span>
                      </div>
                      <button className={styles.removeBtn} onClick={() => removeDomain(idx)}>
                        <X size={14} />
                      </button>
                    </div>
                  );
                })
              )}
            </div>
            
            <span className={styles.helperText}>
              {t.wildcardHelper}
            </span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Action Banner (Save / Cancel) */}
      <AnimatePresence>
        {hasChanges && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            transition={{ type: 'spring', stiffness: 400, damping: 28 }}
            className={styles.actionBanner}
          >
            <div className={styles.bannerLeft}>
              <div className={styles.bannerDot} />
              <span className={styles.bannerText}>{t.unsavedChanges}</span>
            </div>
            <div className={styles.bannerActions}>
              <button className={styles.cancelBtn} onClick={handleCancel}>
                <X size={14} /> {t.cancel}
              </button>
              <button className={styles.saveBtn} onClick={handleSave}>
                <Save size={14} /> {t.saveChanges}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Toast Feedback */}
      {showToast && (
        <div className={styles.toastWrapper}>
          <Toast
            message={toastMessage}
            type={toastType}
            onDismiss={() => setShowToast(false)}
          />
        </div>
      )}
    </motion.div>
  );
}
