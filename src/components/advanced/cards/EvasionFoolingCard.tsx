import { Ghost } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { useEngineStore } from '../../../store/engineStore';
import type { AdvancedConfig } from '../../../store/engineStore';

interface Props {
  config: AdvancedConfig;
  update: <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => void;
}

const FOOLING_FLAGS = [
  { value: 'badseq', label: 'BadSeq', desc: 'Sends fake packets with an invalid sequence number. (--dpi-desync-fooling=badseq)' },
  { value: 'badsum', label: 'BadSum', desc: 'Generates invalid TCP checksums for fake packets. (--dpi-desync-fooling=badsum)' },
  { value: 'md5sig', label: 'MD5Sig', desc: 'Sets MD5 signature option to fool simple DPI. (--dpi-desync-fooling=md5sig)' },
  { value: 'datanoack', label: 'DataNoAck', desc: 'Sends payload immediately without waiting for ACK. (--dpi-desync-fooling=datanoack)' },
  { value: 'hopbyhop', label: 'HopByHop', desc: 'IPv6: Add Hop-by-Hop options to bypass filter. (--dpi-desync-fooling=hopbyhop)' },
  { value: 'hopbyhop2', label: 'HopByHop2', desc: 'IPv6: Use the second Hop-by-Hop fooling variant. (--dpi-desync-fooling=hopbyhop2)' },
  { value: 'ts', label: 'Timestamp', desc: 'Apply TCP timestamp-based fooling. (--dpi-desync-fooling=ts)' },
];

export function EvasionFoolingCard({ config: c, update }: Props) {
  const { language } = useEngineStore();
  const isTr = language === 'tr';

  const handleToggleFlag = (flag: string) => {
    const active = c.desyncFooling.includes(flag);
    const next = active 
      ? c.desyncFooling.filter((f: string) => f !== flag)
      : [...c.desyncFooling, flag];
    update('desyncFooling', next);
  };

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <Ghost size={18} className={styles.cardIcon} style={{ color: '#ec4899' }} />
        <h3>{isTr ? 'Atlatma & Yanıltma Seçenekleri' : 'Evasion & Fooling Options'}</h3>
      </div>
      <div className={styles.settingsList}>
        {FOOLING_FLAGS.map((flag) => {
          const isChecked = c.desyncFooling.includes(flag.value);
          return (
            <div key={flag.value} className={styles.settingRow}>
              <div className={styles.settingInfo}>
                <label>{flag.label}</label>
                <span>{isTr ? flag.desc.replace('Sends', 'Gönderir').replace('Generates', 'Oluşturur').replace('Sets', 'Ayarlar').replace('IPv6:', 'IPv6:').replace('Add', 'Ekler') : flag.desc}</span>
              </div>
              <div className={styles.switchWrapper}>
                <input
                  type="checkbox"
                  id={`fooling-toggle-${flag.value}`}
                  className={styles.switchInput}
                  checked={isChecked}
                  onChange={() => handleToggleFlag(flag.value)}
                />
                <label htmlFor={`fooling-toggle-${flag.value}`} className={styles.switchLabel} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
