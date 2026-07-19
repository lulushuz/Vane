import { useCallback, useState, useEffect } from 'react';
import { motion, Variants } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Settings, AlertCircle, GitBranch, X } from 'lucide-react';
import { getVersion } from '@tauri-apps/api/app';
import { useEngineStore } from '../store/engineStore';
import { UpdateBanner } from '../components/UpdateBanner';
import { EngineHealthBadge } from '../components/EngineHealthBadge';
import { Toast } from '../components/Toast/Toast';
import styles from './WidgetView.module.css';
import logoUrl from '../assets/logo.png';

export function WidgetView() {
  const {
    status,
    startEngine,
    stopEngine,
  } = useEngineStore();

  const [isActionLoading, setIsActionLoading] = useState(false);
  const isRunning = status.variant === 'running';
  const isStarting = status.variant === 'starting';
  const isError = status.variant === 'error';

  const [authError, setAuthError] = useState<string | null>(null);
  const [version, setVersion] = useState<string>('');

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  useEffect(() => {
    if (status.variant === 'error' && status.code === 'AUTHORIZATION_FAILED') {
      setAuthError('Redirection could not be started because authorization was not granted.');
    } else {
      setAuthError(null);
    }
  }, [status]);

  const handleToggle = useCallback(async () => {
    if (isStarting || isActionLoading) return;
    setIsActionLoading(true);
    try {
      if (isRunning) {
        await stopEngine();
      } else {
        await startEngine();
      }
    } finally {
      setIsActionLoading(false);
    }
  }, [isRunning, isStarting, isActionLoading, startEngine, stopEngine]);

  const openSettings = async () => {
    try {
      await invoke('open_settings_window');
    } catch (e) {
      console.error('Failed to open settings', e);
    }
  };

  const closeWindow = async () => {
    try {
      // hide() is used intentionally: the backend's CloseRequested handler would call
      // prevent_close() + hide() anyway, so calling hide() directly avoids the roundtrip.
      await getCurrentWebviewWindow().hide();
    } catch (e) {
      console.error('Failed to hide window', e);
    }
  };

  useEffect(() => {
    let hasGainedFocus = false;
    let isGracePeriod = true;

    const timer = setTimeout(() => {
      isGracePeriod = false;
    }, 2000);

    const setupFocusListener = async () => {
      const windowObj = getCurrentWebviewWindow();
      const unlisten = await windowObj.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          hasGainedFocus = true;
        } else {
          if (!isGracePeriod && hasGainedFocus) {
            windowObj.hide();
          }
        }
      });
      return unlisten;
    };

    let unlistenPromise = setupFocusListener();

    return () => {
      clearTimeout(timer);
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const openGithub = () => {
    invoke('open_url', { url: 'https://github.com/lulushuz/Vane' });
  };

  // Animated Radio SVG Variants
  const pathVariants: Variants = {
    off: { opacity: 1, pathLength: 1, scale: 1 },
    starting: { opacity: [1, 1, 1], transition: { repeat: Infinity, duration: 1.5 } },
    on: { opacity: 1, scale: 1, transition: { duration: 0.5 } }
  };
  const wave1Variants: Variants = {
    off: { opacity: 0.6, scale: 1 },
    starting: { opacity: [0, 0.5, 0], scale: [0.8, 1, 0.8], transition: { repeat: Infinity, duration: 1.2 } },
    on: { opacity: 0.7, scale: 1, transition: { duration: 0.5 } }
  };
  const wave2Variants: Variants = {
    off: { opacity: 0.3, scale: 1 },
    starting: { opacity: [0, 0.3, 0], scale: [0.9, 1, 0.9], transition: { repeat: Infinity, duration: 1.2, delay: 0.2 } },
    on: { opacity: 0.3, scale: 1, transition: { duration: 0.5 } }
  };

  const animState = isRunning ? 'on' : isStarting ? 'starting' : 'off';
  const iconColor = isRunning ? '#3b82f6' : isStarting ? '#ffcc00' : '#ef4444'; // Kapalıyken (off) artık kırmızı

  return (
    <div className={styles.container}>
      {/* ─── Header ─── */}
      <header className={styles.header}>
        <div className={styles.headerLeft}>
          <img src={logoUrl} alt="Vane Logo" className={styles.brandIcon} width={30} height={30} />
          <div className={styles.brandTexts}>
            <span className={styles.brandName}>Vane</span>
            <span className={styles.alphaBadge}>v{version}</span>
          </div>
        </div>

        <div className={styles.headerRight}>
          <button className={styles.iconBtn} onClick={openGithub} title="Github">
            <GitBranch size={14} />
          </button>
          <button className={styles.iconBtn} onClick={openSettings} title="Advanced Settings">
            <Settings size={14} />
          </button>
          <button className={`${styles.iconBtn} ${styles.closeBtn}`} onClick={closeWindow} title="Close">
            <X size={14} />
          </button>
        </div>
      </header>

      {/* ─── Update Banner ─── */}
      <UpdateBanner />

      {/* ─── Body ─── */}
      <div className={styles.content}>
        <button
          className={`${styles.mainToggleBtn} ${isRunning ? styles.on : isStarting ? styles.starting : styles.off}`}
          onClick={handleToggle}
          disabled={isStarting || isActionLoading}
          data-tauri-drag-region="false"
        >
          <motion.svg
            width="100"
            height="100"
            viewBox="0 0 24 24"
            fill="none"
            stroke={iconColor}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className={styles.centerIcon}
            animate={animState}
          >
            <motion.path d="M12 11a1 1 0 1 0 0-2 1 1 0 0 0 0 2z" variants={pathVariants} />
            <motion.path d="M12 11v8" variants={pathVariants} />
            <motion.path d="M9 19h6" variants={pathVariants} />
            <motion.path d="M15.5 6.5a5 5 0 0 1 0 7" variants={wave1Variants} />
            <motion.path d="M8.5 13.5a5 5 0 0 1 0-7" variants={wave1Variants} />
            <motion.path d="M18.5 3.5a9 9 0 0 1 0 13" variants={wave2Variants} />
            <motion.path d="M5.5 16.5a9 9 0 0 1 0-13" variants={wave2Variants} />
          </motion.svg>
        </button>

        {isError && status.variant === 'error' && (
          <div className={styles.errorBox}>
            <AlertCircle size={14} />
            <span title={status.message}>
              {status.message.length > 52
                ? `${status.message.slice(0, 52)}…`
                : status.message}
            </span>
          </div>
        )}

        <div className={`${styles.statusText} ${isRunning ? styles.textOn : isError ? styles.textError : styles.textOff}`}>
          {isStarting
            ? 'Connecting...'
            : isRunning
              ? 'Connected'
              : isError
                ? 'Error'
                : 'Disconnected'}
        </div>

        <EngineHealthBadge />
      </div>

      {/* ─── Toast Container ─── */}
      <div className={styles.toastContainer}>
        {authError && (
          <Toast message={authError} type="warning" onDismiss={() => setAuthError(null)} />
        )}
      </div>
    </div>
  );
}
