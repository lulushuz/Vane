import { useState, useEffect, useRef } from 'react';
import { LogViewer } from '../components/LogViewer/LogViewer';
import { useEngineStore } from '../store/engineStore';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Activity, Radio } from 'lucide-react';
import { translations } from '../utils/translations';
import { motion } from 'framer-motion';
import styles from './LogView.module.css';

export function LogView() {
  const { logs, clearLogs, language } = useEngineStore();
  const t = translations[language];

  // DNS Activity Graph State
  const [dnsData, setDnsData] = useState<number[]>(new Array(30).fill(0));
  const dnsCountRef = useRef(0);

  // Internet Speed Graph State (Rx: Download, Tx: Upload)
  const [rxData, setRxData] = useState<number[]>(new Array(30).fill(0));
  const [txData, setTxData] = useState<number[]>(new Array(30).fill(0));
  const prevNetBytesRef = useRef<{ rx: number; tx: number } | null>(null);

  useEffect(() => {
    // 1. Setup DNS Activity listener
    let active = true;
    let dnsUnlisten: (() => void) | null = null;

    const setupDnsListener = async () => {
      try {
        const unlisten = await listen('dns_activity', () => {
          if (active) dnsCountRef.current += 1;
        });
        dnsUnlisten = unlisten;
      } catch (err) {
        console.error('Failed to listen to dns_activity:', err);
      }
    };
    setupDnsListener();

    // 2. Setup 1-second interval for DNS and Internet speed
    const fetchStats = async () => {
      try {
        const bytes = await invoke<[number, number]>('get_network_stats');
        const [rx, tx] = bytes;

        let currentRxSpeed = 0;
        let currentTxSpeed = 0;
        if (prevNetBytesRef.current !== null) {
          currentRxSpeed = Math.max(0, rx - prevNetBytesRef.current.rx);
          currentTxSpeed = Math.max(0, tx - prevNetBytesRef.current.tx);
        }
        prevNetBytesRef.current = { rx, tx };

        if (active) {
          // Update internet speed data (Download & Upload)
          setRxData((prev) => [...prev.slice(1), currentRxSpeed]);
          setTxData((prev) => [...prev.slice(1), currentTxSpeed]);

          // Update DNS queries data
          setDnsData((prev) => {
            const currentDns = dnsCountRef.current;
            dnsCountRef.current = 0; // reset for next second
            return [...prev.slice(1), currentDns];
          });
        }
      } catch (err) {
        console.error('Failed to fetch network stats:', err);
      }
    };

    // Initial fetch
    fetchStats();
    const interval = setInterval(fetchStats, 1000);

    return () => {
      active = false;
      clearInterval(interval);
      if (dnsUnlisten) dnsUnlisten();
    };
  }, []);

  // Helpers to format graphs
  const renderChart = (data: number[], color: string, gradId: string, isSpeed: boolean) => {
    const width = 230;
    const height = 64;
    const padding = 4;
    const maxVal = Math.max(...data, isSpeed ? 1024 * 50 : 5);

    const points = data.map((val, idx) => {
      const x = (idx / (data.length - 1)) * (width - padding * 2) + padding;
      const y = height - (val / maxVal) * (height - padding * 2) - padding;
      return { x, y };
    });

    const pathD = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
    const areaD = `${pathD} L ${points[points.length - 1].x} ${height} L ${points[0].x} ${height} Z`;

    return (
      <div className={styles.chartWrapper}>
        <svg width="100%" height="100%" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" style={{ display: 'block' }}>
          <defs>
            <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity="0.25" />
              <stop offset="100%" stopColor={color} stopOpacity="0.0" />
            </linearGradient>
          </defs>
          <path d={areaD} fill={`url(#${gradId})`} />
          <path d={pathD} fill="none" stroke={color} strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </div>
    );
  };

  const renderSpeedChart = (rx: number[], tx: number[]) => {
    const width = 230;
    const height = 64;
    const padding = 4;
    const maxVal = Math.max(...rx, ...tx, 1024 * 50); // min 50 KB/s scale

    const getPoints = (data: number[]) => data.map((val, idx) => {
      const x = (idx / (data.length - 1)) * (width - padding * 2) + padding;
      const y = height - (val / maxVal) * (height - padding * 2) - padding;
      return { x, y };
    });

    const rxPoints = getPoints(rx);
    const txPoints = getPoints(tx);

    const rxPathD = rxPoints.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
    const txPathD = txPoints.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');

    const rxAreaD = `${rxPathD} L ${rxPoints[rxPoints.length - 1].x} ${height} L ${rxPoints[0].x} ${height} Z`;
    const txAreaD = `${txPathD} L ${txPoints[txPoints.length - 1].x} ${height} L ${txPoints[0].x} ${height} Z`;

    return (
      <div className={styles.chartWrapper}>
        <svg width="100%" height="100%" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" style={{ display: 'block' }}>
          <defs>
            <linearGradient id="rxGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#10b981" stopOpacity="0.2" />
              <stop offset="100%" stopColor="#10b981" stopOpacity="0.0" />
            </linearGradient>
            <linearGradient id="txGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.15" />
              <stop offset="100%" stopColor="#3b82f6" stopOpacity="0.0" />
            </linearGradient>
          </defs>
          <path d={rxAreaD} fill="url(#rxGrad)" />
          <path d={txAreaD} fill="url(#txGrad)" />
          <path d={rxPathD} fill="none" stroke="#10b981" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          <path d={txPathD} fill="none" stroke="#3b82f6" strokeWidth="1.2" strokeDasharray="2,2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </div>
    );
  };

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return '0 B/s';
    const k = 1024;
    const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    return parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const currentDnsSpeed = dnsData[dnsData.length - 1];
  const currentRx = rxData[rxData.length - 1];
  const currentTx = txData[txData.length - 1];

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
      className={styles.view}
    >
      {/* Real-time graphs section */}
      <div className={styles.chartsGrid}>
        {/* DNS Queries Graph */}
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className={styles.chartCard}
        >
          <div className={styles.chartHeader}>
            <div className={styles.chartTitleRow}>
              <Activity size={14} className={styles.dnsIcon} />
              <span>{t.dnsForwarderTraffic}</span>
            </div>
            <span className={styles.dnsValue}>{currentDnsSpeed} Q/s</span>
          </div>
          {renderChart(dnsData, '#5c7cfa', 'dnsGrad', false)}
        </motion.div>

        {/* Network Speed Graph (Dual Download/Upload) */}
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.05 }}
          className={styles.chartCard}
        >
          <div className={styles.chartHeader}>
            <div className={styles.chartTitleRow}>
              <Radio size={14} className={styles.netIcon} />
              <span>{t.networkSpeed}</span>
            </div>
            <div style={{ display: 'flex', gap: 8, fontSize: 10 }}>
              <span style={{ color: '#10b981', fontWeight: 600 }}>↓ {formatSpeed(currentRx)}</span>
              <span style={{ color: '#3b82f6', fontWeight: 600 }}>↑ {formatSpeed(currentTx)}</span>
            </div>
          </div>
          {renderSpeedChart(rxData, txData)}
        </motion.div>
      </div>

      <div className={styles.viewerContainer}>
        <LogViewer logs={logs} onClear={clearLogs} />
      </div>
    </motion.div>
  );
}
