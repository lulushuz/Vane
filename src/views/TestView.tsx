import { useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import styles from './TestView.module.css';
import { normalizeIpcError } from '../types/ipc';

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
      setResult({
        success: false,
        latencyMs: 0,
        statusCode: null,
        error: `${error.message} (${error.code})`
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
      setDnsResult({
        systemDnsOk: false,
        dohDnsOk: false,
        diagnosis: `Teşhis hatası: ${error.message} (${error.code})`,
        recommendation: 'Lütfen tekrar deneyin.',
      });
    } finally {
      setIsDnsChecking(false);
    }
  }, []);

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <h2 className={styles.title}>Bağlantı Testi (Ping)</h2>
        <p className={styles.subtitle}>DPI Bypass'ı doğrudan test edebilirsiniz.</p>
      </header>

      <div className={styles.content}>
        {/* DNS Teşhis Bölümü */}
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

        {/* Mevcut Ping Test Bölümü */}
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
