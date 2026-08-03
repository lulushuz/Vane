import { ShieldCheck, AlertTriangle } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { Toggle } from '../ui/Toggle';
import { useEngineStore } from '../../../store/engineStore';
import { translations } from '../../../utils/translations';
import { useState } from 'react';

export function SafetyCard() {
  const {
    killSwitch,
    watchdog,
    setKillSwitch,
    setWatchdog,
    status,
    language,
  } = useEngineStore();

  const t = translations[language];
  const isRunning = status.variant === 'ready';

  const [initialKillSwitch] = useState(killSwitch);

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <ShieldCheck size={18} className={styles.cardIcon} style={{ color: '#5c7cfa' }} />
        <h3>{t.privacySecurity}</h3>
      </div>
      <div className={styles.settingsList}>
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{t.dnsLeakProtection}</label>
            <span>{t.dnsLeakProtectionDesc}</span>
          </div>
          <Toggle checked={killSwitch} onChange={setKillSwitch} />
        </div>

        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{t.watchdog}</label>
            <span>{t.watchdogDesc}</span>
          </div>
          <Toggle checked={watchdog} onChange={setWatchdog} />
        </div>

        {isRunning && (killSwitch !== initialKillSwitch) && (
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '8px 12px',
            background: 'rgba(245, 158, 11, 0.1)',
            border: '1px solid rgba(245, 158, 11, 0.2)',
            borderRadius: 6,
            color: '#f59e0b',
            fontSize: 10,
            lineHeight: 1.4,
            marginTop: 4
          }}>
            <AlertTriangle size={14} style={{ flexShrink: 0 }} />
            <span>{t.restartEngineWarning}</span>
          </div>
        )}
      </div>
    </div>
  );
}
