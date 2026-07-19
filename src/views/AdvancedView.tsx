import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { Trash2, RotateCcw, RefreshCw, HardDriveDownload, HardDriveUpload } from 'lucide-react';
import styles from './AdvancedView.module.css';

import {
  useEngineStore,
  type AdvancedConfig,
  DEFAULT_ADVANCED_CONFIG,
} from '../store/engineStore';
import { parseArgsToConfig, serializeConfigToArgs } from '../utils/argsParser';
import { validateImportedPreset } from '../utils/presetValidator';
import type { Preset } from '../types/engine';
import { translations } from '../utils/translations';

import { PresetDropdown } from '../components/advanced/bars/PresetDropdown';
import { UnsavedBanner } from '../components/advanced/bars/UnsavedBanner';
import { DpiDesyncCard } from '../components/advanced/cards/DpiDesyncCard';
import { PacketTrafficCard } from '../components/advanced/cards/PacketTrafficCard';
import { ProtocolPortsCard } from '../components/advanced/cards/ProtocolPortsCard';
import { SafetyCard } from '../components/advanced/cards/SafetyCard';
import { ProxyCard } from '../components/advanced/cards/ProxyCard';
import { EvasionFoolingCard } from '../components/advanced/cards/EvasionFoolingCard';
import { PayloadsCard } from '../components/advanced/cards/PayloadsCard';
import { TProxyCard } from '../components/advanced/cards/TProxyCard';

function slugify(text: string) {
  return text.toString().toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/[^\w-]+/g, '')
    .replace(/--+/g, '-')
    .replace(/^-+/, '')
    .replace(/-+$/, '');
}

