import { Shield } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { Toggle } from '../ui/Toggle';
import { NumberInput } from '../ui/NumberInput';
import { useEngineStore } from '../../../store/engineStore';
import type { AdvancedConfig } from '../../../store/engineStore';

interface Props {
  config: AdvancedConfig;
  update: <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => void;
}

export function PacketTrafficCard({ config: c, update }: Props) {
  const { language } = useEngineStore();
  const isTr = language === 'tr';

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <Shield size={18} className={styles.cardIcon} />
        <h3>{isTr ? 'Paket & Trafik Ayarları' : 'Packet & Traffic'}</h3>
      </div>
      <div className={styles.settingsList}>
        {/* Auto TTL */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Otomatik TTL' : 'Auto TTL'}</label>
            <span>{isTr ? 'Hedef için otomatik olarak güvenli bir sahte TTL seçer. (--dpi-desync-autottl)' : 'Automatically pick a safe fake TTL for the target. (--dpi-desync-autottl)'}</span>
          </div>
          <Toggle checked={c.autoTtl} onChange={(v) => update('autoTtl', v)} />
        </div>

        {/* Fake TTL */}
        {!c.autoTtl && (
          <div className={styles.settingRow}>
            <div className={styles.settingInfo}>
              <label>{isTr ? 'Sahte TTL (Fake TTL)' : 'Fake TTL'}</label>
              <span>{isTr ? 'Sahte paket yaşam süresi (katı sağlayıcılar için 3-8 önerilir). (--dpi-desync-ttl)' : 'Fake packet life (3-8 is recommended for strict ISPs). (--dpi-desync-ttl)'}</span>
            </div>
            <NumberInput value={c.fakeTtl} min={1} max={64} onChange={(v) => update('fakeTtl', v)} />
          </div>
        )}

        {/* Fake TTL Ext */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Harici TTL Algılama (TTL Ext)' : 'External TTL Evasion'}</label>
            <span>{isTr ? 'Paketle gelen winws bu bayrağı desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support this flag.'}</span>
          </div>
          <NumberInput disabled value={parseInt(c.fakeTtlExt || '0', 10)} min={0} max={64} onChange={(v) => update('fakeTtlExt', String(v))} />
        </div>

        {/* MSS Fix */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'MSS Boyutu Düzeltmesi' : 'MSS Fix'}</label>
            <span>{isTr ? 'Paketle gelen winws --mss bayrağını desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support --mss.'}</span>
          </div>
          <NumberInput disabled value={c.mssFix || 0} min={800} max={1500} onChange={(v) => update('mssFix', v)} />
        </div>

        {/* Desync Repeats */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Tekrarlama Sayısı (Repeats)' : 'Desync Repeats'}</label>
            <span>{isTr ? 'Paket manipülasyonunun kaç kez tekrarlanacağı. (--dpi-desync-repeats)' : 'How many times desync manipulation is repeated. (--dpi-desync-repeats)'}</span>
          </div>
          <NumberInput value={c.desyncRepeats} min={1} max={20} onChange={(v) => update('desyncRepeats', v)} />
        </div>

        {/* TCP Window Size */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'TCP Pencere Boyutu' : 'TCP Receiver Window'}</label>
            <span>{isTr ? 'Sunucudan gelen veriyi küçültmek için alıcı penceresini ayarlar. (--wssize)' : 'Adjust the receive window to reduce server-to-client chunks. (--wssize)'}</span>
          </div>
          <NumberInput value={c.tcpWindowSize} min={0} max={65535} onChange={(v) => update('tcpWindowSize', v)} />
        </div>
      </div>
    </div>
  );
}
