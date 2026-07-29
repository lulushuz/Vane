import { ServerCrash } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import { Toggle } from '../ui/Toggle';
import { NumberInput } from '../ui/NumberInput';
import { useEngineStore } from '../../../store/engineStore';
import type { AdvancedConfig } from '../../../store/engineStore';

interface Props {
  config: AdvancedConfig;
  update: <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => void;
}

export function DpiDesyncCard({ config: c, update }: Props) {
  const { language } = useEngineStore();

  const isTr = language === 'tr';
  const noneLabel = isTr ? 'Yok' : 'None';

  const DESYNC_OPTIONS = [
    { value: 'none', label: noneLabel },
    { value: 'fake', label: 'Fake' },
    { value: 'multisplit', label: 'MultiSplit' },
    { value: 'multidisorder', label: 'MultiDisorder' },
    { value: 'fakedsplit', label: 'FakedSplit' },
    { value: 'fakeddisorder', label: 'FakedDisorder' },
    { value: 'hostfakesplit', label: 'HostFakeSplit' },
    { value: 'syndata', label: 'Syndata' },
  ];

  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <ServerCrash size={18} className={styles.cardIcon} />
        <h3>{isTr ? 'DPI Desenkronizasyon Yöntemleri' : 'DPI Desynchronization'}</h3>
      </div>
      <div className={styles.settingsList}>
        {/* General Desync Method */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Genel Yöntem' : 'General Method'}</label>
            <span>{isTr ? 'Genel paket bölme veya manipülasyon taktiği. (--dpi-desync)' : 'General packet splitting or corruption tactic. (--dpi-desync)'}</span>
          </div>
          <div className={styles.flexCol}>
            <select className={styles.selectBox} value={c.desyncMethod} onChange={(e) => update('desyncMethod', e.target.value)}>
              <option value="none">{noneLabel}</option>
              <option value="fake">Fake</option>
              <option value="multisplit">MultiSplit</option>
              <option value="multidisorder">MultiDisorder</option>
              <option value="fakedsplit">FakedSplit</option>
              <option value="fakeddisorder">FakedDisorder</option>
              <option value="hostfakesplit">HostFakeSplit</option>
              <option value="syndata">Syndata</option>
              <option value="custom">Custom</option>
            </select>
            {c.desyncMethod === 'custom' && (
              <input type="text" className={`${styles.textInput} ${styles.mt2}`} placeholder="örn: fake,multidisorder" value={c.customDesyncMethod} onChange={(e) => update('customDesyncMethod', e.target.value)} />
            )}
          </div>
        </div>

        {/* Any Protocol */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Tüm Protokoller' : 'Any Protocol'}</label>
            <span>{isTr ? 'Sadece HTTP/S değil, tüm giden protokollere uygula. (--dpi-desync-any-protocol)' : 'Apply to all outbound protocols, not just HTTP/S. (--dpi-desync-any-protocol)'}</span>
          </div>
          <Toggle checked={c.anyProtocol} onChange={(v) => update('anyProtocol', v)} />
        </div>

        {/* Cutoff */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Desync Kesme Limiti (Cutoff)' : 'Desync Cutoff'}</label>
            <span>{isTr ? 'Kaç paketten sonra manipülasyonun duracağını belirler (örn: d3). (--dpi-desync-cutoff)' : 'Number of packets or bytes to desync before stopping (e.g. d3). (--dpi-desync-cutoff)'}</span>
          </div>
          <input 
            type="text" 
            className={styles.textInput} 
            value={c.desyncCutoff} 
            onChange={(e) => update('desyncCutoff', e.target.value)}
            placeholder="e.g. d3"
            pattern="[nds][0-9]+"
          />
        </div>

        {/* HTTP Specific desync */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'HTTP Özel Yöntemi' : 'HTTP Specific Strategy'}</label>
            <span>{isTr ? 'Paketle gelen winws bu bayrağı desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support this flag.'}</span>
          </div>
          <select disabled className={styles.selectBox} value={c.desyncHttp} onChange={(e) => update('desyncHttp', e.target.value)}>
            {DESYNC_OPTIONS.map(opt => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
          </select>
        </div>

        {/* HTTPS (TLS) Specific desync */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'HTTPS (TLS) Özel Yöntemi' : 'HTTPS (TLS) Specific Strategy'}</label>
            <span>{isTr ? 'Paketle gelen winws bu bayrağı desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support this flag.'}</span>
          </div>
          <select disabled className={styles.selectBox} value={c.desyncHttps} onChange={(e) => update('desyncHttps', e.target.value)}>
            {DESYNC_OPTIONS.map(opt => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
          </select>
        </div>

        {/* QUIC Specific desync */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'QUIC Özel Yöntemi' : 'QUIC Specific Strategy'}</label>
            <span>{isTr ? 'Paketle gelen winws bu bayrağı desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support this flag.'}</span>
          </div>
          <select disabled className={styles.selectBox} value={c.desyncQuic} onChange={(e) => update('desyncQuic', e.target.value)}>
            <option value="none">{noneLabel}</option>
            <option value="split">Split</option>
            <option value="disorder">Disorder</option>
            <option value="fake">Fake</option>
          </select>
        </div>

        {/* Second Stage Desync */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'İkinci Aşama Desync' : 'Second Stage Desync'}</label>
            <span>{isTr ? 'Paketle gelen winws bu bayrağı desteklemediği için kullanılamaz.' : 'Unavailable because the bundled winws does not support this flag.'}</span>
          </div>
          <select disabled className={styles.selectBox} value={c.desync2} onChange={(e) => update('desync2', e.target.value)}>
            <option value="none">{noneLabel}</option>
            <option value="split">Split</option>
            <option value="split2">Split2</option>
            <option value="disorder">Disorder</option>
            <option value="fake">Fake</option>
            <option value="oob">OOB</option>
            <option value="syndata">Syndata</option>
          </select>
        </div>

        {/* Split Position (General) */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'Genel Bölme Konumu' : 'General Split Position'}</label>
            <span>{isTr ? 'Paketin bölüneceği genel byte konumu. (--dpi-desync-split-pos)' : 'At which byte the general packets are split. (--dpi-desync-split-pos)'}</span>
          </div>
          <NumberInput value={c.splitPosition} min={1} max={1500} onChange={(v) => update('splitPosition', v)} />
        </div>

        {/* HTTP Split Type */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'HTTP Bölme Türü' : 'HTTP Split Type'}</label>
            <span>{isTr ? 'HTTP bölme yöntemi (method / host). (--dpi-desync-split-http-req)' : 'Select HTTP split tactic (method / host). (--dpi-desync-split-http-req)'}</span>
          </div>
          <select className={styles.selectBox} value={c.splitHttpReq} onChange={(e) => update('splitHttpReq', e.target.value)}>
            <option value="none">{noneLabel}</option>
            <option value="method">Method</option>
            <option value="host">Host</option>
          </select>
        </div>

        {/* HTTP Split Position */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'HTTP Bölme Konumu' : 'HTTP Split Position'}</label>
            <span>{isTr ? 'Bu ayrı konum bayrağı paketle gelen winws tarafından desteklenmiyor.' : 'This separate position flag is not supported by the bundled winws.'}</span>
          </div>
          <NumberInput disabled value={c.splitPosHttpReq} min={0} max={1500} onChange={(v) => update('splitPosHttpReq', v)} />
        </div>

        {/* TLS Split Type */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'TLS Bölme Türü' : 'TLS Split Type'}</label>
            <span>{isTr ? 'HTTPS/TLS bölme yöntemi (sni / sniext). (--dpi-desync-split-tls)' : 'Select TLS ClientHello split tactic (sni / sniext). (--dpi-desync-split-tls)'}</span>
          </div>
          <select className={styles.selectBox} value={c.splitTls} onChange={(e) => update('splitTls', e.target.value)}>
            <option value="none">{noneLabel}</option>
            <option value="sni">SNI</option>
            <option value="sniext">SNI Extension</option>
          </select>
        </div>

        {/* TLS Split Position */}
        <div className={styles.settingRow}>
          <div className={styles.settingInfo}>
            <label>{isTr ? 'TLS Bölme Konumu' : 'TLS Split Position'}</label>
            <span>{isTr ? 'Bu ayrı konum bayrağı paketle gelen winws tarafından desteklenmiyor.' : 'This separate position flag is not supported by the bundled winws.'}</span>
          </div>
          <NumberInput disabled value={c.splitPosTls} min={0} max={1500} onChange={(v) => update('splitPosTls', v)} />
        </div>
      </div>
    </div>
  );
}
