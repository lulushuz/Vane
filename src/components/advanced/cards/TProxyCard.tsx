import { Compass } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { Toggle } from '../ui/Toggle';
import { useEngineStore } from '../../../store/engineStore';
import type { AdvancedConfig } from '../../../store/engineStore';

interface Props {
  config: AdvancedConfig;
  update: <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => void;
}

export function TProxyCard(_props: Props) {
  const { language } = useEngineStore();
  const isTr = language === 'tr';

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <Compass size={18} className={styles.cardIcon} style={{ color: '#a855f7' }} />
        <h3>{isTr ? 'Proxy & IPSet Listeleri' : 'Proxy & IPSet Lists'}</h3>
      </div>
      <div className={styles.settingsList}>
        {/* TPWS Proxy Mode Toggle */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'TPWS Proxy Modu' : 'TPWS Proxy Mode'}</label>
            <span>{isTr ? 'Bu pakette TPWS ikilisi bulunmadığı için kullanılamaz.' : 'Unavailable because this package does not include a TPWS binary.'}</span>
          </div>
          <Toggle checked={false} onChange={() => undefined} disabled />
        </div>

        {/* IPSet Path */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'IPSet Dosya Yolu' : 'IPSet List Path'}</label>
            <span>{isTr ? 'Güvenli dosya içe aktarma desteği eklenene kadar kullanılamaz.' : 'Unavailable until a safe file-import flow is implemented.'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value=""
            onChange={() => undefined}
            placeholder={isTr ? 'Bu sürümde desteklenmiyor' : 'Not supported in this release'}
            disabled
          />
        </div>
      </div>
    </div>
  );
}
