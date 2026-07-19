import { Radio } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { Toggle } from '../ui/Toggle';
import { useEngineStore } from '../../../store/engineStore';
import type { AdvancedConfig } from '../../../store/engineStore';

interface Props {
  config: AdvancedConfig;
  update: <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => void;
}

export function ProtocolPortsCard({ config: c, update }: Props) {
  const { language } = useEngineStore();
  const isTr = language === 'tr';

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <Radio size={18} className={styles.cardIcon} />
        <h3>{isTr ? 'Protokol & Port Ayarları' : 'Protocol & Ports'}</h3>
      </div>
      <div className={styles.settingsList}>
        {/* QUIC / UDP Handling */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'QUIC / UDP İşleme' : 'QUIC / UDP Handling'}</label>
            <span>{isTr ? 'UDP 443 trafiğini (YouTube, Discord vb.) desync tüneline dahil et. (--wf-udp=443)' : 'Include UDP 443 traffic (YouTube, Discord, etc.) in bypass. (--wf-udp=443)'}</span>
          </div>
          <Toggle checked={c.quicUdpHandling} onChange={(v) => update('quicUdpHandling', v)} />
        </div>

        {/* HTTP Ports */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'HTTP Portları' : 'HTTP Ports'}</label>
            <span>{isTr ? 'Yönlendirme motoruna girecek TCP portları (virgülle ayırın). (--wf-tcp)' : 'TCP ports to enter the bypass engine (comma separated). (--wf-tcp)'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.httpPorts} 
            onChange={(e) => update('httpPorts', e.target.value)} 
            inputMode="numeric"
            pattern="[0-9, -]+"
            placeholder="80, 443"
          />
        </div>

        {/* Bind Interface / IP */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Dinlenecek Arayüz / IP' : 'Bind Interface / IP'}</label>
            <span>{isTr ? 'Paketle gelen winws --bind-addr bayrağını desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support --bind-addr.'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.bindInterface} 
            onChange={(e) => update('bindInterface', e.target.value)} 
            placeholder="e.g. 127.0.0.1"
            disabled
          />
        </div>
      </div>
    </div>
  );
}