export function AdvancedView() {
  const {
    status,
    activePresetId,
    presets,
    advancedConfig,
    setActivePreset,
    setAdvancedConfig,
    resetAdvancedConfig,
    setAdvancedDirty,
    upsertPreset,
    deletePreset,
    startEngine,
    stopEngine,
    appendLog,
    language,
  } = useEngineStore();

  const fileInputRef = useRef<HTMLInputElement>(null);
  const t = translations[language];

  const isRunning = status.variant === 'running';
  const [isApplying, setIsApplying] = useState(false);
  const [isDirty, setIsDirty] = useState(false);
  const [isReset, setIsReset] = useState(false);
  const [profileName, setProfileName] = useState(language === 'tr' ? 'Özel Profilim' : 'My Custom Preset');

  const [snapshot, setSnapshot] = useState<AdvancedConfig>(advancedConfig);

  const activePreset = presets.find(p => p.id === activePresetId);

  const handlePresetSelect = async (newId: string) => {
    setActivePreset(newId);
    if (isRunning) {
      await stopEngine();
      await startEngine(newId);
    }
  };

  useEffect(() => {
    setAdvancedDirty(isDirty);
  }, [isDirty, setAdvancedDirty]);

  useEffect(() => {
    return () => { setAdvancedDirty(false); };
  }, [setAdvancedDirty]);

  useEffect(() => {
    if (!activePresetId) return;
    const preset = presets.find(p => p.id === activePresetId);
    if (!preset?.args) return;

    const parsed = parseArgsToConfig(preset.args);
    if (parsed.invalidArgs.length > 0) {
      appendLog(
        language === 'tr'
          ? `[ADVANCED] ${parsed.invalidArgs.length} geçersiz veya desteklenmeyen argüman güvenlik için uygulanmadı: ${parsed.invalidArgs.join(', ')}`
          : `[ADVANCED] ${parsed.invalidArgs.length} invalid or unsupported argument(s) were not applied for safety: ${parsed.invalidArgs.join(', ')}`,
        'warn',
      );
    }
    setAdvancedConfig(parsed);
    setSnapshot(parsed);
    setIsDirty(false);
    setProfileName(preset.isCustom ? preset.label : 'My Custom Preset');
  }, [activePresetId, presets, setAdvancedConfig, appendLog, language]);

  const update = <K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]) => {
    setAdvancedConfig({ [key]: value } as Partial<AdvancedConfig>);
    if (!isDirty) {
      setIsDirty(true);
      if (!activePreset?.isCustom) setProfileName('My Custom Preset');
    }
    setIsReset(false);
  };

  const handleReset = () => {
    resetAdvancedConfig();
    setSnapshot(DEFAULT_ADVANCED_CONFIG);
    setIsDirty(true);
    setIsReset(true);
    setProfileName('Default');
  };

  const handleCancel = () => {
    setAdvancedConfig(snapshot);
    setIsDirty(false);
    setIsReset(false);
  };

  const buildPreset = (name: string): Preset => ({
    id: slugify(name),
    label: name,
    description: 'Custom configuration created from the Advanced tab.',
    icon: '⚙️',
    args: serializeConfigToArgs(advancedConfig),
    isCustom: true,
    priority: 10,
    category: 'custom',
  });

  const handleSave = async () => {
    if (!profileName.trim()) return;
    setIsApplying(true);
    try {
      const newPreset = buildPreset(profileName.trim());
      await invoke('save_custom_preset', { preset: newPreset });
      upsertPreset(newPreset);
      setActivePreset(newPreset.id);
      setSnapshot(advancedConfig);
      setIsDirty(false);
      if (isRunning) { await stopEngine(); await startEngine(newPreset.id); }
      appendLog(language === 'tr'
        ? `[ADVANCED] “${newPreset.label}” profili doğrulandı, atomik olarak kaydedildi ve seçildi.`
        : `[ADVANCED] Profile “${newPreset.label}” was validated, saved atomically, and selected.`, 'info');
    } catch (e) {
      console.error('Save error:', e);
      appendLog(language === 'tr'
        ? `[ERROR] Gelişmiş ayarlar doğrulanamadı ve motora gönderilmedi: ${e}`
        : `[ERROR] Advanced settings failed validation and were not sent to the engine: ${e}`, 'error');
    } finally {
      setIsApplying(false);
    }
  };

  const handleDelete = async () => {
    if (!activePresetId || !activePreset?.isCustom) return;

    const confirmMsg = language === 'tr'
      ? `'${activePreset.label}' profilini silmek istediğinize emin misiniz?`
      : `Are you sure you want to delete the profile '${activePreset.label}'?`;

    if (window.confirm(confirmMsg)) {
      setIsApplying(true);
      try {
        await deletePreset(activePresetId);
        setIsDirty(false);
        setIsReset(false);
      } catch (err) {
        console.error('Delete error:', err);
      } finally {
        setIsApplying(false);
      }
    }
  };

  const handleExport = async () => {
    if (!profileName.trim()) return;
    const preset = buildPreset(profileName.trim());
    const jsonStr = JSON.stringify(preset, null, 2);
    
    try {
      const filePath = await save({
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }],
        defaultPath: `${preset.id}.json`
      });
      
      if (filePath) {
        await invoke('export_preset', { filePath, content: jsonStr });
      }
    } catch (e) {
      console.error('Export error:', e);
    }
  };

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (event) => {
      try {
        const rawJson = JSON.parse(event.target?.result as string);
        const validation = validateImportedPreset(rawJson);
        if (!validation.ok) {
          const invalidMsg = language === 'tr'
            ? `Geçersiz veya güvenli olmayan profil:\n\n${validation.error}`
            : `Invalid or unsafe preset:\n\n${validation.error}`;
          alert(invalidMsg);
          return;
        }

        const preset = validation.preset;
        preset.id = slugify(preset.label || `imported-${Date.now()}`);

        const parsed = parseArgsToConfig(preset.args);
        if (parsed.invalidArgs.length > 0) {
          appendLog(
            language === 'tr'
              ? `[ADVANCED] İçe aktarılan profildeki ${parsed.invalidArgs.length} geçersiz veya desteklenmeyen argüman uygulanmadı: ${parsed.invalidArgs.join(', ')}`
              : `[ADVANCED] ${parsed.invalidArgs.length} invalid or unsupported argument(s) in the imported preset were not applied: ${parsed.invalidArgs.join(', ')}`,
            'warn',
          );
        }
        setAdvancedConfig(parsed);
        setProfileName(preset.label);

        await invoke('save_custom_preset', { preset });
        upsertPreset(preset);
        setActivePreset(preset.id);
        setSnapshot(parsed);
        setIsDirty(false);

        if (isRunning) { await stopEngine(); await startEngine(preset.id); }
      } catch (err) {
        console.error('Import error:', err);
        const failReadMsg = language === 'tr'
          ? 'Dosya okunamadı. Lütfen geçerli bir JSON profili seçin.'
          : 'Failed to read the file. Please select a valid JSON preset.';
        alert(failReadMsg);
      }
    };
    reader.readAsText(file);
  };

  const c = advancedConfig;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
      className={styles.view}
    >
      <div className={styles.headerRow}>
        <div className={styles.titleArea}>
          <h2 className={styles.title}>{t.advancedTitle}</h2>
        </div>
        <div className={styles.headerControls}>
          <div className={styles.leftActions}>
            <button 
              className={styles.actionBtn} 
              onClick={() => fileInputRef.current?.click()} 
              title={language === 'tr' ? 'Profil İçe Aktar' : 'Import Profile'}
            >
              <HardDriveUpload size={16} />
              <span>{language === 'tr' ? 'İçe Aktar' : 'Import'}</span>
            </button>
            
            <button 
              className={styles.actionBtn} 
              onClick={handleExport} 
              disabled={!profileName.trim()} 
              title={language === 'tr' ? 'Profili Dışa Aktar' : 'Export Profile'}
            >
              <HardDriveDownload size={16} />
              <span>{language === 'tr' ? 'Dışa Aktar' : 'Export'}</span>
            </button>
          </div>

          <input 
            type="file" 
            accept=".json" 
            ref={fileInputRef} 
            style={{ display: 'none' }} 
            onChange={(e) => {
              handleFileChange(e);
              if (fileInputRef.current) fileInputRef.current.value = '';
            }} 
          />

          <div className={styles.rightActions}>
            {isApplying && <RefreshCw size={18} className={styles.spin} color="#5c7cfa" />}
            <PresetDropdown activePresetId={activePresetId} presets={presets} onSelect={handlePresetSelect} />
            {activePreset?.isCustom && activePreset.id !== 'default' && !isDirty && (
              <button className={styles.actionBtn} onClick={handleDelete} title={language === 'tr' ? 'Bu profili sil' : 'Delete this profile'} style={{ color: '#ff6b6b', borderColor: 'rgba(255, 107, 107, 0.2)' }}>
                <Trash2 size={16} />
              </button>
            )}
            <button className={styles.actionBtn} onClick={handleReset} title={t.resetDefault}>
              <RotateCcw size={16} />
            </button>
          </div>
        </div>
      </div>

      <div className={styles.groups}>
        <DpiDesyncCard config={c} update={update} />
        <PacketTrafficCard config={c} update={update} />
        <ProtocolPortsCard config={c} update={update} />
        <EvasionFoolingCard config={c} update={update} />
        <PayloadsCard config={c} update={update} />
        <TProxyCard config={c} update={update} />
        <SafetyCard />
        <ProxyCard />
      </div>

      <UnsavedBanner 
        isDirty={isDirty} 
        isReset={isReset} 
        isApplying={isApplying} 
        profileName={profileName} 
        setProfileName={setProfileName} 
        onCancel={handleCancel} 
        onSave={handleSave} 
      />
    </motion.div>
  );
}
