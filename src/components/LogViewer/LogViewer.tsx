import { useEffect, useRef } from 'react';
import type { LogLine } from '../../types/engine';
import { useEngineStore } from '../../store/engineStore';
import { translations } from '../../utils/translations';
import styles from './LogViewer.module.css';

interface LogViewerProps {
  logs: LogLine[];
  onClear: () => void;
}

/** Terminal benzeri log görüntüleyici. Yeni satır geldiğinde otomatik scroll yapar. */
export function LogViewer({ logs, onClear }: LogViewerProps) {
  const { language } = useEngineStore();
  const t = translations[language];
  const outputRef = useRef<HTMLDivElement>(null);

  // Yeni log gelince aşağı scroll
  useEffect(() => {
    const el = outputRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [logs.length]);

  return (
    <div className={styles.viewer}>
      <div className={styles.toolbar}>
        <span className={styles.toolbarTitle}>{t.engineOutput}</span>
        <div className={styles.toolbarActions}>
          <span className={styles.logCount}>{logs.length} {t.lines}</span>
          <button
            id="log-clear-btn"
            className={styles.toolbarBtn}
            onClick={onClear}
            disabled={logs.length === 0}
          >
            {t.clear}
          </button>
        </div>
      </div>

      <div ref={outputRef} className={styles.output} role="log" aria-live="polite">
        {logs.length === 0 ? (
          <div className={styles.empty}>{t.notStartedYet}</div>
        ) : (
          logs.map((line) => <LogLineRow key={line.id} line={line} />)
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// LogLineRow — tek satır render'ı ayrı bileşene taşındı (SRP)
// ---------------------------------------------------------------------------

const TAG_STYLES: Record<string, string> = {
  'MOTOR':      'tagEngine',
  'ENGINE':     'tagEngine',
  'DNS':        'tagDns',
  'ADBLOCK':    'tagAdblock',
  'PATTERN':    'tagSecurity',
  'DESEN':      'tagSecurity',
  'GÜVENLİK':  'tagSecurity',
  'SECURITY':   'tagSecurity',
  'GÜNCELLEME': 'tagUpdate',
  'UPDATE':     'tagUpdate',
  'SİSTEM':     'tagSystem',
  'SYSTEM':     'tagSystem',
  'HATA':       'tagError',
  'ERROR':      'tagError',
  'UYARI':      'tagWarn',
  'WARN':       'tagWarn',
  'WARNING':    'tagWarn',
  'INFO':       'tagSystem',
  'BİLGİ':      'tagSystem',
};

const TAG_TRANSLATIONS: Record<string, Record<'tr' | 'en', string>> = {
  'MOTOR':      { tr: 'MOTOR', en: 'ENGINE' },
  'ENGINE':     { tr: 'MOTOR', en: 'ENGINE' },
  'DNS':        { tr: 'DNS', en: 'DNS' },
  'ADBLOCK':    { tr: 'ADBLOCK', en: 'ADBLOCK' },
  'PATTERN':    { tr: 'DESEN', en: 'PATTERN' },
  'DESEN':      { tr: 'DESEN', en: 'PATTERN' },
  'GÜVENLİK':  { tr: 'GÜVENLİK', en: 'SECURITY' },
  'SECURITY':   { tr: 'GÜVENLİK', en: 'SECURITY' },
  'GÜNCELLEME': { tr: 'GÜNCELLEME', en: 'UPDATE' },
  'UPDATE':     { tr: 'GÜNCELLEME', en: 'UPDATE' },
  'SİSTEM':     { tr: 'SİSTEM', en: 'SYSTEM' },
  'SYSTEM':     { tr: 'SİSTEM', en: 'SYSTEM' },
  'HATA':       { tr: 'HATA', en: 'ERROR' },
  'ERROR':      { tr: 'HATA', en: 'ERROR' },
  'UYARI':      { tr: 'UYARI', en: 'WARNING' },
  'WARN':       { tr: 'UYARI', en: 'WARNING' },
  'WARNING':    { tr: 'UYARI', en: 'WARNING' },
  'INFO':       { tr: 'BİLGİ', en: 'INFO' },
  'BİLGİ':      { tr: 'BİLGİ', en: 'INFO' },
};

function getTagStyleKey(tag: string): string {
  return TAG_STYLES[tag.toUpperCase()] ?? 'tagGeneric';
}

function getTranslatedTag(tag: string, lang: 'tr' | 'en'): string {
  const upper = tag.toUpperCase();
  return TAG_TRANSLATIONS[upper]?.[lang] ?? tag;
}

function LogLineRow({ line }: { line: LogLine }) {
  const { language } = useEngineStore();
  const time = line.timestamp.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });

  // [TAG] pattern — satır başındaki köşeli parantez içindeki etiketi al
  const tagMatch = line.content.match(/^\[([^\]]+)\]\s*([\s\S]*)$/);

  if (tagMatch) {
    const tag = tagMatch[1];
    const rest = tagMatch[2];
    const tagKey = getTagStyleKey(tag);
    const translatedTag = getTranslatedTag(tag, language);

    return (
      <div className={styles.logLine}>
        <span className={styles.timestamp}>{time}</span>
        <span className={`${styles.tag} ${styles[tagKey]}`}>{translatedTag}</span>
        <span className={`${styles['content--' + line.level]} ${styles.message}`}>{rest}</span>
      </div>
    );
  }

  return (
    <div className={styles.logLine}>
      <span className={styles.timestamp}>{time}</span>
      <span className={`${styles['content--' + line.level]} ${styles.message}`}>{line.content}</span>
    </div>
  );
}
