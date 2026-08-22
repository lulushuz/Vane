import { useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LayoutDashboard, Scroll, DatabaseZap, FlaskConical, LifeBuoy, Layers } from 'lucide-react';
import { HomeView } from './HomeView';
import { LogView } from './LogView';
import { AdvancedView } from './AdvancedView';
import { DnsView } from './DnsView';
import { FeedbackView } from './FeedbackView';
import { PatternView } from './PatternView';
import { useEngineStore } from '../store/engineStore';
import { translations } from '../utils/translations';
import styles from './SettingsWindow.module.css';

type SettingsTab = 'general' | 'connection' | 'dns' | 'advanced' | 'pattern' | 'feedback';

export function SettingsWindow() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [visitedTabs, setVisitedTabs] = useState<Set<SettingsTab>>(new Set(['general']));
  const { advancedDirty, language } = useEngineStore();

  const appWindow = getCurrentWebviewWindow();
  const closeWindow = async () => await appWindow.close();
  const minimizeWindow = async () => await appWindow.minimize();
  const toggleMaximize = async () => await appWindow.toggleMaximize();

  const t = translations[language];

  const topTabs = [
    { id: 'general', label: t.general, icon: LayoutDashboard },
    { id: 'dns', label: t.dns, icon: DatabaseZap },
    { id: 'advanced', label: t.advanced, icon: FlaskConical },
    { id: 'pattern', label: t.pattern, icon: Layers },
  ];

  const bottomTabs = [
    { id: 'connection', label: t.logs, icon: Scroll },
    { id: 'feedback', label: t.feedback, icon: LifeBuoy },
  ];

  const handleTabChange = (tabId: SettingsTab) => {
    if (advancedDirty && activeTab === 'advanced' && tabId !== 'advanced') {
      const confirmed = window.confirm(
        language === 'tr'
          ? 'Kaydedilmemiş değişiklikler var. Devam ederseniz değişiklikleriniz kaybolacaktır. Ayrılmak istiyor musunuz?'
          : 'There are unsaved changes. If you continue, your changes will be lost. Do you want to leave?'
      );
      if (!confirmed) return;
    }
    setVisitedTabs((prev) => {
      if (prev.has(tabId)) return prev;
      const next = new Set(prev);
      next.add(tabId);
      return next;
    });
    setActiveTab(tabId);
  };

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <div 
          className={styles.dragRegion} 
          data-tauri-drag-region="true"
          onPointerDown={() => appWindow.startDragging()}
        >
          <span className={styles.windowTitle}>{t.settingsTitle}</span>
        </div>

        <div className={styles.windowControls}>
          <div className={styles.controlBtn} onClick={minimizeWindow}>
            <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="white" /></svg>
          </div>
          <div className={styles.controlBtn} onClick={toggleMaximize}>
            <svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" stroke="white" fill="none" /></svg>
          </div>
          <div className={`${styles.controlBtn} ${styles.closeBtn}`} onClick={closeWindow}>
            <svg width="10" height="10" viewBox="0 0 10 10">
              <path d="M1 1L9 9M9 1L1 9" stroke="white" strokeWidth="1.2" />
            </svg>
          </div>
        </div>
      </header>

      <div className={styles.mainLayout}>
        <nav className={styles.sideBar}>
          {/* ─ Üst Grup ─ */}
          {topTabs.map((tab) => {
            const IconComponent = tab.icon;
            const isAdvanced = tab.id === 'advanced';
            return (
              <button
                key={tab.id}
                className={`${styles.navItem} ${activeTab === tab.id ? styles.active : ''}`}
                onClick={() => handleTabChange(tab.id as SettingsTab)}
              >
                <div className={styles.activeIndicator} />
                <div style={{ position: 'relative' }}>
                  <IconComponent className={styles.icon} size={22} strokeWidth={1.5} />
                  {isAdvanced && advancedDirty && (
                    <div style={{
                      position: 'absolute', top: -3, right: -3,
                      width: 8, height: 8, borderRadius: '50%',
                      background: '#f59e0b',
                    }} />
                  )}
                </div>
                <span className={styles.label}>{tab.label}</span>
              </button>
            );
          })}

          {/* Esnek Boşluk */}
          <div style={{ flex: 1 }} />

          {/* ─ Alt Grup ─ */}
          {bottomTabs.map((tab) => {
            const IconComponent = tab.icon;
            return (
              <button
                key={tab.id}
                className={`${styles.navItem} ${activeTab === tab.id ? styles.active : ''}`}
                onClick={() => handleTabChange(tab.id as SettingsTab)}
              >
                <div className={styles.activeIndicator} />
                <IconComponent className={styles.icon} size={22} strokeWidth={1.5} />
                <span className={styles.label}>{tab.label}</span>
              </button>
            );
          })}
        </nav>

        <main className={styles.contentWrapper}>
          <div className={styles.viewWrapper}>
            {visitedTabs.has('general') && (
              <div style={{ display: activeTab === 'general' ? 'contents' : 'none' }}>
                <HomeView />
              </div>
            )}
            {visitedTabs.has('connection') && (
              <div style={{ display: activeTab === 'connection' ? 'contents' : 'none' }}>
                <LogView />
              </div>
            )}
            {visitedTabs.has('dns') && (
              <div style={{ display: activeTab === 'dns' ? 'contents' : 'none' }}>
                <DnsView />
              </div>
            )}
            {visitedTabs.has('advanced') && (
              <div style={{ display: activeTab === 'advanced' ? 'contents' : 'none' }}>
                <AdvancedView />
              </div>
            )}
            {visitedTabs.has('pattern') && (
              <div style={{ display: activeTab === 'pattern' ? 'contents' : 'none' }}>
                <PatternView />
              </div>
            )}
            {visitedTabs.has('feedback') && (
              <div style={{ display: activeTab === 'feedback' ? 'contents' : 'none' }}>
                <FeedbackView />
              </div>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}