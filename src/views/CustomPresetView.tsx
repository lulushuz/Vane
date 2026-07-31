import { useState } from 'react';
import { usePresets } from '../hooks/usePresets';
import type { Preset } from '../types/engine';
import { Toast } from '../components/Toast/Toast';
import styles from './CustomPresetView.module.css';

type EditorMode = 'form' | 'json';

export function CustomPresetView() {
  const { customPresets, saveCustomPreset, deleteCustomPreset } = usePresets();

  const [mode, setMode] = useState<EditorMode>('form');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Form state
  const [id, setId] = useState('');
  const [label, setLabel] = useState('');
  const [description, setDescription] = useState('');
  const [argsText, setArgsText] = useState('');

  // JSON state
  const [jsonText, setJsonText] = useState('{\n  "id": "my-custom-preset",\n  "label": "My Preset",\n  "description": "",\n  "icon": "⚙️",\n  "args": ["--wf-tcp=443"]\n}');

  const handleSaveForm = async () => {
    try {
      if (!id || !label) {
        throw new Error('ID and Name fields are required.');
      }

      const args = argsText.split('\n').map(a => a.trim()).filter(a => a.length > 0);

      const preset: Preset = {
        id,
        label,
        description,
        icon: '⚙️',
        args,
        isCustom: true,
      };

      await saveCustomPreset(preset);
      setSuccess(`${label} saved successfully!`);
      setId('');
      setLabel('');
      setDescription('');
      setArgsText('');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSaveJson = async () => {
    try {
      const parsed = JSON.parse(jsonText);
      if (!parsed.id || !parsed.label || !Array.isArray(parsed.args)) {
        throw new Error('Invalid format. id, label (string) and args (array) are required.');
      }

      const preset: Preset = {
        id: parsed.id,
        label: parsed.label,
        description: parsed.description || '',
        icon: parsed.icon || '⚙️',
        args: parsed.args,
        isCustom: true,
      };

      await saveCustomPreset(preset);
      setSuccess(`${preset.label} saved successfully!`);
    } catch (err: unknown) {
      setError(`JSON Error: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const handleDelete = async (presetId: string) => {
    try {
      await deleteCustomPreset(presetId);
      setSuccess('Preset deleted.');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleImportJson = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (event) => {
      try {
        const text = event.target?.result as string;
        let parsed = JSON.parse(text);

        if (!Array.isArray(parsed)) {
          parsed = [parsed];
        }

        let importedCount = 0;
        for (const item of parsed) {
          if (item.id && item.label && Array.isArray(item.args)) {
            const preset: Preset = { ...item, isCustom: true };
            await saveCustomPreset(preset);
            importedCount++;
          }
        }

        setSuccess(`${importedCount} preset(s) imported successfully!`);
        e.target.value = '';
      } catch (err) {
        setError(`Import error: ${err instanceof Error ? err.message : String(err)}`);
        e.target.value = '';
      }
    };
    reader.readAsText(file);
  };

  return (
    <div className={styles.view}>
      <div className={styles.header}>
        <h2 className={styles.title}>Add Custom Preset</h2>
        <div className={styles.tabs}>
          <button
            className={`${styles.tabBtn} ${mode === 'form' ? styles.active : ''}`}
            onClick={() => setMode('form')}
          >
            Form
          </button>
          <button
            className={`${styles.tabBtn} ${mode === 'json' ? styles.active : ''}`}
            onClick={() => setMode('json')}
          >
            JSON
          </button>
        </div>
      </div>

      <div className={styles.editorArea}>
        {mode === 'form' ? (
          <div className={styles.formMode}>
            <div className={styles.inputGroup}>
              <label>ID (must be unique, no spaces):</label>
              <input value={id} onChange={e => setId(e.target.value)} placeholder="my-preset" />
            </div>
            <div className={styles.inputGroup}>
              <label>Name:</label>
              <input value={label} onChange={e => setLabel(e.target.value)} placeholder="My Preset" />
            </div>
            <div className={styles.inputGroup}>
              <label>Description:</label>
              <input value={description} onChange={e => setDescription(e.target.value)} placeholder="My bypass configuration..." />
            </div>
            <div className={styles.inputGroup}>
              <label>Arguments (one per line):</label>
              <textarea
                value={argsText}
                onChange={e => setArgsText(e.target.value)}
                placeholder={"--wf-tcp=443\n--dpi-desync=fake"}
                rows={5}
              />
            </div>
            <button className={styles.saveBtn} onClick={handleSaveForm}>Save</button>
          </div>
        ) : (
          <div className={styles.jsonMode}>
            <textarea
              className={styles.jsonEditor}
              value={jsonText}
              onChange={e => setJsonText(e.target.value)}
              spellCheck={false}
            />
            <div style={{ display: 'flex', gap: '10px' }}>
              <button className={styles.saveBtn} onClick={handleSaveJson}>Save JSON</button>

              <label className={styles.saveBtn} style={{ background: '#4b5563', cursor: 'pointer', textAlign: 'center' }}>
                Import from File
                <input
                  type="file"
                  accept=".vane,.json"
                  onChange={handleImportJson}
                  style={{ display: 'none' }}
                />

              </label>
            </div>
          </div>
        )}
      </div>

      <div className={styles.listSection}>
        <h3 className={styles.listTitle}>Saved Custom Presets</h3>
        {customPresets.length === 0 ? (
          <p className={styles.emptyMsg}>No custom presets yet.</p>
        ) : (
          <ul className={styles.customList}>
            {customPresets.map(preset => (
              <li key={preset.id} className={styles.customItem}>
                <div className={styles.itemInfo}>
                  <span className={styles.itemName}>{preset.label}</span>
                  <span className={styles.itemId}>({preset.id})</span>
                </div>
                <button
                  className={styles.deleteBtn}
                  onClick={() => handleDelete(preset.id)}
                >
                  Delete
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className={styles.toastContainer}>
        <Toast message={error} type="error" onDismiss={() => setError(null)} />
        <Toast message={success} type="success" onDismiss={() => setSuccess(null)} />
      </div>
    </div>
  );
}
