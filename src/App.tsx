import { useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { WidgetView } from './views/WidgetView';
import { SettingsWindow } from './views/SettingsWindow';
import { useEventListeners } from './hooks/useEventListeners';
import { useEngineStore } from './store/engineStore';
import type { EngineStatus } from './types/engine';

/* 
   Resolve the label synchronously at module load time — it is a static
   property on the window object and never changes during the lifetime of the
   WebView. This sidesteps the blank-frame race condition caused by the
   previous useState('') pattern where the first render always returned null. 
*/
const WINDOW_LABEL = getCurrentWindow().label;

export default function App() {
  const { setStatus, refreshPresets, refreshDnsStatus, hasHydrated, persistenceError, language } = useEngineStore();

  useEffect(() => {
    const bootstrap = async () => {
      if (!hasHydrated || persistenceError) return;
      try {
        const currentStatus = await invoke<EngineStatus>('get_engine_status');
        setStatus(currentStatus);
      } catch (e) {
        console.error('Failed to get engine status:', e);
      }

      await refreshPresets();
      await refreshDnsStatus();
    };

    bootstrap();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasHydrated, persistenceError]);

  // Register global event listeners for this window.
  useEventListeners();

  if (!hasHydrated) return null;
  if (persistenceError) {
    return (
      <div style={{ padding: 20, color: '#fff', background: '#15171c', minHeight: '100vh' }}>
        {language === 'tr'
          ? 'Ayarlar güvenli biçimde okunamadı. Vane, kayıtlı ayarlarınızı varsayılanlarla ezmemek için başlatmayı durdurdu.'
          : 'Settings could not be read safely. Vane stopped startup to avoid overwriting your saved settings with defaults.'}
      </div>
    );
  }

  if (WINDOW_LABEL === 'settings') return <SettingsWindow />;
  return <WidgetView />;
}
