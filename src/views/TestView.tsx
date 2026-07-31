import { useState, useCallback, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import styles from './TestView.module.css';
import { normalizeIpcError } from '../types/ipc';
import { useDiagnosticsStore } from '../store/diagnosticsStore';

interface PingResult {
  success: boolean;
  latencyMs: number;
  statusCode: number | null;
  error: string | null;
}

interface DnsCheckResult {
  systemDnsOk: boolean;
  dohDnsOk: boolean;
  diagnosis: string;
  recommendation: string;
}

const QUICK_TARGETS = [
  { name: 'Discord', url: 'discord.com' },
  { name: 'Instagram', url: 'instagram.com' },
  { name: 'X (Twitter)', url: 'x.com' },
  { name: 'YouTube', url: 'youtube.com' }
];

const DNS_DIAGNOSE_TARGETS = [
  { name: 'Discord', domain: 'discord.com' },
  { name: 'YouTube', domain: 'youtube.com' },
  { name: 'X (Twitter)', domain: 'x.com' },
];

export function TestView() {
  const [customUrl, setCustomUrl] = useState('');
  const [isTesting, setIsTesting] = useState(false);
  const [result, setResult] = useState<PingResult | null>(null);
  const [activeTarget, setActiveTarget] = useState<string | null>(null);

  const [isDnsChecking, setIsDnsChecking] = useState(false);
  const [dnsResult, setDnsResult] = useState<DnsCheckResult | null>(null);

  // Diagnostics store hooks
  const {
    integrityStatus,
    isCheckingIntegrity,
    healthSnapshot,
    trafficReport,
    isHealthChecking,
    isProbeRunning,
    isExporting,
    lastExportPath,
    error: diagError,
    fetchArtifactIntegrity,
    runLocalDiagnostics,
    runTrafficDiagnostics,
    cancelTrafficDiagnostics,
    exportDiagnosticsBundle,
  } = useDiagnosticsStore();

  const reqVersionRef = useRef(0);

  useEffect(() => {
    fetchArtifactIntegrity();
  }, [fetchArtifactIntegrity]);

  const performTest = useCallback(async (url: string) => {
    if (!url) return;
    setIsTesting(true);
    setResult(null);
    setActiveTarget(url);
    
    try {
      const res = await invoke<PingResult>('check_url_health', { url });
      setResult(res);
    } catch (e) {
      const error = normalizeIpcError(e);
      const cleanMsg = error.message.replace(/C:\\Users\\[^\s]+/gi, '[redacted]').replace(/\/home\/[^\s]+/gi, '[redacted]');
      setResult({
        success: false,
        latencyMs: 0,
        statusCode: null,
        error: `${cleanMsg} (${error.code})`
      });
    } finally {
      setIsTesting(false);
    }
  }, []);

  const performDnsCheck = useCallback(async (domain: string) => {
    setIsDnsChecking(true);
    setDnsResult(null);
    try {
      const res = await invoke<DnsCheckResult>('check_dns_block', { domain });
      setDnsResult(res);
    } catch (e) {
      const error = normalizeIpcError(e);
      const cleanMsg = error.message.replace(/C:\\Users\\[^\s]+/gi, '[redacted]').replace(/\/home\/[^\s]+/gi, '[redacted]');
      setDnsResult({
        systemDnsOk: false,
        dohDnsOk: false,
        diagnosis: `Teşhis hatası: ${cleanMsg} (${error.code})`,
        recommendation: 'Lütfen tekrar deneyin.',
      });
    } finally {
      setIsDnsChecking(false);
    }
  }, []);

  const handleExportBundle = async () => {
    const defaultPath = 'vane-diagnostics-bundle.json';
    await exportDiagnosticsBundle(defaultPath);
  };

  const handleRunLocalDiagnostics = async () => {
    const curVersion = ++reqVersionRef.current;
    await runLocalDiagnostics();
    if (curVersion !== reqVersionRef.current) return; // stale response protection
  };


  const handleRunTrafficDiagnostics = async () => {
    const curVersion = ++reqVersionRef.current;
    await runTrafficDiagnostics();
    if (curVersion !== reqVersionRef.current) return; // stale response protection
  };

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <h2 className={styles.title}>Bağlantı ve Sistem Teşhisi</h2>
        <p className={styles.subtitle}>Bütünlük doğrulaması, yerel teşhis ve trafik testlerini buradan yönetebilirsiniz.</p>
      </header>

      <div className={styles.content}>
        {/* ─── 1. Bütünlük & Teşhis Paneli ─────────────────────────────── */}
        <div style={{
          background: 'rgba(59, 130, 246, 0.08)',
          border: '1px solid rgba(59, 130, 246, 0.25)',
          borderRadius: '12px',
          padding: '16px',
          marginBottom: '16px'
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
            <span style={{ fontSize: '0.8rem', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-secondary)' }}>
              🛡️ Motor Bütünlüğü & Gelişmiş Teşhis
            </span>
            <button
              onClick={fetchArtifactIntegrity}
              disabled={isCheckingIntegrity}
              style={{
                background: 'rgba(59, 130, 246, 0.15)',
                border: '1px solid rgba(59, 130, 246, 0.3)',
                color: '#60a5fa',
                borderRadius: '6px',
                padding: '4px 10px',
                fontSize: '0.75rem',
                fontWeight: 600,
                cursor: isCheckingIntegrity ? 'not-allowed' : 'pointer'
              }}
            >
              {isCheckingIntegrity ? 'Yenileniyor...' : 'Bütünlüğü Yeniden Doğrula'}
            </button>
          </div>

          {/* Integrity status badge */}
          <div style={{
            background: integrityStatus?.status === 'verified' ? 'rgba(34, 197, 94, 0.1)' : 'rgba(239, 68, 68, 0.1)',
            border: `1px solid ${integrityStatus?.status === 'verified' ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
            borderRadius: '8px',
            padding: '10px',
            marginBottom: '12px',
            fontSize: '0.82rem'
          }}>
            <div>
              <strong>Bütünlük Durumu: </strong>
              {integrityStatus?.status === 'verified'
                ? '✅ Doğrulandı (Verified)'
                : integrityStatus
                  ? `⚠️ Başarısız (${integrityStatus.status})`
                  : 'Kontrol edilmedi'}
            </div>
            {integrityStatus?.status !== 'verified' && integrityStatus && (
              <p style={{ color: '#ef4444', margin: '4px 0 0', fontSize: '0.78rem' }}>
                Vane motor dosyalarının bütünlük doğrulaması başarısız oldu. Güvenlik nedeniyle motor ve optimizer çalışması engellendi.
              </p>
            )}
          </div>

          {/* Action buttons */}
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginBottom: '12px' }}>
            <motion.button
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              disabled={isHealthChecking}
              onClick={handleRunLocalDiagnostics}
              style={{
                background: 'rgba(59, 130, 246, 0.15)',
                border: '1px solid rgba(59, 130, 246, 0.3)',
                color: '#60a5fa',
                borderRadius: '8px',
                padding: '6px 12px',
                fontSize: '0.82rem',
                fontWeight: 600,
                cursor: isHealthChecking ? 'not-allowed' : 'pointer',
              }}
            >
              {isHealthChecking ? '⏳ Yerel Teşhis...' : 'Yerel Teşhisi Çalıştır'}
            </motion.button>

            <motion.button
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              disabled={isProbeRunning}
              onClick={handleRunTrafficDiagnostics}
              style={{
                background: 'rgba(59, 130, 246, 0.15)',
                border: '1px solid rgba(59, 130, 246, 0.3)',
                color: '#60a5fa',
                borderRadius: '8px',
                padding: '6px 12px',
                fontSize: '0.82rem',
                fontWeight: 600,
                cursor: isProbeRunning ? 'not-allowed' : 'pointer',
              }}
            >
              {isProbeRunning ? '⏳ Trafik Test Ediliyor...' : 'Trafik Teşhisi Çalıştır'}
            </motion.button>

            {isProbeRunning && (
              <button
                onClick={cancelTrafficDiagnostics}
                style={{
                  background: 'rgba(239, 68, 68, 0.15)',
                  border: '1px solid rgba(239, 68, 68, 0.3)',
                  color: '#ef4444',
                  borderRadius: '8px',
                  padding: '6px 12px',
                  fontSize: '0.82rem',
                  fontWeight: 600,
                  cursor: 'pointer'
                }}
              >
                Trafik Testini İptal Et
              </button>
            )}

            <button
              onClick={handleExportBundle}
              disabled={isExporting}
              style={{
                background: 'rgba(168, 85, 247, 0.15)',
                border: '1px solid rgba(168, 85, 247, 0.3)',
                color: '#c084fc',
                borderRadius: '8px',
                padding: '6px 12px',
                fontSize: '0.82rem',
                fontWeight: 600,
                cursor: isExporting ? 'not-allowed' : 'pointer'
              }}
            >
              {isExporting ? 'Aktarılıyor...' : 'Teşhis Paketini Dışa Aktar'}
            </button>
          </div>

          {/* Diagnostic status cards */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
            {/* Engine Ready ayrımı */}
            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '10px', borderRadius: '8px', fontSize: '0.8rem' }}>
              <div style={{ color: 'var(--text-secondary)', marginBottom: '4px' }}>Engine Ready Status</div>
              <strong style={{
                color: healthSnapshot?.overall === 'healthy' ? '#22c55e' : healthSnapshot ? '#f59e0b' : '#9ca3af'
              }}>
                {healthSnapshot
                  ? healthSnapshot.overall === 'healthy' ? '✅ Motor Hazır (Healthy)' : `⚠️ ${healthSnapshot.overall}`
                  : 'Henüz kontrol edilmedi'}
              </strong>
            </div>

            {/* Traffic Reachable ve Not checked ayrımı */}
            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '10px', borderRadius: '8px', fontSize: '0.8rem' }}>
              <div style={{ color: 'var(--text-secondary)', marginBottom: '4px' }}>Traffic Reachable Status</div>
              <strong style={{
                color: trafficReport ? (trafficReport.successRatio > 0.5 ? '#22c55e' : '#ef4444') : '#9ca3af'
              }}>
                {trafficReport
                  ? `Trafik Ulaşılabilir (Oran: %${Math.round(trafficReport.successRatio * 100)})`
                  : 'Kontrol edilmedi (Not checked)'}
              </strong>
            </div>
          </div>

          {/* DpiBypassAssessment Inconclusive or Unknown */}
          {trafficReport && (
            <div style={{ marginTop: '8px', fontSize: '0.78rem', color: '#a5b4fc' }}>
              DPI Bypass Değerlendirmesi: <strong>{trafficReport.assessment === 'inconclusive' ? 'Kesin Değil (Inconclusive)' : trafficReport.assessment}</strong>
            </div>
          )}

          {lastExportPath && (
            <div style={{ marginTop: '8px', fontSize: '0.75rem', color: '#22c55e' }}>
              Teşhis paketi dışa aktarıldı: {lastExportPath}
            </div>
          )}

          {diagError && (
            <div style={{ marginTop: '8px', fontSize: '0.75rem', color: '#ef4444' }}>
              Hata: {diagError.replace(/C:\\Users\\[^\s]+/gi, '[redacted]').replace(/\/home\/[^\s]+/gi, '[redacted]')}
            </div>
          )}
        </div>

        {/* ─── 2. DNS Teşhis Bölümü ─────────────────────────────────── */}
        <div style={{
          background: 'rgba(139, 92, 246, 0.08)',
          border: '1px solid rgba(139, 92, 246, 0.25)',
          borderRadius: '12px',
          padding: '16px',
          marginBottom: '16px'
        }}>
          <span style={{ fontSize: '0.8rem', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-secondary)' }}>
            🔬 DNS Sorun Teşhisi
          </span>
          <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', margin: '8px 0 12px' }}>
            Siteye erişemiyorsan sorun DNS mi DPI mi? Öğrenmek için tıkla:
          </p>
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            {DNS_DIAGNOSE_TARGETS.map(t => (
              <motion.button
                key={t.domain}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                disabled={isDnsChecking}
                onClick={() => performDnsCheck(t.domain)}
                style={{
                  background: 'rgba(139, 92, 246, 0.15)',
                  border: '1px solid rgba(139, 92, 246, 0.3)',
                  color: '#a78bfa',
                  borderRadius: '8px',
                  padding: '6px 14px',
                  fontSize: '0.85rem',
                  fontWeight: 600,
                  cursor: isDnsChecking ? 'not-allowed' : 'pointer',
                  opacity: isDnsChecking ? 0.5 : 1,
                }}
              >
                {isDnsChecking ? '⏳ Teşhis Yapılıyor...' : `${t.name} Teşhis Et`}
              </motion.button>
            ))}
          </div>

          <AnimatePresence>
            {dnsResult && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                exit={{ opacity: 0, height: 0 }}
                style={{ marginTop: '12px' }}
              >
                <div style={{
                  background: dnsResult.systemDnsOk ? 'rgba(34,197,94,0.08)' : 'rgba(239,68,68,0.08)',
                  border: `1px solid ${dnsResult.systemDnsOk ? 'rgba(34,197,94,0.3)' : 'rgba(239,68,68,0.3)'}`,
                  borderRadius: '8px',
                  padding: '12px'
                }}>
                  <div style={{ display: 'flex', gap: '16px', marginBottom: '8px' }}>
                    <span style={{ fontSize: '0.82rem' }}>
                      Sistem DNS: {dnsResult.systemDnsOk ? '✅ Çalışıyor' : '❌ Bloklanmış'}
                    </span>
                    <span style={{ fontSize: '0.82rem' }}>
                      Doğrudan IP: {dnsResult.dohDnsOk ? '✅ Çalışıyor' : '❌ Engelli'}
                    </span>
                  </div>
                  <p style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-primary)', margin: '0 0 6px' }}>
                    {dnsResult.diagnosis}
                  </p>
                  <p style={{ fontSize: '0.82rem', color: 'var(--text-secondary)', margin: 0 }}>
                    💡 {dnsResult.recommendation}
                  </p>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        {/* ─── 3. Mevcut Ping Test Bölümü ───────────────────────────── */}
        <div className={styles.quickTargets}>
          <span className={styles.sectionLabel}>Hızlı Hedefler</span>
          <div className={styles.grid}>
            {QUICK_TARGETS.map(t => (
              <motion.button
                key={t.name}
                className={`${styles.targetBtn} ${activeTarget === t.url ? styles.activeTarget : ''}`}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                onClick={() => { setCustomUrl(t.url); performTest(t.url); }}
                disabled={isTesting}
              >
                {t.name}
              </motion.button>
            ))}
          </div>
        </div>

        <div className={styles.customInputArea}>
            <span className={styles.sectionLabel}>Özel Hedef</span>
            <div className={styles.inputGroup}>
                <input 
                  type="text" 
                  className={styles.input} 
                  placeholder="e.g. reddit.com" 
                  value={customUrl}
                  onChange={e => setCustomUrl(e.target.value)}
                  onKeyDown={e => e.key === 'Enter' && performTest(customUrl)}
                  disabled={isTesting}
                />
                <button 
                  className={styles.testBtn} 
                  onClick={() => performTest(customUrl)}
                  disabled={isTesting || !customUrl}
                  aria-label="Test Et"
                >
                  {isTesting ? <span className={styles.spinner} /> : 'Test Et'}
                </button>
            </div>
        </div>

        <AnimatePresence>
            {result && (
                <motion.div 
                  className={`${styles.resultCard} ${result.success ? styles.success : styles.error}`}
                  initial={{ opacity: 0, y: 10, scale: 0.98 }}
                  animate={{ opacity: 1, y: 0, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                >
                    <div className={styles.resultHeader}>
                        <span className={styles.resultIcon}>{result.success ? '🟢' : '🔴'}</span>
                        <span className={styles.resultStatus}>
                            {result.success ? 'Ulaşılabilir' : 'Ulaşılamadı (Zaman Aşımı / Blok)'}
                        </span>
                    </div>
                    
                    <div className={styles.resultDetails}>
                        <div className={styles.metric}>
                            <span className={styles.metricLabel}>Gecikme (Ping)</span>
                            <span className={styles.metricValue}>{result.latencyMs} ms</span>
                        </div>
                        {result.statusCode && (
                            <div className={styles.metric}>
                                <span className={styles.metricLabel}>HTTP Kodu</span>
                                <span className={styles.metricValue}>{result.statusCode}</span>
                            </div>
                        )}
                    </div>
                    
                    {!result.success && result.error && (
                        <div className={styles.errorText}>
                           Gerekçe: {result.error}
                        </div>
                    )}
                </motion.div>
            )}
        </AnimatePresence>
      </div>
    </div>
  );
}

