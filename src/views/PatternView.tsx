import { useState } from 'react';
import { useEngineStore } from '../store/engineStore';
import { X, Plus, CheckCircle, AlertTriangle } from 'lucide-react';
import { translations } from '../utils/translations';
import { motion, AnimatePresence } from 'framer-motion';
import styles from './PatternView.module.css';
import { CustomSelect } from '../components/CustomSelect/CustomSelect';

const isDomainValid = (domain: string): boolean => {
  if (!domain || domain.length > 253 || !domain.includes('.') || !/^[\x00-\x7F]+$/.test(domain)) {
    return false;
  }
  return domain.split('.').every((label) =>
    label.length > 0
    && label.length <= 63
    && /^[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?$/.test(label),
  );
};

export function PatternView() {
  const {
    bypassMode,
    whitelistDomains,
    blacklistDomains,
    setBypassMode,
    setWhitelistDomains,
    setBlacklistDomains,
    appendLog,
    language,
  } = useEngineStore();

  const t = translations[language];

  const [newDomain, setNewDomain] = useState('');

  const addDomain = () => {
    const trimmed = newDomain.trim().toLowerCase();
    if (!trimmed) return;
    if (!isDomainValid(trimmed)) {
      appendLog(language === 'tr'
        ? `[ERROR] “${trimmed}” geçerli bir alan adı değil; desen listesine eklenmedi.`
        : `[ERROR] “${trimmed}” is not a valid domain and was not added to the pattern list.`, 'error');
      return;
    }

    if (bypassMode === 'whitelist') {
      if (whitelistDomains.includes(trimmed)) return;
      setWhitelistDomains([...whitelistDomains, trimmed]);
    } else {
      if (blacklistDomains.includes(trimmed)) return;
      setBlacklistDomains([...blacklistDomains, trimmed]);
    }
    setNewDomain('');
  };

  const removeDomain = (idx: number) => {
    if (bypassMode === 'whitelist') {
      setWhitelistDomains(whitelistDomains.filter((_, i) => i !== idx));
    } else {
      setBlacklistDomains(blacklistDomains.filter((_, i) => i !== idx));
    }
  };

  const activeDomains = bypassMode === 'whitelist' ? whitelistDomains : blacklistDomains;
  const modeOptions = [
    { value: 'all' as const, label: t.bypassAll, description: t.bypassAllDesc },
    { value: 'whitelist' as const, label: t.onlyWhitelist, description: t.onlyWhitelistDesc },
    { value: 'blacklist' as const, label: t.excludeBlacklist, description: t.excludeBlacklistDesc },
  ];

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
        <CustomSelect
          value={bypassMode}
          options={modeOptions}
          ariaLabel={language === 'tr' ? 'Bypass modu' : 'Bypass mode'}
          onChange={setBypassMode}
        />
      </div>

      {/* Domain List Manager Section */}
      <AnimatePresence mode="wait">
        {bypassMode !== 'all' && (
          <motion.div 
            key={bypassMode}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2 }}
            className={styles.managerSection}
          >
            <div className={styles.managerHeader}>
              <span className={styles.managerLabel}>
                {bypassMode === 'whitelist' ? t.whitelistDomains : t.blacklistDomains}
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
              <button className={styles.addBtn} onClick={addDomain} disabled={!newDomain.trim()}>
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
    </motion.div>
  );
}
