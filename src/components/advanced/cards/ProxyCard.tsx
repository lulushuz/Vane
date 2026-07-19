import { Network } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { useEngineStore } from '../../../store/engineStore';
import { translations } from '../../../utils/translations';
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function ProxyCard() {
  const {
    proxySocks5,
    setProxySocks5,
    language,
    appendLog,
    dnsProtocol,
  } = useEngineStore();

  const t = translations[language];
  const [draft, setDraft] = useState(proxySocks5);

  useEffect(() => setDraft(proxySocks5), [proxySocks5]);

  const saveProxy = async () => {
    try {
      const verified = await invoke<string>('validate_socks5_proxy', { proxy: draft });
      if (verified && dnsProtocol === 'dot') {
        throw new Error(language === 'tr' ? 'Önce DNS protokolünü DoH olarak seçin.' : 'Select DoH as the DNS protocol first.');
      }
      const applied = await setProxySocks5(verified);
      setDraft(applied ? verified : proxySocks5);
    } catch (error) {
      setDraft(proxySocks5);
      appendLog(language === 'tr'
        ? `[ERROR] SOCKS5 proxy kaydedilmedi: ${error}`
        : `[ERROR] SOCKS5 proxy was not saved: ${error}`, 'error');
    }
  };

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <Network size={18} className={styles.cardIcon} style={{ color: '#10b981' }} />
        <h3>{t.proxy}</h3>
      </div>
      <div className={styles.settingsList}>
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{t.socks5Proxy}</label>
            <span>{t.socks5ProxyDesc}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={saveProxy}
            placeholder="127.0.0.1:9050"
          />
        </div>
      </div>
    </div>
  );
}
