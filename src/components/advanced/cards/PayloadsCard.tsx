import { Binary } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { useEngineStore } from '../../../store/engineStore';
import type { AdvancedConfig } from '../../../store/engineStore';

interface Props {
  config: AdvancedConfig;
  update: <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => void;
}

export function PayloadsCard({ config: c, update }: Props) {
  const { language } = useEngineStore();
  const isTr = language === 'tr';

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <Binary size={18} className={styles.cardIcon} style={{ color: '#eab308' }} />
        <h3>{isTr ? 'Özel Veri Paketleri (Payloads)' : 'Custom Payloads & SNI'}</h3>
      </div>
      <div className={styles.settingsList}>
        {/* Fake TLS SNI */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Sahte TLS SNI' : 'Fake TLS SNI'}</label>
            <span>{isTr ? 'Paketle gelen winws bu bayrağı desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support this flag.'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.fakeTlsSni} 
            onChange={(e) => update('fakeTlsSni', e.target.value)} 
            placeholder="e.g. google.com"
            maxLength={253}
            disabled
          />
        </div>

        {/* Fake HTTP Payload */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Sahte HTTP Verisi (Payload)' : 'HTTP Fake Payload'}</label>
            <span>{isTr ? 'Güvenli dosya seçici ve payload doğrulaması tamamlanana kadar kullanılamaz.' : 'Unavailable until a safe file picker and payload validation are implemented.'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.fakeHttpPayload} 
            onChange={(e) => update('fakeHttpPayload', e.target.value)} 
            placeholder="e.g. fake_http.bin"
            disabled
          />
        </div>

        {/* Fake TLS Payload */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Sahte TLS Verisi (Payload)' : 'TLS Fake Payload'}</label>
            <span>{isTr ? 'Güvenli dosya seçici ve payload doğrulaması tamamlanana kadar kullanılamaz.' : 'Unavailable until a safe file picker and payload validation are implemented.'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.fakeTlsPayload} 
            onChange={(e) => update('fakeTlsPayload', e.target.value)} 
            placeholder="e.g. fake_tls.bin"
            disabled
          />
        </div>

        {/* Fake QUIC Payload */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Sahte QUIC Verisi (Payload)' : 'QUIC Fake Payload'}</label>
            <span>{isTr ? 'Güvenli dosya seçici ve payload doğrulaması tamamlanana kadar kullanılamaz.' : 'Unavailable until a safe file picker and payload validation are implemented.'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.fakeQuicPayload} 
            onChange={(e) => update('fakeQuicPayload', e.target.value)} 
            placeholder="e.g. fake_quic.bin"
            disabled
          />
        </div>
      </div>
    </div>
  );
}
