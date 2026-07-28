import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useEngineStore } from '../store/engineStore';
import styles from './EngineHealthBadge.module.css';

interface HealthStatus {
  healthy: boolean;
  latencyMs: number;
  checkedAt: string;
  target: string;
}

/** How often to ping the health check (ms). 60s to avoid suspicious traffic patterns. */
const HEALTH_CHECK_INTERVAL_MS = 60_000;
/** Cooldown between manual checks (ms) to prevent spamming the backend. */
const MANUAL_CHECK_COOLDOWN_MS = 5_000;

/**
 * Displays a live connectivity status badge when the engine is running.
 *
 * Design decisions:
 * - Only polls while engine status is 'running' — zero overhead when stopped.
 * - Uses `setInterval` rather than Rust events to keep backend clean.
 * - Checks user-defined targets (max 3).
 */
export function EngineHealthBadge() {
  const { status, presets, activePresetId, healthCheckTargets, bypassMode, whitelistDomains } = useEngineStore();
  const isRunning = status.variant === 'running';
  const activePreset = presets.find((p) => p.id === activePresetId);
  const presetLabel = activePreset?.label ?? activePresetId ?? 'Default';

  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const lastCheckTime = useRef<number>(0);

  const runCheck = async (isManual = false) => {
    if (isChecking) return;

    const now = Date.now();
    if (isManual && now - lastCheckTime.current < MANUAL_CHECK_COOLDOWN_MS) {
      // Cooldown active, ignore manual click to prevent spam
      return;
    }

    setIsChecking(true);
    lastCheckTime.current = Date.now();

    let targetsToCheck = healthCheckTargets;
    const isDefaultTarget = healthCheckTargets.length === 0 || (healthCheckTargets.length === 1 && healthCheckTargets[0] === 'example.com');
    if (bypassMode === 'whitelist' && whitelistDomains && whitelistDomains.length > 0 && isDefaultTarget) {
      targetsToCheck = whitelistDomains.slice(0, 3);
    }

    try {
      const result = await invoke<HealthStatus>('get_engine_health', { 
        targets: targetsToCheck 
      });
      setHealth(result);
    } catch {
      setHealth({ 
        healthy: false, 
        latencyMs: 0, 
        checkedAt: '--:--:--', 
        target: (targetsToCheck && targetsToCheck.length > 0) ? targetsToCheck.join(', ') : 'Unknown' 
      });
    } finally {
      setIsChecking(false);
    }
  };

  useEffect(() => {
    if (!isRunning) {
      // Clear badge and stop polling when engine stops
      setHealth(null);
      if (intervalRef.current) clearInterval(intervalRef.current);
      return;
    }

    // Initial check delayed slightly (500ms) on engine start to allow WinDivert socket binding to finish
    const startTimer = setTimeout(() => {
      runCheck();
    }, 500);

    // Schedule periodic checks
    intervalRef.current = setInterval(() => runCheck(false), HEALTH_CHECK_INTERVAL_MS);

    return () => {
      clearTimeout(startTimer);
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isRunning, healthCheckTargets]);

  if (!isRunning || !health) {
    return (
      <div className={`${styles.badge} ${styles.disconnected}`}>
        {presetLabel}
      </div>
    );
  }

  const isHighLatency = health.latencyMs > 3000;
  const label = health.healthy
    ? isHighLatency
      ? `${presetLabel} | ⚠ ${health.latencyMs}ms`
      : `${presetLabel} | ${health.latencyMs}ms`
    : `${presetLabel} | ✕ Unreachable`;

  const cls = health.healthy
    ? isHighLatency
      ? styles.badge + ' ' + styles.warn
      : styles.badge + ' ' + styles.healthy
    : styles.badge + ' ' + styles.error;

  return (
    <div
      className={cls}
      title={`${health.target} — ${health.checkedAt}`}
      onClick={() => runCheck(true)}
      role="button"
      aria-label="Connection status — click to check now"
    >
      {isChecking ? (
        <>
          {presetLabel} | <span className={styles.spinner} />
        </>
      ) : (
        label
      )}
    </div>
  );
}
