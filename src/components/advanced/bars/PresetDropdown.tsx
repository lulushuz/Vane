import { useState, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown, Shield, Settings } from 'lucide-react';
import styles from '../../../views/AdvancedView.module.css';
import type { Preset } from '../../../types/engine';
import { PresetIcons } from '../presetIcons';

interface PresetDropdownProps {
  activePresetId: string | null;
  presets: Preset[];
  onSelect: (id: string) => void;
}

export function PresetDropdown({ activePresetId, presets, onSelect }: PresetDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const activePreset = presets.find((p) => p.id === activePresetId);
  const ActiveIcon = activePreset 
    ? (PresetIcons[activePreset.id] || (activePreset.isCustom ? Settings : Shield)) 
    : Shield;

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSelect = (id: string) => {
    onSelect(id);
    setIsOpen(false);
  };

  return (
    <div className={styles.dropdownContainer} ref={dropdownRef} style={{ width: 280, marginTop: 0 }}>
      <button
        className={styles.presetDropdownBtn}
        onClick={() => setIsOpen(!isOpen)}
        style={{ padding: '8px 12px' }}
      >
        <div className={styles.presetsBtnContent}>
          <ActiveIcon size={16} className={styles.presetIconActive} />
          <span>
            {activePreset
              ? activePreset.label.length > 25
                ? activePreset.label.slice(0, 25) + '…'
                : activePreset.label
              : 'Select...'}
          </span>
        </div>
        <ChevronDown size={14} className={`${styles.dropdownArrow} ${isOpen ? styles.dropdownArrowOpen : ''}`} />
      </button>

      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.15 }}
            className={styles.dropdownMenu}
          >
            {presets.map((p) => {
              const IconComp = PresetIcons[p.id] || (p.isCustom ? Settings : Shield);
              return (
                <div
                  key={p.id}
                  className={`${styles.dropdownItem} ${activePresetId === p.id ? styles.dropdownItemActive : ''}`}
                  onClick={() => handleSelect(p.id)}
                >
                  <IconComp size={15} className={styles.dropdownItemIcon} />
                  <div className={styles.dropdownItemTexts}>
                    <span className={styles.dropdownItemLabel}>
                      {p.label.length > 20 ? p.label.slice(0, 20) + '…' : p.label}
                    </span>
                  </div>
                </div>
              );
            })}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
